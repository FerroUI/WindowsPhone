use crate::{native, pending::Session};
#[cfg(target_os = "ios")]
use phone_host::HostPermission;
use phone_host::{platform::*, HostEnvironment, HostPlatform};
use std::rc::Rc;

pub struct IosPlatform {
    session: Rc<Session>,
    network: crate::network::NetworkMonitor,
    mode: HostMode,
}

impl Default for IosPlatform {
    fn default() -> Self {
        Self::new()
    }
}

impl IosPlatform {
    pub fn new() -> Self {
        native::monitor_power(true);
        native::monitor_device(true);
        let network = crate::network::NetworkMonitor::default();
        network.set_enabled(true);
        Self {
            session: Rc::new(Session::default()),
            network,
            mode: HostMode::App,
        }
    }

    pub fn embedded() -> Self {
        Self {
            mode: HostMode::Embedded,
            ..Self::new()
        }
    }

    #[cfg(target_os = "ios")]
    fn start(&self, request: PlatformRequest) -> Result<PlatformTask, String> {
        let state = self.state();
        let task = match request {
            PlatformRequest::Refresh => {
                let task = native::permission(&self.session, false)?;
                return Ok(Box::pin(async move { task.await?; Ok(PlatformResponse::State(state)) }));
            }
            PlatformRequest::RequestNotifications => {
                let task = native::permission(&self.session, true)?;
                return Ok(Box::pin(async move { task.await?; Ok(PlatformResponse::State(state)) }));
            }
            PlatformRequest::Device { request } => return crate::device::start(&self.session, request),
            PlatformRequest::OpenSettings { page } => native::settings(&self.session, match page {
                SystemSettingsPage::Application | SystemSettingsPage::Permissions | SystemSettingsPage::Privacy => 0,
                SystemSettingsPage::Notifications => 2,
                SystemSettingsPage::DefaultApps => 3,
                SystemSettingsPage::Sound | SystemSettingsPage::AudioOutput | SystemSettingsPage::Cast => {
                    return crate::device::start(&self.session, device::DeviceRequest::OpenControl {
                        control: match page {
                            SystemSettingsPage::Sound => device::DeviceControl::MediaVolume,
                            SystemSettingsPage::Cast => device::DeviceControl::Cast,
                            _ => device::DeviceControl::AudioOutput,
                        }
                    });
                }
                _ => return Err("iOS does not publish a direct link to this system settings page. Open Settings to change it.".into()),
            })?,
            PlatformRequest::Haptic { kind } => {
                let kind = match kind { HapticKind::Selection => 0, HapticKind::Light => 1, HapticKind::Medium => 2, HapticKind::Heavy => 3 };
                return Ok(ready(if native::haptic(kind) {
                    Ok(PlatformResponse::Accepted { message: "Haptic feedback requested.".into() })
                } else { Err("Haptic feedback requires the active UIKit application.".into()) }));
            }
            PlatformRequest::ShareText { title, text, url } => {
                let url = if url.is_empty() { String::new() } else { phone_host::urls::web_url(&url)? };
                let text = [title, text].into_iter().filter(|value| !value.is_empty()).collect::<Vec<_>>().join("\n");
                native::share(&self.session, &text, &url)?
            }
            PlatformRequest::Notify { title, body } => {
                if native::notification_permission() != HostPermission::Granted {
                    return Err("Allow notifications in device settings before sending a banner.".into());
                }
                native::notify(&self.session, &title, &body)?
            }
            PlatformRequest::Locate => native::locate(&self.session)?,
            PlatformRequest::ListApplications | PlatformRequest::LaunchApplication { .. } =>
                return Err("iOS does not expose an installed-application catalog or bundle-ID launcher. Use explicit supported URL handoffs.".into()),
            PlatformRequest::RequestLauncher =>
                return Err("iOS does not offer a replacement Home Screen or system-shell role to this application.".into()),
        };
        Ok(Box::pin(async move {
            let value = task.await?;
            if value.code == 3 {
                if !value.first.is_finite()
                    || !value.second.is_finite()
                    || !(-90.0..=90.0).contains(&value.first)
                    || !(-180.0..=180.0).contains(&value.second)
                {
                    return Err("The iOS location provider returned invalid coordinates.".into());
                }
                Ok(PlatformResponse::Location {
                    latitude: value.first,
                    longitude: value.second,
                })
            } else {
                Ok(PlatformResponse::Accepted {
                    message: value.message,
                })
            }
        }))
    }
}

impl PlatformPlugin for IosPlatform {
    fn set_notification_policy(&self, policy: &DeviceNotificationPolicy) {
        native::set_notification_policy(policy);
    }

    fn id(&self) -> &'static str {
        "org.ferroui.platform.ios"
    }

    fn state(&self) -> PlatformState {
        PlatformState {
            platform: HostPlatform::Ios,
            mode: self.mode,
            launcher: HostRole::Unsupported,
            shell: HostRole::Unsupported,
            features: if cfg!(target_os = "ios") {
                vec![HostFeature::SystemSettings, HostFeature::NativeSharing, HostFeature::Haptics,
                     HostFeature::Notifications, HostFeature::Location, HostFeature::HardwareControls,
                     HostFeature::Contacts, HostFeature::Telephony, HostFeature::Messaging, HostFeature::Maps]
            } else { Vec::new() },
            detail: "UIKit application or embedded surface. iOS controls Home Screen, installed-app visibility and system-shell privileges.".into(),
        }
    }

    fn environment(&self) -> HostEnvironment {
        let (battery_percent, charging, power_saver) = native::power();
        HostEnvironment {
            platform: HostPlatform::Ios,
            battery_percent,
            charging,
            external_power: charging,
            power_saver,
            online: self.network.online(),
            notification_permission: native::notification_permission(),
            storage_persisted: cfg!(target_os = "ios").then_some(true),
            persistent_storage_supported: Some(false),
            ..Default::default()
        }
    }

    fn owns_network_observation(&self) -> bool {
        cfg!(target_os = "ios")
    }

    fn dispatch(&self, request: PlatformRequest) -> PlatformTask {
        if let Err(error) = request.validate() {
            return ready(Err(error));
        }
        #[cfg(target_os = "ios")]
        {
            self.start(request)
                .unwrap_or_else(|error| ready(Err(error)))
        }
        #[cfg(not(target_os = "ios"))]
        {
            let _ = request;
            ready(Err(
                "The iOS integration requires UIKit on an iOS device or simulator.".into(),
            ))
        }
    }

    #[cfg(target_os = "ios")]
    fn open_file(&self, path: &std::path::Path, _mime: &str) -> Option<PlatformTask> {
        // UIKit infers the attachment type from the validated asset extension.
        // Both the native-only caller and bridge enforce app-owned file access.
        Some(match native::share_file(&self.session, path) {
            Ok(task) => Box::pin(async move {
                let reply = task.await?;
                Ok(PlatformResponse::Accepted {
                    message: reply.message,
                })
            }),
            Err(error) => ready(Err(error)),
        })
    }

    fn lifecycle(&self, event: HostLifecycle) {
        match event {
            HostLifecycle::Resumed => {
                self.network.set_enabled(true);
                native::monitor_power(true);
                native::monitor_device(true);
                #[cfg(target_os = "ios")]
                if let Ok(task) = native::permission(&self.session, false) {
                    // Returning from Settings observes the actual permission;
                    // this query never presents the authorization prompt.
                    drop(
                        ferroui_base::threading::Dispatcher::current_dispatcher().spawn_local(
                            async move {
                                let _ = task.await;
                            },
                        ),
                    );
                }
            }
            HostLifecycle::Suspended => {
                self.network.set_enabled(false);
                self.session.cancel_locations();
                self.session.cancel_device_reads();
                native::monitor_power(false);
                native::monitor_device(false);
            }
            HostLifecycle::Closing => {
                self.network.set_enabled(false);
                self.session.close();
                native::monitor_power(false);
                native::close_device();
            }
        }
    }

    fn drain_events(&self) -> Vec<PlatformResponse> {
        #[cfg(target_os = "ios")]
        {
            native::device_events()
        }
        #[cfg(not(target_os = "ios"))]
        {
            Vec::new()
        }
    }
}
