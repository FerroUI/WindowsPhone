//! A gesture owns a group identity and the document that displayed it. Native
//! keys are captured at contact/command creation and rechecked against the live
//! provider, so an arrival, revocation or reused key cannot broaden dismissal.
use phone_domain::{AppId, Id};
use phone_host::platform::{DeviceOperation, DeviceRequest};
use phone_state::PhoneViewModel;

#[derive(Clone, Debug)]
pub(crate) struct Dismissal {
    name: String,
    epoch: u64,
    group: Identity,
}

#[derive(Clone, Debug)]
enum Identity {
    Local { app: AppId, anchor: Id },
    Device { package: String, keys: Vec<String> },
}

impl Dismissal {
    pub fn from_name(vm: &PhoneViewModel, name: &str) -> Option<Self> {
        let anchor = name
            .strip_prefix("Notification-")
            .and_then(|id| id.parse::<Id>().ok());
        let package = name.strip_prefix("DeviceNotifications-");
        if anchor.is_none() && package.is_none() {
            return None;
        }
        let settings = vm.data().settings.clone();
        if !settings.notifications {
            return None;
        }
        let group = if let Some(anchor) = anchor {
            let app = vm
                .data()
                .notifications
                .iter()
                .find(|item| item.id == anchor)?
                .app;
            if !settings.notifications_allowed(app) {
                return None;
            }
            Identity::Local { app, anchor }
        } else {
            let package = package?;
            let keys = crate::device_notifications::clearable_keys(vm, Some(package));
            if keys.is_empty() {
                return None;
            }
            Identity::Device {
                package: package.into(),
                keys,
            }
        };
        Some(Self {
            name: name.into(),
            epoch: vm.document_epoch(),
            group,
        })
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    /// Rendering already owns a validated provider snapshot. Reuse its keys
    /// instead of cloning the complete device store for every group header.
    pub fn device(vm: &PhoneViewModel, package: &str, keys: Vec<String>) -> Option<Self> {
        (!keys.is_empty()).then(|| Self {
            name: format!("DeviceNotifications-{package}"),
            epoch: vm.document_epoch(),
            group: Identity::Device {
                package: package.into(),
                keys,
            },
        })
    }

    pub fn dismiss(&self, vm: &PhoneViewModel) {
        if vm.document_epoch() != self.epoch || !vm.data().settings.notifications {
            return;
        }
        match &self.group {
            Identity::Local { app, anchor } => {
                let owned = {
                    let data = vm.data();
                    data.settings.notifications_allowed(*app)
                        && data
                            .notifications
                            .iter()
                            .any(|item| item.id == *anchor && item.app == *app)
                };
                if owned {
                    vm.dismiss_notification_group(*anchor);
                }
            }
            Identity::Device { package, keys } => {
                let device = vm.device_state();
                if !device.snapshot.supports(DeviceOperation::ReadNotifications)
                    || !device
                        .snapshot
                        .supports(DeviceOperation::NotificationActions)
                {
                    return;
                }
                let keys = device
                    .notifications
                    .iter()
                    .filter(|item| {
                        item.package_id == *package && item.clearable && keys.contains(&item.key)
                    })
                    .map(|item| item.key.clone())
                    .collect::<Vec<_>>();
                if !keys.is_empty() {
                    vm.request_device(DeviceRequest::DismissNotifications { keys });
                }
            }
        }
    }
}
