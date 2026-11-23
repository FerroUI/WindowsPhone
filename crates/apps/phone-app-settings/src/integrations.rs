//! Settings entry points into messaging, browsing, photos and location.
use super::update;
use ferroui_view::prelude::*;
use phone_controls::{action, flat, toggle, Icon};
use phone_domain::{AppId, Route};
use phone_sdk::common::{muted, text};
use phone_state::{HostRequest, PhoneViewModel};
use std::rc::Rc;

pub(super) fn photos(vm: Rc<PhoneViewModel>) -> Element {
    let import = vm.clone();
    let capture = vm.clone();
    let open = vm.clone();
    rsx! {StackPanel{spacing:14.0,
        {flat("choose photos",move ||import.request(HostRequest::ImportPhotos))}
        {flat("open camera",move ||capture.navigate(Route::App(AppId::Camera)))}
        {flat("open Photos",move ||open.navigate(Route::App(AppId::Photos)))}
        {muted("Imported pictures are copied into this app. Share or export a backup to keep them elsewhere.",16.0)}
    }}
}

pub(super) fn browser(vm: Rc<PhoneViewModel>) -> Element {
    let count = vm.data().bookmarks.len();
    let open = vm.clone();
    rsx! {StackPanel{spacing:18.0,
        {text(&format!("{count} favorites"),27.0)}
        {flat("open Internet Explorer",move ||open.navigate(Route::App(AppId::InternetExplorer)))}
        {muted("Web addresses open with your browser. Website security settings determine whether a page can be embedded.",16.0)}
    }}
}

pub(super) fn messaging(vm: Rc<PhoneViewModel>) -> Element {
    if vm
        .platform_state()
        .supports(phone_sdk::host::platform::HostFeature::Messaging)
    {
        return super::device::communications(vm);
    }
    let enabled = vm.data().settings.notifications;
    let model = vm.clone();
    let open = vm.clone();
    rsx! {StackPanel{spacing:18.0,
        {toggle("Notifications",enabled,move |value|update(&model,|settings|settings.notifications=value))}
        {flat("open messaging",move ||open.navigate(Route::App(AppId::Messaging)))}
        {muted("Sending a text opens your device's messaging app. Local drafts do not indicate delivery.",16.0)}
    }}
}

pub(super) fn location(vm: Rc<PhoneViewModel>) -> Element {
    let value = vm.data().settings.location;
    let model = vm.clone();
    let locate = vm.clone();
    rsx! {StackPanel{spacing:18.0,
        if vm.platform_state().supports(phone_sdk::host::platform::HostFeature::HardwareControls) {
            {phone_sdk::device::control(vm.clone(),phone_sdk::host::platform::DeviceControl::Location)}
            {phone_sdk::device::permission(vm.clone(),phone_sdk::host::platform::DevicePermission::Location,"location")}
        }
        {toggle("Use location in this app",value,move |enabled|update(&model,|settings|settings.location=enabled))}
        {action(Icon::Location,"find my location",move ||locate.request(HostRequest::Locate))}
        {muted("Your device asks for permission before sharing your location.",16.0)}
    }}
}
