//! Reusable device-service UI. All mutations use typed host contracts and show
//! observed values; opening a settings page never changes an in-app radio flag.
use crate::common::{muted, text};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use ferroui_base::layout::Orientation;
use ferroui_view::prelude::*;
use phone_controls::{flat, toggle, Icon};
use phone_host::{platform::*, HostRequest};
use phone_state::PhoneViewModel;
use std::rc::Rc;

pub fn encode_route_value(value: &str) -> String {
    URL_SAFE_NO_PAD.encode(value)
}
pub fn decode_route_value(value: &str, max_bytes: usize) -> Result<String, String> {
    if value.len() > max_bytes.saturating_mul(2) {
        return Err("Device route exceeds its limit.".into());
    }
    let bytes = URL_SAFE_NO_PAD
        .decode(value)
        .map_err(|_| "Invalid device route encoding.")?;
    if bytes.len() > max_bytes {
        return Err("Device route exceeds its limit.".into());
    }
    String::from_utf8(bytes).map_err(|_| "Device route must be UTF-8.".into())
}
pub fn timestamp(vm: &PhoneViewModel, milliseconds: i64) -> String {
    let value = milliseconds
        .div_euclid(1000)
        .saturating_add(i64::from(vm.utc_offset()));
    let date = phone_domain::date_parts(value);
    let today = phone_domain::date_parts(vm.local_now());
    let time = phone_domain::time_text(value, vm.data().settings.time_24h);
    if date == today {
        time
    } else {
        format!(
            "{} · {time}",
            vm.data().settings.date_order.format(date.0, date.1, date.2)
        )
    }
}
/// Native ListBox recycling over an immutable observation snapshot. Unchanged
/// query results preserve ItemsSource identity and scroll/focus state.
pub fn list<T: Clone + PartialEq + 'static>(
    name: &'static str,
    rows: Vec<T>,
    render: Rc<dyn Fn(T) -> Element>,
) -> Element {
    list_inner(name, rows, render, None)
}
pub fn list_with_node<T: Clone + PartialEq + 'static>(
    name: &'static str,
    rows: Vec<T>,
    render: Rc<dyn Fn(T) -> Element>,
    node: NodeRef<ListBox>,
) -> Element {
    list_inner(name, rows, render, Some(node))
}
fn list_inner<T: Clone + PartialEq + 'static>(
    name: &'static str,
    rows: Vec<T>,
    render: Rc<dyn Fn(T) -> Element>,
    node: Option<NodeRef<ListBox>>,
) -> Element {
    use ferroui_base::BoxedValue;
    use ferroui_controls::ItemsSource;
    use std::cell::RefCell;
    struct Cache<T> {
        rows: Vec<T>,
        source: ItemsSource,
    }
    fn source<T: Clone + PartialEq + 'static>(rows: &[T]) -> ItemsSource {
        ItemsSource::from_items(rows.iter().cloned().map(|v| Some(Rc::new(v) as BoxedValue)))
    }
    let cache = use_hook(|| {
        Rc::new(RefCell::new(Cache {
            source: source(&rows),
            rows: rows.clone(),
        }))
    });
    let source = {
        let mut cache = cache.borrow_mut();
        if cache.rows != rows {
            cache.source = source(&rows);
            cache.rows = rows;
        }
        cache.source.clone()
    };
    let template = use_hook(move || template(move |value: T| render(value)));
    match node {
        Some(node) => {
            rsx! {ListBox{name:name,class:"phone-list",node_ref:node,items_source:source,item_template:template}}
        }
        None => {
            rsx! {ListBox{name:name,class:"phone-list",items_source:source,item_template:template}}
        }
    }
}

pub const SETTINGS_PAGES: &[(&str, SystemSettingsPage)] = &[
    ("application", SystemSettingsPage::Application),
    ("network", SystemSettingsPage::Network),
    ("notifications", SystemSettingsPage::Notifications),
    ("default apps", SystemSettingsPage::DefaultApps),
    ("Wi-Fi", SystemSettingsPage::Wifi),
    ("Bluetooth", SystemSettingsPage::Bluetooth),
    ("cellular+SIM", SystemSettingsPage::Cellular),
    ("flight mode", SystemSettingsPage::Airplane),
    ("internet sharing", SystemSettingsPage::Hotspot),
    ("VPN", SystemSettingsPage::Vpn),
    ("tap+send", SystemSettingsPage::Nfc),
    ("location", SystemSettingsPage::Location),
    ("display", SystemSettingsPage::Display),
    ("brightness", SystemSettingsPage::Brightness),
    ("screen rotation", SystemSettingsPage::Rotation),
    ("screen timeout", SystemSettingsPage::ScreenTimeout),
    ("sound", SystemSettingsPage::Sound),
    ("audio output", SystemSettingsPage::AudioOutput),
    ("quiet hours", SystemSettingsPage::DoNotDisturb),
    ("battery", SystemSettingsPage::Battery),
    ("battery saver", SystemSettingsPage::BatterySaver),
    ("storage", SystemSettingsPage::Storage),
    ("security", SystemSettingsPage::Security),
    ("biometrics", SystemSettingsPage::Biometrics),
    ("email+accounts", SystemSettingsPage::Accounts),
    ("account sync", SystemSettingsPage::Sync),
    ("date+time", SystemSettingsPage::DateTime),
    ("language", SystemSettingsPage::Language),
    ("keyboard", SystemSettingsPage::Keyboard),
    ("accessibility", SystemSettingsPage::Accessibility),
    ("privacy", SystemSettingsPage::Privacy),
    ("permissions", SystemSettingsPage::Permissions),
    (
        "notification access",
        SystemSettingsPage::NotificationAccess,
    ),
    ("change system settings", SystemSettingsPage::WriteSettings),
    ("phone", SystemSettingsPage::Phone),
    ("messaging", SystemSettingsPage::Messaging),
    ("data usage", SystemSettingsPage::DataUsage),
    ("data saver", SystemSettingsPage::DataSaver),
    ("project my screen", SystemSettingsPage::Cast),
    ("nearby sharing", SystemSettingsPage::NearbyShare),
    ("night light", SystemSettingsPage::NightLight),
    ("system update", SystemSettingsPage::SystemUpdate),
    ("device information", SystemSettingsPage::About),
];

pub fn icon(control: DeviceControl) -> Icon {
    match control {
        DeviceControl::Wifi | DeviceControl::Hotspot => Icon::Wifi,
        DeviceControl::Bluetooth => Icon::Bluetooth,
        DeviceControl::Airplane => Icon::Airplane,
        DeviceControl::Cellular | DeviceControl::DataSaver => Icon::Signal,
        DeviceControl::Location => Icon::Location,
        DeviceControl::Rotation => Icon::Rotation,
        DeviceControl::Brightness
        | DeviceControl::AutoBrightness
        | DeviceControl::Flashlight
        | DeviceControl::NightLight => Icon::Sun,
        DeviceControl::BatterySaver => Icon::Battery,
        DeviceControl::DoNotDisturb => Icon::Mute,
        DeviceControl::RingerVolume
        | DeviceControl::MediaVolume
        | DeviceControl::AlarmVolume
        | DeviceControl::RingerMode => Icon::Volume,
        DeviceControl::AudioOutput => Icon::Headphones,
        DeviceControl::ScreenTimeout => Icon::Timer,
        DeviceControl::Vpn => Icon::Lock,
        DeviceControl::Nfc | DeviceControl::NearbyShare => Icon::Share,
        DeviceControl::Sync => Icon::Refresh,
        DeviceControl::Cast => Icon::Video,
    }
}
pub fn value_label(value: &ControlValue) -> String {
    match value {
        ControlValue::Unknown => "not reported".into(),
        ControlValue::Toggle(value) => if *value { "on" } else { "off" }.into(),
        ControlValue::Level(value) => format!("{}%", (value * 100.0).round() as u32),
        ControlValue::Choice(value) => value.clone(),
    }
}
pub fn control_summary(vm: &PhoneViewModel, control: DeviceControl) -> String {
    vm.device_state().snapshot.control(control).map_or_else(
        || "device settings".into(),
        |state| {
            let value = value_label(&state.value);
            match state.access {
                ControlAccess::Panel => format!("{value} · system settings"),
                ControlAccess::Permission => format!("{value} · access required"),
                ControlAccess::Unavailable => "unavailable on this device".into(),
                _ => value,
            }
        },
    )
}
pub fn control(vm: Rc<PhoneViewModel>, control: DeviceControl) -> Element {
    let observed = vm.device_state().snapshot.control(control).cloned();
    let refresh = vm.clone();
    let Some(state) = observed else {
        return rsx! {StackPanel{spacing:12.0,
            {muted("Device information has not been reported.",17.0)}{flat("refresh device information",move||refresh.refresh_device())}
        }};
    };
    let mut body = stack_panel().spacing(14.0);
    match (&state.access, &state.value) {
        (ControlAccess::Writable, ControlValue::Toggle(value)) => {
            let model = vm.clone();
            body = body.child(toggle(control.label(), *value, move |value| {
                model.request_device(DeviceRequest::SetControl {
                    control,
                    value: ControlValue::Toggle(value),
                })
            }));
        }
        (ControlAccess::Writable, ControlValue::Level(value)) => {
            let model = vm.clone();
            let observed = *value;
            body = body.child(text(&value_label(&state.value), 38.0)).child(rsx!{Slider{
                class:"phone-slider",minimum:0.0,maximum:1.0,small_change:0.05,large_change:0.1,value:observed,
                automation_properties_name:format!("Device {}",control.label()),
                on_value_changed:move |event| { let next=event.new_value().clamp(0.0,1.0); if (next-observed).abs()>=0.01 { model.request_device(DeviceRequest::SetControl {control,value:ControlValue::Level(next)}); } }
            }});
        }
        (ControlAccess::Writable, ControlValue::Choice(_))
            if control == DeviceControl::RingerMode || control == DeviceControl::ScreenTimeout =>
        {
            body = body.child(text(&value_label(&state.value), 30.0));
            let values: &[(&str, &str)] = if control == DeviceControl::RingerMode {
                &[
                    ("ring", "normal"),
                    ("vibrate", "vibrate"),
                    ("silent", "silent"),
                ]
            } else {
                &[
                    ("15 seconds", "15"),
                    ("30 seconds", "30"),
                    ("1 minute", "60"),
                    ("2 minutes", "120"),
                    ("5 minutes", "300"),
                    ("10 minutes", "600"),
                ]
            };
            for (label, value) in values {
                let model = vm.clone();
                let value = (*value).to_owned();
                body = body.child(flat(label, move || {
                    model.request_device(DeviceRequest::SetControl {
                        control,
                        value: ControlValue::Choice(value.clone()),
                    })
                }));
            }
        }
        _ => {
            body = body.child(text(&value_label(&state.value), 30.0));
        }
    }
    if !state.detail.is_empty() {
        body = body.child(muted(&state.detail, 17.0));
    }
    if matches!(
        state.access,
        ControlAccess::Panel | ControlAccess::Permission
    ) {
        let model = vm.clone();
        body = body.child(flat(
            if state.access == ControlAccess::Permission {
                "allow device access"
            } else {
                "open device settings"
            },
            move || model.request_device(DeviceRequest::OpenControl { control }),
        ));
    }
    if state.access == ControlAccess::ReadOnly {
        body = body.child(muted(
            "This value is reported by the device. The operating system owns this setting.",
            16.0,
        ));
    }
    body.child(flat("refresh", move || refresh.refresh_device()))
        .into()
}
pub fn role(vm: Rc<PhoneViewModel>, role: DeviceRole) -> Element {
    let snapshot = vm.device_state().snapshot;
    let value = snapshot.roles.iter().find(|value| value.role == role);
    let status = value.map_or(HostRole::Unsupported, |v| v.status);
    let detail = value
        .map(|v| v.detail.as_str())
        .unwrap_or("This role is unavailable on this host.");
    let request = vm.clone();
    let refresh = vm.clone();
    rsx! {StackPanel{spacing:10.0,
        {text(role.label(),24.0)}
        {muted(match status {HostRole::Active=>"active",HostRole::Available=>"available",HostRole::Requested=>"waiting for system confirmation",HostRole::Unsupported=>"unavailable"},17.0)}
        if !detail.is_empty(){{muted(detail,16.0)}}
        if status==HostRole::Available {{flat("choose in system settings",move||request.request_device(DeviceRequest::RequestRole {role}))}}
        if status==HostRole::Requested {{flat("refresh role",move||refresh.refresh_device())}}
    }}
}
pub fn permission(
    vm: Rc<PhoneViewModel>,
    permission: DevicePermission,
    label: &'static str,
) -> Element {
    let status = vm.device_state().snapshot.permission(permission);
    let model = vm.clone();
    rsx! {StackPanel{spacing:8.0,
        {muted(&format!("{label}: {}",match status{PermissionState::Granted=>"allowed",PermissionState::Limited=>"selected items",PermissionState::Unknown=>"checking",PermissionState::Prompt=>"not requested",PermissionState::Denied=>"denied",PermissionState::Restricted=>"restricted",PermissionState::Unsupported=>"unavailable"}),16.0)}
        if matches!(status,PermissionState::Prompt|PermissionState::Denied|PermissionState::Limited) {
            {flat(&format!("allow {label}"),move||model.request_device(DeviceRequest::RequestPermission{permission}))}
        }
    }}
}
pub fn settings_link(vm: Rc<PhoneViewModel>, page: SystemSettingsPage, label: &str) -> Element {
    let model = vm.clone();
    flat(label, move || {
        model.request(HostRequest::Platform {
            request: PlatformRequest::OpenSettings { page },
        })
    })
}
pub fn connections(vm: Rc<PhoneViewModel>) -> Element {
    let snapshot = vm.device_state().snapshot;
    let refresh = vm.clone();
    let mut content = stack_panel().spacing(22.0);
    for value in snapshot.roles {
        if value.status != HostRole::Unsupported {
            content = content.child(role(vm.clone(), value.role));
        }
    }
    content.child(rsx!{StackPanel{orientation:Orientation::Horizontal,{flat("refresh device services",move||refresh.refresh_device())}}}).into()
}
