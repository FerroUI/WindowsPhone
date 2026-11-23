use crate::{path, request};
use ferroui_base::{layout::HorizontalAlignment as H, BoxedValue};
use ferroui_controls::ItemsSource;
use ferroui_view::prelude::*;
use phone_controls::{action, app_bar, flat, input, Icon};
use phone_host::platform::*;
use phone_sdk::{common::*, AppContext};
use std::{cell::RefCell, rc::Rc};
struct State {
    query: Signal<String>,
    tab: Signal<bool>,
}
impl Default for State {
    fn default() -> Self {
        Self {
            query: signal(String::new()),
            tab: signal(false),
        }
    }
}
struct Cached {
    rows: Vec<DeviceApplication>,
    source: ItemsSource,
}
impl Cached {
    fn new(rows: Vec<DeviceApplication>) -> Self {
        let source = ItemsSource::from_items(
            rows.iter()
                .cloned()
                .map(|row| Some(Rc::new(row) as BoxedValue)),
        );
        Self { rows, source }
    }
}
fn catalog(cx: AppContext) -> Element {
    let state = cx.session.state::<State>();
    let query = state.query.get().trim().to_lowercase();
    let rows = cx
        .vm
        .device_applications()
        .into_iter()
        .filter(|app| app.title.to_lowercase().contains(&query))
        .collect::<Vec<_>>();
    let empty = rows.is_empty();
    let cache = use_hook(|| Rc::new(RefCell::new(Cached::new(rows.clone()))));
    let source = {
        let mut cached = cache.borrow_mut();
        if cached.rows != rows {
            *cached = Cached::new(rows);
        }
        cached.source.clone()
    };
    let item_template = use_hook({
        let cx = cx.clone();
        move || {
            template(move |app: DeviceApplication| {
                let open = cx.clone();
                let pin = cx.clone();
                let id = app.id.clone();
                let pin_id = id.clone();
                let title = app.title.clone();
                rsx! {Button{class:"phone-flat",min_height:64.0,horizontal_alignment:H::Stretch,horizontal_content_alignment:H::Stretch,padding:(20.0,10.0),automation_properties_name:app.title.clone(),
                on_click:move |_|request(&open,PlatformRequest::LaunchApplication{id:id.clone()}),
                on_context_requested:move|event|{match pin.pin(&path(&pin_id),&title){Ok(())=>pin.vm.set_status("Pinned device app to Start.".into()),Err(error)=>pin.vm.set_status(error)}event.set_handled(true);},
                {text(&app.title,26.0)}}}
            })
        }
    });
    rsx! {Grid{row_definitions:"Auto,Auto,*",row_spacing:10.0,
        Border{margin:(20.0,4.0),{input("search device apps").name("DeviceAppSearch").text(state.query.two_way())}}
        Border{Grid.row:1,margin:(20.0,0.0),{muted("Tap to open. Hold an app to pin it to Start.",15.0)}}
        if empty {Border{Grid.row:2,margin:20.0,{muted(if query.is_empty(){"No launchable device apps are listed. Use refresh to check again."}else{"No device apps match your search."},18.0)}}}
        ListBox{Grid.row:2,name:"DeviceApplicationList",class:"phone-list",items_source:source,item_template:item_template}
    }}
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
    let detail = state.detail.clone();
    let mut body = stack_panel()
        .margin(20.0)
        .spacing(16.0)
        .child(text(
            &format!("{} · {}", state.platform.label(), state.mode.label()),
            28.0,
        ))
        .child(muted(&detail, 18.0))
        .child(muted(
            &format!(
                "Home role: {}\nShell session: {}",
                role_label(state.launcher),
                role_label(state.shell)
            ),
            17.0,
        ));
    for (feature, label, request_value) in [
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
            let context = cx.clone();
            body = body.child(flat(label, move || {
                request(&context, request_value.clone())
            }));
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
    let catalog_context = cx.clone();
    let content = if tools {
        integrations(&cx)
    } else {
        rerender(move || catalog(catalog_context.clone()))
    };
    let body = rsx! {Grid{row_definitions:"Auto,*",
        StackPanel{orientation:ferroui_base::layout::Orientation::Horizontal,spacing:18.0,margin:(20.0,6.0,20.0,14.0),
            if supported {{flat("apps",move||catalog_state.tab.set(false))}}
            {flat("device",move||tools_state.tab.set(true))}}
        Border{Grid.row:1,{content}}
    }};
    let refresh = cx.clone();
    let state_refresh = cx.clone();
    page(
        "device apps",
        body,
        app_bar(
            vec![action(Icon::Refresh, "refresh device apps", move || {
                request(&state_refresh, PlatformRequest::Refresh);
                if supported {
                    request(&refresh, PlatformRequest::ListApplications);
                }
            })],
            false,
            vec![],
        ),
    )
}
