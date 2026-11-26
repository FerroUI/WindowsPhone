//! The launcher drawer combines shared applications and the observed OS catalog.
//! A recycled ListBox keeps a large installed-app catalog out of the visual tree.
use crate::{common, session::Session};
use ferroui_base::{
    layout::{HorizontalAlignment as H, VerticalAlignment as V},
    media::FontWeight,
};
use ferroui_view::prelude::*;
use phone_controls::{flat, icon, GroupedListState, Icon, ListGroup};
use phone_host::{platform::*, HostRequest};
use phone_state::PhoneViewModel;
use std::{cell::RefCell, collections::BTreeMap, rc::Rc};
const DEVICE_APP: &str = "org.ferroui.device";
const HEADER_HEIGHT: f64 = 56.0;
struct State {
    selected: Signal<Option<DeviceApplication>>,
    list: NodeRef<ListBox>,
    groups: Rc<GroupedListState>,
    query: RefCell<String>,
}
impl Default for State {
    fn default() -> Self {
        Self {
            selected: signal(None),
            list: NodeRef::new(),
            groups: GroupedListState::new(HEADER_HEIGHT),
            query: RefCell::new(String::new()),
        }
    }
}
#[derive(Clone, PartialEq)]
enum Row {
    Header(char),
    Shared { id: String, title: String },
    Device(DeviceApplication),
}
impl Row {
    fn title(&self) -> &str {
        match self {
            Self::Shared { title, .. } => title,
            Self::Device(app) => &app.title,
            Self::Header(_) => "",
        }
    }
    fn key(&self) -> &str {
        match self {
            Self::Shared { id, .. } => id,
            Self::Device(app) => &app.id,
            Self::Header(_) => "",
        }
    }
}
fn rows(
    shared: Vec<(String, String)>,
    devices: Vec<DeviceApplication>,
    query: &str,
) -> (Vec<Row>, BTreeMap<char, usize>) {
    let query = query.trim().to_lowercase();
    let mut groups: BTreeMap<char, Vec<Row>> = BTreeMap::new();
    for item in shared
        .into_iter()
        .map(|(id, title)| Row::Shared { id, title })
        .chain(devices.into_iter().map(Row::Device))
    {
        if !item.title().to_lowercase().contains(&query)
            && !item.key().to_lowercase().contains(&query)
        {
            continue;
        }
        let letter = item
            .title()
            .chars()
            .next()
            .filter(char::is_ascii_alphabetic)
            .map(|v| v.to_ascii_lowercase())
            .unwrap_or('#');
        groups.entry(letter).or_default().push(item);
    }
    let mut result = Vec::new();
    let mut indices = BTreeMap::new();
    for (letter, mut items) in groups {
        indices.insert(letter, result.len());
        if query.is_empty() {
            result.push(Row::Header(letter));
        }
        items.sort_by_cached_key(|item| (item.title().to_lowercase(), item.key().to_owned()));
        result.extend(items);
    }
    (result, indices)
}
pub(crate) fn dismiss(ui: &Session) -> bool {
    let state = ui.state::<State>();
    if state.selected.get_untracked().is_some() {
        state.selected.set(None);
        true
    } else {
        false
    }
}
pub(crate) fn has_overlay(ui: &Session) -> bool {
    ui.state::<State>().selected.get_untracked().is_some()
}
pub(crate) fn reset(ui: &Session) {
    let state = ui.state::<State>();
    state.selected.set(None);
    state.query.borrow_mut().clear();
    state.groups.set_groups(Vec::new());
    state.groups.reset();
}
pub(super) fn view(vm: Rc<PhoneViewModel>, ui: Rc<Session>) -> Element {
    let state = ui.state::<State>();
    let query = ui.query.get();
    let installed = vm.data().installed.clone();
    let shared = ui
        .app_manifests()
        .into_iter()
        .filter(|app| {
            app.listed
                && app.builtin.map_or_else(
                    || vm.extension_enabled(app.id),
                    |id| installed.contains(&id),
                )
        })
        .map(|app| (app.id.into(), app.title.into()))
        .collect();
    let (rows, groups) = rows(shared, vm.device_applications(), &query);
    let normalized_query = query.trim().to_lowercase();
    state.groups.set_groups(if normalized_query.is_empty() {
        groups
            .iter()
            .map(|(key, index)| ListGroup::new(key.to_string(), *index))
            .collect()
    } else {
        Vec::new()
    });
    if *state.query.borrow() != normalized_query {
        *state.query.borrow_mut() = normalized_query;
        state.groups.reset();
        ui.alphabet.set(false);
    }
    let count = rows
        .iter()
        .filter(|row| !matches!(row, Row::Header(_)))
        .count();
    let model = vm.clone();
    let session = ui.clone();
    let list = phone_sdk::device::grouped_list(
        "NativeAllAppsList",
        rows,
        Rc::new(move |row| match row {
            Row::Header(letter) => {
                let alphabet = session.alphabet;
                group_header(
                    &model,
                    alphabet,
                    &format!("AppGroup-{letter}"),
                    &letter.to_string(),
                )
            }
            Row::Shared { id, title } => {
                let Some(app) = session.registry().by_id(&id).cloned() else {
                    return Element::Empty;
                };
                let open = model.clone();
                let open_ui = session.clone();
                let pin = model.clone();
                let pin_ui = session.clone();
                let square = super::app_square(&app.manifest, &common::accent(&model));
                let manifest = app.manifest;
                let label = title.clone();
                rsx! {Button{name:format!("App-{}",manifest.builtin.map(|v|v.icon()).unwrap_or(manifest.id)),class:"phone-flat",height:59.0,padding:(14.0,0.0),horizontal_alignment:H::Stretch,horizontal_content_alignment:H::Stretch,automation_properties_name:title.clone(),
                on_click:move|_|if let Some(id)=manifest.builtin{open_ui.open_app(&open,id)}else{open.resume_extension(manifest.id)},
                on_context_requested:move|event|{let model=pin.clone();let title=label.clone();pin_ui.confirm(&format!("pin {title} to start?"),Rc::new(move||{if let Some(id)=manifest.builtin{model.pin(id);model.home();}else{match model.pin_app(manifest.id,"",&title){Ok(())=>model.home(),Err(error)=>model.set_status(error)}}}));event.set_handled(true);},
                Grid{column_definitions:"45,*",column_spacing:14.0,{square}TextBlock{Grid.column:1,text:title,font_size:26.0,font_weight:FontWeight::Light,vertical_alignment:V::Center}}}}
            }
            Row::Device(app) => {
                let open = model.clone();
                let menu = session.state::<State>().selected;
                let selected = app.clone();
                let id = app.id.clone();
                let label = format!("Open device app {}", app.title);
                rsx! {Button{name:format!("App-native-{}",phone_sdk::device::encode_route_value(&app.id)),class:"phone-flat",height:59.0,padding:(14.0,0.0),horizontal_alignment:H::Stretch,horizontal_content_alignment:H::Stretch,automation_properties_name:label,
                on_click:move|_|open.request(HostRequest::Platform{request:PlatformRequest::LaunchApplication{id:id.clone()}}),
                on_context_requested:move|event|{menu.set(Some(selected.clone()));event.set_handled(true);},
                Grid{column_definitions:"45,*",column_spacing:14.0,Border{width:45.0,height:45.0,background:common::accent(&model).as_str(),{icon(Icon::List,25.0,"White")}}TextBlock{Grid.column:1,text:app.title,font_size:26.0,font_weight:FontWeight::Light,vertical_alignment:V::Center}}}}
            }
        }),
        state.list,
        state.groups.clone(),
    );
    let refresh = vm.clone();
    let load = vm.clone();
    let home = vm.clone();
    let clear = ui.query;
    let sticky = state.groups.current_group();
    let alphabet = ui.alphabet;
    let mut content = rsx! {Grid{name:"NativeAllAppsRoot",row_definitions:"Auto,Auto,*",on_loaded:move|_|load.request(HostRequest::Platform{request:PlatformRequest::ListApplications}),
        Grid{column_definitions:"48,*,40",margin:(10.0,12.0,14.0,8.0),
            Button{class:"phone-flat",automation_properties_name:"Back to Start",on_click:move|_|home.home(),{icon(Icon::Back,23.0,&common::fg(&vm))}}
            TextBox{Grid.column:1,name:"AppSearch",class:"phone-input",text:ui.query.two_way(),placeholder_text:"search",automation_properties_name:"Search apps",font_size:21.0,min_height:43.0}
            Button{Grid.column:2,class:"phone-flat",automation_properties_name:"Clear app search",on_click:move|_|clear.set(String::new()),{icon(Icon::Close,17.0,&common::fg(&vm))}}}
        Grid{Grid.row:1,column_definitions:"*,Auto",margin:(20.0,0.0,16.0,4.0),{common::muted(&format!("{count} apps"),15.0)}Button{Grid.column:1,class:"phone-flat",content:"refresh",font_size:16.0,on_click:move|_|refresh.request(HostRequest::Platform{request:PlatformRequest::ListApplications})}}
        Border{Grid.row:2,{list}}
        if let Some(letter)=sticky {Grid{Grid.row:2,clip_to_bounds:true,
            Border{height:HEADER_HEIGHT,vertical_alignment:V::Top,background:common::bg(&vm).as_str(),
                render_transform:Some(state.groups.translation().into()),
                {group_header(&vm,alphabet,"AppStickyGroup",&letter)}
            }
        }}
        if count==0{Border{Grid.row:2,margin:20.0,{common::empty("no results","Try a different app name.")}}}
    }};
    if ui.alphabet.get() {
        let mut cells = uniform_grid().columns(4).margin((20.0, 24.0));
        let cell = ((crate::shell::phone_size(&vm).0 - 40.0) / 4.0 - 10.0).max(44.0);
        for letter in "#abcdefghijklmnopqrstuvwxyz".chars() {
            let index = groups.get(&letter).copied();
            let alphabet = ui.alphabet;
            let grouped = state.groups.clone();
            let palette = phone_controls::palette();
            cells=cells.child(rsx!{Button{name:format!("AppJump-{letter}"),class:"phone-flat",height:cell,margin:5.0,is_enabled:index.is_some(),background:if index.is_some(){palette.accent.as_str()}else{palette.subtle},horizontal_alignment:H::Stretch,vertical_alignment:V::Stretch,horizontal_content_alignment:H::Center,vertical_content_alignment:V::Center,automation_properties_name:format!("Jump to {letter}"),on_click:move|_|{alphabet.set(false);grouped.jump(&letter.to_string());},TextBlock{text:letter.to_string(),font_size:37.0,foreground:if index.is_some(){"White"}else{palette.secondary}}}});
        }
        content = rsx! {Grid{{content}Border{background:common::bg(&vm).as_str(),{common::scroll(cells.into())}}}};
    }
    if let Some(app) = state.selected.get() {
        content = rsx! {Grid{{content}{actions(vm,ui,app)}}};
    }
    content
}

fn group_header(vm: &PhoneViewModel, alphabet: Signal<bool>, name: &str, letter: &str) -> Element {
    let accent = common::accent(vm);
    rsx! {Button{name:name,class:"phone-flat",height:HEADER_HEIGHT,width:42.0,horizontal_alignment:H::Left,margin:(14.0,0.0),
        automation_properties_name:format!("Jump to letter {letter}"),on_click:move|_|alphabet.set(true),
        Border{width:32.0,height:32.0,border_brush:accent.as_str(),border_thickness:2.0,
            TextBlock{text:letter,font_size:23.0,foreground:accent.as_str(),horizontal_alignment:H::Center,vertical_alignment:V::Center}
        }
    }}
}

fn actions(vm: Rc<PhoneViewModel>, ui: Rc<Session>, app: DeviceApplication) -> Element {
    let close = ui.state::<State>().selected;
    let dismiss = close;
    let snapshot = vm.device_state().snapshot;
    let mut commands = stack_panel()
        .spacing(14.0)
        .child(common::text(&app.title, 30.0));
    let open = vm.clone();
    let id = app.id.clone();
    commands = commands.child(flat("open", move || {
        close.set(None);
        open.request(HostRequest::Platform {
            request: PlatformRequest::LaunchApplication { id: id.clone() },
        });
    }));
    if vm.extension_enabled(DEVICE_APP) {
        let pin = vm.clone();
        let id = app.id.clone();
        let title = app.title.clone();
        commands = commands.child(flat("pin to Start", move || {
            let path = format!("launch/{}", phone_sdk::device::encode_route_value(&id));
            match pin.pin_app(DEVICE_APP, &path, &title) {
                Ok(()) => {
                    close.set(None);
                    pin.home();
                }
                Err(error) => pin.set_status(error),
            }
        }));
    }
    if snapshot.supports(DeviceOperation::AppShortcuts) && vm.extension_enabled(DEVICE_APP) {
        let shortcuts = vm.clone();
        let session = ui.clone();
        let application = app.id.clone();
        commands = commands.child(flat("app shortcuts", move || {
            close.set(None);
            let payload = serde_json::json!({"kind":"shortcuts","application":application});
            if let Err(error) = session.queue_activation(DEVICE_APP, payload) {
                shortcuts.set_status(error);
                return;
            }
            if !crate::launch_app(&shortcuts, DEVICE_APP, "") {
                session.clear_activation();
                shortcuts.set_status("Device apps is unavailable.".into());
            }
        }));
    }
    for (label, request, available) in [
        (
            "app information",
            DeviceRequest::OpenApplicationSettings {
                application: app.id.clone(),
            },
            snapshot.supports(DeviceOperation::ApplicationDetails),
        ),
        (
            "uninstall",
            DeviceRequest::RequestUninstallApplication {
                application: app.id.clone(),
            },
            snapshot.supports(DeviceOperation::UninstallApplications),
        ),
    ] {
        if available {
            let model = vm.clone();
            commands = commands.child(flat(label, move || {
                close.set(None);
                model.request_device(request.clone());
            }));
        }
    }
    commands = commands.child(flat("close", move || dismiss.set(None)));
    rsx! {Border{name:"NativeAppActions",background:common::bg(&vm).as_str(),border_brush:common::accent(&vm).as_str(),border_thickness:2.0,padding:20.0,margin:18.0,vertical_alignment:V::Center,{common::scroll(commands.into())}}}
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn catalog_merges_names_without_confusing_native_and_shared_identities() {
        let (mut rows, groups) = rows(
            vec![("phone.phone".into(), "Phone".into())],
            vec![
                DeviceApplication {
                    id: "native.phone/.Main".into(),
                    title: "Phone".into(),
                },
                DeviceApplication {
                    id: "native.maps/.Main".into(),
                    title: "Maps".into(),
                },
            ],
            "",
        );
        assert_eq!(groups.keys().copied().collect::<Vec<_>>(), vec!['m', 'p']);
        rows.retain(|v| !matches!(v, Row::Header(_)));
        assert_eq!(rows.len(), 3);
        let (filtered, _) = super::rows(
            vec![],
            vec![DeviceApplication {
                id: "native.maps/.Main".into(),
                title: "Maps".into(),
            }],
            "NATIVE.MAPS",
        );
        assert_eq!(filtered.len(), 1);
    }
}
