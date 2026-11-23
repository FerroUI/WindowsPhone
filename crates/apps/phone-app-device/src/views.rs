use crate::{path, request, shortcut_path, shortcuts_available};
use ferroui_base::layout::{HorizontalAlignment as H, Orientation, VerticalAlignment as V};
use ferroui_view::prelude::*;
use phone_controls::{action, app_bar, flat, input, Icon};
use phone_host::platform::*;
use phone_sdk::{common::*, session::Session, AppContext};
use phone_state::PhoneViewModel;
use std::{cell::Cell, collections::BTreeSet, rc::Rc};

struct State {
    query: Signal<String>,
    tab: Signal<bool>,
    pinned_only: Signal<bool>,
    selected: Signal<Option<DeviceApplication>>,
    shortcuts_for: Signal<Option<DeviceApplication>>,
    requested_revision: Cell<u64>,
    started: Cell<Option<(i64, u64)>>,
    error: Signal<String>,
}
impl Default for State {
    fn default() -> Self {
        Self {
            query: signal(String::new()),
            tab: signal(false),
            pinned_only: signal(false),
            selected: signal(None),
            shortcuts_for: signal(None),
            requested_revision: Cell::new(0),
            started: Cell::new(None),
            error: signal(String::new()),
        }
    }
}
#[derive(Clone, PartialEq)]
enum Row {
    Header(String),
    App(DeviceApplication, bool),
}
fn pinned_paths(vm: &PhoneViewModel) -> BTreeSet<String> {
    let data = vm.data();
    let ids = data
        .all_tiles()
        .filter_map(|tile| match tile.target {
            Some(phone_domain::TileTarget::AppLink(id)) => Some(id),
            _ => None,
        })
        .collect::<BTreeSet<_>>();
    data.app_links
        .iter()
        .filter(|link| link.app == crate::APP_ID && ids.contains(&link.id))
        .map(|link| link.path.clone())
        .collect()
}
fn rows(
    mut apps: Vec<DeviceApplication>,
    query: &str,
    pinned: &BTreeSet<String>,
    pinned_only: bool,
) -> Vec<Row> {
    apps.retain(|app| {
        (app.title.to_lowercase().contains(query) || app.id.to_lowercase().contains(query))
            && (!pinned_only || pinned.contains(&path(&app.id)))
    });
    apps.sort_by_cached_key(|app| (app.title.to_lowercase(), app.id.clone()));
    let mut rows = Vec::new();
    let mut previous = String::new();
    for app in apps {
        let letter = app
            .title
            .chars()
            .next()
            .filter(|value| value.is_alphabetic())
            .map(|value| value.to_uppercase().collect::<String>())
            .unwrap_or_else(|| "#".into());
        if previous != letter {
            rows.push(Row::Header(letter.clone()));
            previous = letter;
        }
        let pinned = pinned.contains(&path(&app.id));
        rows.push(Row::App(app, pinned));
    }
    rows
}
fn catalog(cx: AppContext) -> Element {
    let state = cx.session.state::<State>();
    let query = state.query.get().trim().to_lowercase();
    let pinned = state.pinned_only.get();
    let rows = rows(
        cx.vm.device_applications(),
        &query,
        &pinned_paths(&cx.vm),
        pinned,
    );
    let count = rows
        .iter()
        .filter(|row| matches!(row, Row::App(..)))
        .count();
    let context = cx.clone();
    let all = state.clone();
    let only_pinned = state.clone();
    let list = phone_sdk::device::list(
        "DeviceApplicationList",
        rows,
        Rc::new(move |row| match row {
            Row::Header(letter) => {
                rsx! {Border{margin:(20.0,12.0,20.0,2.0),TextBlock{text:letter,font_size:30.0,foreground:accent(&context.vm).as_str()}}}
            }
            Row::App(app, pinned) => {
                let open = context.clone();
                let menu = context.session.state::<State>();
                let hold = menu.clone();
                let id = app.id.clone();
                let selected = app.clone();
                let held = app.clone();
                rsx! {Button{class:"phone-flat",min_height:64.0,horizontal_alignment:H::Stretch,horizontal_content_alignment:H::Stretch,padding:(20.0,10.0),automation_properties_name:format!("{}{}",app.title,if pinned{", pinned to Start"}else{""}),
                    on_click:move|_|request(&open,PlatformRequest::LaunchApplication{id:id.clone()}),
                    on_context_requested:move|event|{menu.selected.set(Some(selected.clone()));event.set_handled(true);},
                    on_holding:move|event|if event.holding_state()==ferroui_base::input::HoldingState::Started{hold.selected.set(Some(held.clone()));event.set_handled(true);},
                    Grid{column_definitions:"*,28",TextBlock{text:app.title,font_size:26.0,text_wrapping:ferroui_base::media::TextWrapping::Wrap},if pinned{Border{Grid.column:1,{phone_controls::icon(Icon::Pin,16.0,&fg(&context.vm))}}}}
                }}
            }
        }),
    );
    rsx! {Grid{row_definitions:"Auto,Auto,Auto,*",row_spacing:6.0,
        Border{margin:(20.0,4.0),{input("search device apps").name("DeviceAppSearch").text(state.query.two_way())}},
        StackPanel{Grid.row:1,orientation:Orientation::Horizontal,margin:(20.0,0.0),spacing:16.0,
            {flat(if pinned{"all"}else{"✓ all"},move||all.pinned_only.set(false))},
            {flat(if pinned{"✓ pinned"}else{"pinned"},move||only_pinned.pinned_only.set(true))}},
        Border{Grid.row:2,margin:(20.0,0.0),{muted(&format!("{count} apps · tap to open, hold for pin and shortcuts"),15.0)}},
        if count==0{Border{Grid.row:3,margin:20.0,{muted(if query.is_empty(){"No apps in this view. Refresh to check the device."}else{"No apps match your search."},18.0)}}},
        Border{Grid.row:3,{list}}
    }}
}
pub(super) fn choose_shortcuts(cx: &AppContext, application: DeviceApplication) {
    if !shortcuts_available(&cx.vm) {
        cx.vm
            .set_status("App shortcuts require the active Home role and shortcut access.".into());
        return;
    }
    let state = cx.session.state::<State>();
    state.selected.set(None);
    state.error.set(String::new());
    state
        .requested_revision
        .set(cx.vm.device_state().shortcut_revision);
    state
        .started
        .set(Some((cx.vm.now(), cx.vm.document_epoch())));
    state.shortcuts_for.set(Some(application.clone()));
    request(
        cx,
        PlatformRequest::Device {
            request: DeviceRequest::Shortcuts {
                application: application.id,
            },
        },
    );
}
fn shortcuts(cx: AppContext, application: DeviceApplication) -> Element {
    let state = cx.session.state::<State>();
    let observed = cx.vm.device_state();
    let pinned = pinned_paths(&cx.vm);
    let loaded = observed.shortcut_revision != state.requested_revision.get()
        && observed.shortcut_application.as_deref() == Some(&application.id);
    let supported = shortcuts_available(&cx.vm);
    let error = state.error.get();
    let refresh = cx.clone();
    let selected = application.clone();
    let back = state.clone();
    let mut body = stack_panel()
        .spacing(14.0)
        .margin(20.0)
        .child(flat("back to apps", move || {
            back.shortcuts_for.set(None);
            back.started.set(None);
        }))
        .child(text(&application.title, 30.0));
    if !supported {
        body=body.child(muted("Choose Windows Phone as the Home app and allow shortcut access to use device shortcuts.",18.0));
    } else if !error.is_empty() {
        body = body.child(muted(&error, 18.0));
    } else if !loaded {
        body = body.child(muted("Checking this app's shortcuts…", 18.0));
    } else if observed.shortcuts.is_empty() {
        body = body.child(muted(
            "This app does not currently publish any shortcuts.",
            18.0,
        ));
    } else {
        for shortcut in observed.shortcuts {
            let open = cx.clone();
            let pin = cx.clone();
            let app = application.id.clone();
            let pin_app = app.clone();
            let id = shortcut.id.clone();
            let pin_id = id.clone();
            let label = shortcut.title.clone();
            let title = format!("{} · {}", application.title, shortcut.title)
                .chars()
                .take(80)
                .collect::<String>();
            let is_pinned = pinned.contains(&shortcut_path(&application.id, &shortcut.id));
            body=body.child(rsx!{StackPanel{spacing:5.0,
                Button{class:"phone-flat",min_height:50.0,horizontal_content_alignment:H::Left,is_enabled:shortcut.enabled,automation_properties_name:format!("Open {} shortcut",shortcut.title),on_click:move|_|request(&open,PlatformRequest::Device{request:DeviceRequest::LaunchShortcut{application:app.clone(),id:id.clone()}}),TextBlock{text:label,font_size:24.0,text_wrapping:ferroui_base::media::TextWrapping::Wrap}},
                if !shortcut.enabled{{muted("Disabled by this app",16.0)}}
                if shortcut.enabled&&!is_pinned{{flat("pin shortcut to Start",move||{match pin.pin(&shortcut_path(&pin_app,&pin_id),&title){Ok(())=>pin.vm.set_status("Pinned shortcut to Start.".into()),Err(error)=>pin.vm.set_status(error)}})}}
                if is_pinned{{muted("Pinned to Start",16.0)}}
            }});
        }
    }
    body = body.child(flat("refresh shortcuts", move || {
        choose_shortcuts(&refresh, selected.clone())
    }));
    scroll(body.into())
}
fn actions(cx: &AppContext, application: DeviceApplication) -> Element {
    let state = cx.session.state::<State>();
    let dismiss = state.clone();
    let done = state.clone();
    let open = cx.clone();
    let open_id = application.id.clone();
    let pin = cx.clone();
    let pin_id = application.id.clone();
    let title = application.title.clone();
    let mut commands = stack_panel()
        .spacing(6.0)
        .margin(18.0)
        .child(text(&application.title, 30.0))
        .child(flat("open", move || {
            request(
                &open,
                PlatformRequest::LaunchApplication {
                    id: open_id.clone(),
                },
            );
            done.selected.set(None);
        }));
    if !pinned_paths(&cx.vm).contains(&path(&application.id)) {
        commands = commands.child(flat("pin to Start", move || {
            match pin.pin(&path(&pin_id), &title) {
                Ok(()) => {
                    pin.session.state::<State>().selected.set(None);
                    pin.vm.set_status("Pinned device app to Start.".into());
                }
                Err(error) => pin.vm.set_status(error),
            }
        }));
    } else {
        commands = commands.child(muted("Pinned to Start", 17.0));
    }
    if shortcuts_available(&cx.vm) {
        let cx = cx.clone();
        commands = commands.child(flat("app shortcuts", move || {
            choose_shortcuts(&cx, application.clone())
        }));
    }
    let close = state.clone();
    commands = commands.child(flat("close", move || close.selected.set(None)));
    rsx! {Grid{background:"#66000000",Button{class:"phone-flat",horizontal_alignment:H::Stretch,vertical_alignment:V::Stretch,automation_properties_name:"Close device app actions",on_click:move|_|dismiss.selected.set(None)},Border{vertical_alignment:V::Bottom,background:bg(&cx.vm).as_str(),{commands}}}}
}
fn role_label(role: HostRole) -> &'static str {
    match role {
        HostRole::Unsupported => "unavailable",
        HostRole::Available => "available",
        HostRole::Requested => "awaiting system choice",
        HostRole::Active => "active",
    }
}
fn integrations(cx: &AppContext) -> Element {
    let state = cx.vm.platform_state();
    let mut body = stack_panel()
        .margin(20.0)
        .spacing(16.0)
        .child(text(
            &format!("{} · {}", state.platform.label(), state.mode.label()),
            28.0,
        ))
        .child(muted(&state.detail, 18.0))
        .child(muted(
            &format!(
                "Home role: {}\nShell session: {}",
                role_label(state.launcher),
                role_label(state.shell)
            ),
            17.0,
        ));
    for (feature, label, value) in [
        (
            HostFeature::LauncherRole,
            "choose as home app",
            PlatformRequest::RequestLauncher,
        ),
        (
            HostFeature::SystemSettings,
            "application settings",
            PlatformRequest::OpenSettings {
                page: SystemSettingsPage::Application,
            },
        ),
        (
            HostFeature::SystemSettings,
            "network settings",
            PlatformRequest::OpenSettings {
                page: SystemSettingsPage::Network,
            },
        ),
        (
            HostFeature::Notifications,
            "allow notifications",
            PlatformRequest::RequestNotifications,
        ),
        (
            HostFeature::Haptics,
            "try haptic feedback",
            PlatformRequest::Haptic {
                kind: HapticKind::Selection,
            },
        ),
    ] {
        if state.supports(feature) {
            let cx = cx.clone();
            body = body.child(flat(label, move || request(&cx, value.clone())));
        }
    }
    scroll(body.into())
}
pub fn view(cx: AppContext) -> Element {
    let state = cx.session.state::<State>();
    let supported = cx.vm.platform_state().supports(HostFeature::InstalledApps);
    let tools = state.tab.get() || !supported;
    let catalog_state = state.clone();
    let tools_state = state.clone();
    let content_context = cx.clone();
    let shortcuts_app = state.shortcuts_for.get();
    let content = if tools {
        integrations(&cx)
    } else if let Some(app) = shortcuts_app {
        rerender(move || shortcuts(content_context.clone(), app.clone()))
    } else {
        rerender(move || catalog(content_context.clone()))
    };
    let body = rsx! {Grid{row_definitions:"Auto,*",StackPanel{orientation:Orientation::Horizontal,spacing:18.0,margin:(20.0,6.0,20.0,14.0),
    if supported{{flat(if tools{"apps"}else{"✓ apps"},move||catalog_state.tab.set(false))}},
    {flat(if tools{"✓ device"}else{"device"},move||tools_state.tab.set(true))}},Border{Grid.row:1,{content}}}};
    let refresh = cx.clone();
    let page = page(
        "device apps",
        body,
        app_bar(
            vec![action(Icon::Refresh, "refresh device apps", move || {
                request(&refresh, PlatformRequest::Refresh);
                refresh.vm.refresh_device();
                if supported {
                    request(&refresh, PlatformRequest::ListApplications);
                }
            })],
            false,
            vec![],
        ),
    );
    let selected = state.selected.get();
    rsx! {Grid{{page}if let Some(application)=selected{{actions(&cx,application)}}}}
}
pub(crate) fn back(ui: &Session) -> bool {
    let state = ui.state::<State>();
    if state.selected.get_untracked().is_some() {
        state.selected.set(None);
        true
    } else if state.shortcuts_for.get_untracked().is_some() {
        state.shortcuts_for.set(None);
        state.started.set(None);
        true
    } else {
        false
    }
}
pub(crate) fn dismiss(ui: &Session) {
    let state = ui.state::<State>();
    state.selected.set(None);
    state.shortcuts_for.set(None);
    state.started.set(None);
    state.error.set(String::new());
}
pub(crate) fn reset(ui: &Session) {
    dismiss(ui);
    let state = ui.state::<State>();
    state.query.set(String::new());
    state.tab.set(false);
    state.pinned_only.set(false);
}
pub(crate) fn tick(vm: &PhoneViewModel, ui: &Session, now: i64) {
    let state = ui.state::<State>();
    let Some((started, epoch)) = state.started.get() else {
        return;
    };
    if epoch != vm.document_epoch() {
        reset(ui);
        return;
    }
    let observed = vm.device_state();
    if state
        .shortcuts_for
        .get_untracked()
        .as_ref()
        .is_some_and(|app| {
            observed.shortcut_revision != state.requested_revision.get()
                && observed.shortcut_application.as_deref() == Some(&app.id)
        })
    {
        state.started.set(None);
    } else if now.saturating_sub(started) > 15 {
        state.started.set(None);
        state
            .error
            .set("The device did not return shortcuts. Check the Home role, then refresh.".into());
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn catalog_groups_sorted_titles_and_filters_pins_without_duplicates() {
        let apps = vec![
            DeviceApplication {
                id: "z".into(),
                title: "Zebra".into(),
            },
            DeviceApplication {
                id: "a".into(),
                title: "alpha".into(),
            },
            DeviceApplication {
                id: "n".into(),
                title: "123".into(),
            },
        ];
        let pinned = BTreeSet::from([path("a")]);
        let all = rows(apps.clone(), "", &pinned, false);
        assert_eq!(
            all.iter()
                .filter_map(|row| if let Row::Header(value) = row {
                    Some(value.as_str())
                } else {
                    None
                })
                .collect::<Vec<_>>(),
            ["#", "A", "Z"]
        );
        assert_eq!(rows(apps.clone(), "", &pinned, true).len(), 2);
        assert_eq!(rows(apps, "zebra", &pinned, false).len(), 2);
    }
}
