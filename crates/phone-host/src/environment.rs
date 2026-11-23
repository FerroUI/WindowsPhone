use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum HostPlatform {
    Browser,
    Windows,
    Macos,
    Linux,
    Android,
    Ios,
    #[default]
    Unknown,
}

impl HostPlatform {
    pub fn label(self) -> &'static str {
        match self {
            Self::Browser => "web browser",
            Self::Windows => "Windows",
            Self::Macos => "macOS",
            Self::Linux => "Linux",
            Self::Android => "Android",
            Self::Ios => "iOS",
            Self::Unknown => "this device",
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum HostPermission {
    Unsupported,
    Prompt,
    Granted,
    Denied,
    #[default]
    Unknown,
}

impl HostPermission {
    pub fn label(self) -> &'static str {
        match self {
            Self::Unsupported => "unavailable on this host",
            Self::Prompt => "permission not requested",
            Self::Granted => "allowed",
            Self::Denied => "blocked in host settings",
            Self::Unknown => "not reported",
        }
    }
}

/// Optional facts are deliberately unknown until a host reports them. A browser
/// quota is an origin estimate, not physical disk capacity or free space.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct HostEnvironment {
    pub platform: HostPlatform,
    pub battery_percent: Option<u8>,
    pub charging: Option<bool>,
    pub external_power: Option<bool>,
    pub power_saver: Option<bool>,
    pub online: Option<bool>,
    pub notification_permission: HostPermission,
    pub storage_usage_bytes: Option<u64>,
    pub storage_quota_bytes: Option<u64>,
    pub storage_persisted: Option<bool>,
    pub persistent_storage_supported: Option<bool>,
    pub media_session_supported: Option<bool>,
}

impl HostEnvironment {
    pub fn normalize(&mut self) {
        self.battery_percent = self.battery_percent.map(|value| value.min(100));
        // JavaScript numbers cannot represent arbitrary u64 values exactly.
        const MAX_SAFE_INTEGER: u64 = 9_007_199_254_740_991;
        self.storage_usage_bytes = self.storage_usage_bytes.filter(|v| *v <= MAX_SAFE_INTEGER);
        self.storage_quota_bytes = self.storage_quota_bytes.filter(|v| *v <= MAX_SAFE_INTEGER);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn old_environment_remains_unknown_and_observations_are_bounded() {
        let mut value: HostEnvironment = serde_json::from_str(
            r#"{"battery_percent":255,"online":false,"storage_quota_bytes":18446744073709551615}"#,
        )
        .unwrap();
        value.normalize();
        assert_eq!(value.battery_percent, Some(100));
        assert_eq!(value.online, Some(false));
        assert_eq!(value.notification_permission, HostPermission::Unknown);
        assert_eq!(value.storage_quota_bytes, None);
        assert_eq!(value.charging, None);
    }
}
