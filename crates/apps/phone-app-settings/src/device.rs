use ferroui_view::prelude::*;
use phone_controls::flat;
use phone_sdk::{
    common::{muted, row, text},
    device,
    host::platform::*,
};
use phone_state::PhoneViewModel;
use std::rc::Rc;

pub(super) fn control_for_page(name: &str) -> Option<DeviceControl> {
    if let Some(key) = name.strip_prefix("device/") {
        return DeviceControl::from_key(key);
    }
    Some(match name {
        "Wi-Fi" => DeviceControl::Wifi,
        "Bluetooth" => DeviceControl::Bluetooth,
        "cellular+SIM" => DeviceControl::Cellular,
        "flight mode" => DeviceControl::Airplane,
        "screen rotation" => DeviceControl::Rotation,
        "brightness" => DeviceControl::Brightness,
        "internet sharing" => DeviceControl::Hotspot,
        "tap+send" => DeviceControl::Nfc,
        "quiet hours" => DeviceControl::DoNotDisturb,
        "flashlight" => DeviceControl::Flashlight,
        "automatic brightness" => DeviceControl::AutoBrightness,
        "screen timeout" => DeviceControl::ScreenTimeout,
        "night light" => DeviceControl::NightLight,
        "project my screen" => DeviceControl::Cast,
        "nearby sharing" => DeviceControl::NearbyShare,
        "VPN" => DeviceControl::Vpn,
        "audio output" => DeviceControl::AudioOutput,
        "account sync" => DeviceControl::Sync,
        "data saver" => DeviceControl::DataSaver,
        _ => return None,
    })
}
pub(super) fn view(vm: Rc<PhoneViewModel>) -> Element {
    let state = vm.device_state();
    let mut content = stack_panel()
        .name("DeviceSettingsRoot")
        .spacing(12.0)
        .child(text("device services", 30.0));
    content = content.child(device::connections(vm.clone()));
    for (label, page) in device::SETTINGS_PAGES {
        if state.snapshot.settings.contains(page) {
            let model = vm.clone();
            let page = *page;
            content = content.child(row(label, "open device settings", move || {
                model.request(phone_state::HostRequest::Platform {
                    request: PlatformRequest::OpenSettings { page },
                })
            }));
        }
    }
    if state.snapshot.settings.is_empty() {
        content = content.child(muted(
            "The host has not reported any supported system settings pages.",
            17.0,
        ));
    }
    if !state.snapshot.detail.is_empty() {
        content = content.child(muted(&state.snapshot.detail, 16.0));
    }
    let refresh = vm.clone();
    content
        .child(flat("refresh", move || refresh.refresh_device()))
        .into()
}
pub(super) fn communications(vm: Rc<PhoneViewModel>) -> Element {
    rsx! {StackPanel{name:"DeviceCommunicationsSettings",spacing:22.0,
        {device::role(vm.clone(),DeviceRole::Dialer)}
        {device::permission(vm.clone(),DevicePermission::Phone,"phone calls")}
        {device::permission(vm.clone(),DevicePermission::CallLog,"call history")}
        {device::role(vm.clone(),DeviceRole::Sms)}
        {device::permission(vm.clone(),DevicePermission::ReadSms,"messages")}
        {device::permission(vm.clone(),DevicePermission::SendSms,"sending messages")}
        {device::permission(vm.clone(),DevicePermission::Contacts,"contacts")}
    }}
}
