//! Replaceable OS integration contracts. Providers execute on their UI thread;
//! the future carries completion, never permission to mutate a phone document.
pub mod device;
mod registry;
mod validation;
pub use device::*;
pub use registry::{HostService, ServiceRegistry};
pub use validation::{validate_applications, MAX_DEVICE_APPLICATIONS};

use crate::HostPlatform;
use serde::{Deserialize, Serialize};
use std::{future::Future, pin::Pin};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum HostMode {
    #[default]
    App,
    Launcher,
    Shell,
    Embedded,
}
impl HostMode {
    pub fn parse(value: &str) -> Result<Self, String> {
        match value {
            "app" => Ok(Self::App),
            "launcher" => Ok(Self::Launcher),
            "shell" => Ok(Self::Shell),
            "embedded" => Ok(Self::Embedded),
            _ => Err("Host mode must be app, launcher, shell or embedded.".into()),
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            Self::App => "application",
            Self::Launcher => "launcher",
            Self::Shell => "desktop shell",
            Self::Embedded => "embedded surface",
        }
    }
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum HostRole {
    #[default]
    Unsupported,
    Available,
    Requested,
    Active,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum HostFeature {
    InstalledApps,
    LauncherRole,
    ShellSession,
    SystemSettings,
    NativeSharing,
    Haptics,
    Notifications,
    Location,
    HardwareControls,
    Telephony,
    Messaging,
    Contacts,
    NotificationCenter,
    Maps,
    AppShortcuts,
}
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct PlatformState {
    pub platform: HostPlatform,
    pub mode: HostMode,
    pub launcher: HostRole,
    pub shell: HostRole,
    pub features: Vec<HostFeature>,
    pub detail: String,
}
impl PlatformState {
    pub fn supports_device_snapshot(&self) -> bool {
        self.features.iter().any(|feature| {
            matches!(
                feature,
                HostFeature::InstalledApps
                    | HostFeature::HardwareControls
                    | HostFeature::Telephony
                    | HostFeature::Messaging
                    | HostFeature::Contacts
                    | HostFeature::NotificationCenter
                    | HostFeature::Maps
                    | HostFeature::AppShortcuts
            )
        })
    }
    pub fn supports(&self, feature: HostFeature) -> bool {
        self.features.contains(&feature)
    }
    pub fn validate(&self) -> Result<(), String> {
        if self.features.len() > 15
            || self.detail.len() > 4096
            || self.detail.chars().any(|c| c.is_control() && c != '\n')
        {
            return Err("Invalid platform description.".into());
        }
        let mut features = self.features.clone();
        features.sort();
        features.dedup();
        if features.len() != self.features.len() {
            return Err("Duplicate platform capability.".into());
        }
        if matches!(self.platform, HostPlatform::Browser | HostPlatform::Ios)
            && (self.launcher != HostRole::Unsupported
                || self.shell != HostRole::Unsupported
                || self.supports(HostFeature::LauncherRole)
                || self.supports(HostFeature::ShellSession)
                || self.supports(HostFeature::InstalledApps)
                || matches!(self.mode, HostMode::Launcher | HostMode::Shell))
        {
            return Err(
                "This platform does not expose replacement launcher or shell roles.".into(),
            );
        }
        if self.mode == HostMode::Launcher && !self.supports(HostFeature::InstalledApps) {
            return Err("This host cannot enumerate launchable applications.".into());
        }
        if self.mode == HostMode::Shell
            && (!self.supports(HostFeature::ShellSession) || self.shell != HostRole::Active)
        {
            return Err(
                "A configured shell session is required before starting shell mode.".into(),
            );
        }
        Ok(())
    }
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeviceApplication {
    pub id: String,
    pub title: String,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum SystemSettingsPage {
    Application,
    Network,
    Notifications,
    DefaultApps,
    Wifi,
    Bluetooth,
    Cellular,
    Airplane,
    Hotspot,
    Vpn,
    Nfc,
    Location,
    Display,
    Brightness,
    Rotation,
    ScreenTimeout,
    Sound,
    AudioOutput,
    DoNotDisturb,
    Battery,
    BatterySaver,
    Storage,
    Security,
    Biometrics,
    Accounts,
    Sync,
    DateTime,
    Language,
    Keyboard,
    Accessibility,
    Privacy,
    Permissions,
    NotificationAccess,
    WriteSettings,
    Phone,
    Messaging,
    DataUsage,
    DataSaver,
    Cast,
    NearbyShare,
    NightLight,
    SystemUpdate,
    About,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum HapticKind {
    Selection,
    Light,
    Medium,
    Heavy,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HostLifecycle {
    Resumed,
    Suspended,
    Closing,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum PlatformRequest {
    Refresh,
    ListApplications,
    LaunchApplication {
        id: String,
    },
    RequestLauncher,
    OpenSettings {
        page: SystemSettingsPage,
    },
    Haptic {
        #[serde(rename = "feedback")]
        kind: HapticKind,
    },
    ShareText {
        title: String,
        text: String,
        url: String,
    },
    RequestNotifications,
    Notify {
        title: String,
        body: String,
    },
    Locate,
    Device {
        request: DeviceRequest,
    },
}
impl PlatformRequest {
    pub fn feature(&self) -> Option<HostFeature> {
        use HostFeature::*;
        Some(match self {
            Self::Refresh => return None,
            Self::ListApplications | Self::LaunchApplication { .. } => InstalledApps,
            Self::RequestLauncher => LauncherRole,
            Self::OpenSettings { .. } => SystemSettings,
            Self::Haptic { .. } => Haptics,
            Self::ShareText { .. } => NativeSharing,
            Self::RequestNotifications | Self::Notify { .. } => Notifications,
            Self::Locate => Location,
            Self::Device { request } => return request.feature(),
        })
    }
    pub fn validate(&self) -> Result<(), String> {
        validation::request(self)
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "kind", content = "value", rename_all = "camelCase")]
pub enum PlatformResponse {
    State(PlatformState),
    Applications(Vec<DeviceApplication>),
    Accepted { message: String },
    Location { latitude: f64, longitude: f64 },
    Device(DeviceResponse),
}
pub type PlatformTask = Pin<Box<dyn Future<Output = Result<PlatformResponse, String>>>>;
pub fn ready(result: Result<PlatformResponse, String>) -> PlatformTask {
    Box::pin(std::future::ready(result))
}
pub trait PlatformPlugin {
    fn id(&self) -> &'static str;
    fn state(&self) -> PlatformState;
    fn set_notification_policy(&self, _policy: &DeviceNotificationPolicy) {}
    fn environment(&self) -> crate::HostEnvironment {
        crate::HostEnvironment {
            platform: self.state().platform,
            ..Default::default()
        }
    }
    /// Own the validity of `environment().online`, including `None` while a
    /// fresh network-path observation is pending or unavailable. The runtime
    /// clears this owner's previous value on suspension. Other providers keep
    /// the existing optional-observation merge behavior. This reports network
    /// path availability, not successful access to an Internet service.
    fn owns_network_observation(&self) -> bool {
        false
    }
    /// Optional native handoff for a caller-validated app-owned file. This is
    /// deliberately not an SDK request accepting arbitrary filesystem paths.
    /// Providers independently check their sandbox and issue temporary grants.
    #[cfg(not(any(target_arch = "wasm32", target_os = "emscripten")))]
    fn open_file(&self, _path: &std::path::Path, _mime: &str) -> Option<PlatformTask> {
        None
    }

    #[cfg(not(any(target_arch = "wasm32", target_os = "emscripten")))]
    fn media_backend(
        &self,
    ) -> Result<Option<Box<dyn crate::native::media_backend::MediaBackend>>, String> {
        Ok(None)
    }
    /// Called synchronously during input dispatch. Start consent/share actions
    /// here when they require user activation; return their eventual result.
    fn dispatch(&self, request: PlatformRequest) -> PlatformTask;
    /// Drain bounded, coalesced native observations on the owning UI thread.
    /// Events carry observations, never commands or document-write authority.
    /// A provider must stop listeners and clear queued events on Closing.
    fn drain_events(&self) -> Vec<PlatformResponse> {
        Vec::new()
    }
    fn lifecycle(&self, _event: HostLifecycle) {}
}
