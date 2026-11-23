//! Native phone controls for explicit host permissions and observed storage.
use crate::common::{muted, text};
use ferroui_view::prelude::*;
use phone_controls::flat;
use phone_state::{HostPermission, HostPlatform, HostRequest, PhoneViewModel};
use std::rc::Rc;

pub(super) fn notifications(vm: Rc<PhoneViewModel>) -> Element {
    let environment = vm.environment();
    let allow = vm.clone();
    let test = vm.clone();
    let enabled = vm.data().settings.notifications;
    rsx! {StackPanel{spacing:12.0,
        {text("Device notifications",24.0)}
        {muted(environment.notification_permission.label(),17.0)}
        if environment.notification_permission==HostPermission::Prompt {
            {flat("allow device notifications",move ||allow.request(HostRequest::RequestNotificationPermission))}
        }
        if environment.notification_permission==HostPermission::Granted && enabled {
            {flat("send a test notification",move ||test.request(HostRequest::Notify{
                title:"Windows Phone".into(),body:"Device notifications are connected.".into()
            }))}
        }
        if environment.platform==HostPlatform::Browser {
            {muted("Allowed appointment and alarm banners can also appear on your device while this app is running. Permission is requested only when you choose Allow. Your browser controls delivery.",16.0)}
        }
    }}
}

pub(super) fn storage(vm: Rc<PhoneViewModel>) -> Element {
    let environment = vm.environment();
    let persist = vm.clone();
    let refresh = vm.clone();
    let usage = environment.storage_usage_bytes.map(size);
    let quota = environment.storage_quota_bytes.map(size);
    rsx! {StackPanel{spacing:12.0,
        {text("Device app storage",26.0)}
        if let Some(usage)=usage {{text(&usage,36.0)}{muted("estimated space used by this site's data",16.0)}}
        if let Some(quota)=quota {{muted(&format!("Available site quota: {quota}"),17.0)}}
        {muted(match environment.storage_persisted {
            Some(true)=>"App data is stored persistently. Keep a separate backup to recover deleted data.",
            Some(false)=>"Your browser may clear app data when it needs space.",
            None=>"This host has not reported a storage policy.",
        },16.0)}
        if environment.persistent_storage_supported==Some(true) && environment.storage_persisted!=Some(true) {
            {flat("keep app data on this device",move ||persist.request(HostRequest::RequestPersistentStorage))}
        }
        if environment.platform==HostPlatform::Browser {
            {flat("refresh storage information",move ||refresh.request(HostRequest::RefreshEnvironment))}
            {muted("The browser reports estimates for this site's origin, including other apps on the same origin. The quota is not your device's free disk space.",15.0)}
        }
    }}
}

fn size(bytes: u64) -> String {
    if bytes < 1024 {
        format!("{bytes} B")
    } else if bytes < 1024 * 1024 {
        format!("{:.1} KB", bytes as f64 / 1024.0)
    } else if bytes < 1024 * 1024 * 1024 {
        format!("{:.2} MB", bytes as f64 / 1_048_576.0)
    } else {
        format!("{:.2} GB", bytes as f64 / 1_073_741_824.0)
    }
}
