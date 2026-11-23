use phone_host::{
    platform::{
        validate_applications, DeviceApplication, HostFeature, HostMode, HostRole,
        PlatformResponse, PlatformState,
    },
    HostEnvironment, HostPermission, HostPlatform,
};
use serde::Deserialize;

const MAX_RESPONSE_BYTES: usize = 1024 * 1024;

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub(crate) struct Observation {
    pub launcher_package: bool,
    pub home_available: bool,
    pub home_held: bool,
    pub notifications: String,
    pub battery_percent: Option<u8>,
    pub charging: Option<bool>,
    pub external_power: Option<bool>,
    pub power_saver: Option<bool>,
    pub online: Option<bool>,
}

impl Observation {
    pub fn platform_state(&self, embedded: bool) -> PlatformState {
        let held = self.launcher_package && self.home_held;
        let launcher = if held {
            HostRole::Active
        } else if self.launcher_package && self.home_available {
            HostRole::Available
        } else {
            HostRole::Unsupported
        };
        let mut features = vec![
            HostFeature::InstalledApps,
            HostFeature::SystemSettings,
            HostFeature::NativeSharing,
            HostFeature::Haptics,
            HostFeature::Notifications,
            HostFeature::Location,
        ];
        if launcher != HostRole::Unsupported {
            features.push(HostFeature::LauncherRole);
        }
        PlatformState {
            platform: HostPlatform::Android,
            mode: if embedded { HostMode::Embedded } else if held { HostMode::Launcher } else { HostMode::App },
            launcher, shell: HostRole::Unsupported, features,
            detail: if embedded {
                "Embedded Android surface. The owner supplies the mounted FerroUI TopLevel and forwards lifecycle events. Android owns permissions and system roles; device-app discovery covers visible launchable activities in the current profile."
            } else if self.launcher_package {
                "Android owns the HOME role, system bars, permissions and process lifetime. Choose this app in Android's home-app dialog to use it as your launcher. Device apps are limited to visible launchable activities in the current profile."
            } else {
                "Application package. The separate launcher package declares Android's HOME intent filter. Device apps are limited to visible launchable activities in the current profile."
            }.into(),
        }
    }

    pub fn environment(self) -> HostEnvironment {
        let mut environment = HostEnvironment {
            platform: HostPlatform::Android,
            battery_percent: self.battery_percent,
            charging: self.charging,
            external_power: self.external_power,
            power_saver: self.power_saver,
            online: self.online,
            notification_permission: match self.notifications.as_str() {
                "granted" => HostPermission::Granted,
                "denied" => HostPermission::Denied,
                "prompt" => HostPermission::Prompt,
                "unavailable" => HostPermission::Unsupported,
                _ => HostPermission::Unknown,
            },
            // The shared media host reports actual MediaSession registration.
            media_session_supported: None,
            ..Default::default()
        };
        environment.normalize();
        environment
    }
}

#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum Reply {
    Pending,
    Accepted {
        message: String,
    },
    Applications {
        applications: Vec<DeviceApplication>,
    },
    Location {
        latitude: f64,
        longitude: f64,
    },
    Error {
        message: String,
    },
}

pub(crate) fn response(text: &str) -> Result<Option<PlatformResponse>, String> {
    if text.len() > MAX_RESPONSE_BYTES {
        return Err("The Android bridge response exceeds its limit.".into());
    }
    let reply: Reply = serde_json::from_str(text)
        .map_err(|_| "Android returned an invalid bridge response.".to_string())?;
    Ok(Some(match reply {
        Reply::Pending => return Ok(None),
        Reply::Error { message } => return Err(bounded_message(message)?),
        Reply::Accepted { message } => PlatformResponse::Accepted {
            message: bounded_message(message)?,
        },
        Reply::Applications { applications } => {
            validate_applications(&applications)?;
            PlatformResponse::Applications(applications)
        }
        Reply::Location {
            latitude,
            longitude,
        } => {
            if !latitude.is_finite()
                || !longitude.is_finite()
                || !(-90.0..=90.0).contains(&latitude)
                || !(-180.0..=180.0).contains(&longitude)
            {
                return Err("Android returned invalid location coordinates.".into());
            }
            PlatformResponse::Location {
                latitude,
                longitude,
            }
        }
    }))
}

fn bounded_message(message: String) -> Result<String, String> {
    if message.len() > 4096 || message.contains('\0') {
        Err("The Android bridge returned an invalid status message.".into())
    } else {
        Ok(message)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn embedding_and_home_selection_are_distinct_from_role_availability() {
        let mut value = Observation::default();
        assert_eq!(value.platform_state(false).mode, HostMode::App);
        value.home_held = true;
        assert_eq!(value.platform_state(false).launcher, HostRole::Unsupported);
        value.launcher_package = true;
        value.home_held = false;
        value.home_available = true;
        assert_eq!(value.platform_state(false).launcher, HostRole::Available);
        assert_eq!(value.platform_state(false).mode, HostMode::App);
        value.home_held = true;
        assert_eq!(value.platform_state(false).mode, HostMode::Launcher);
        let embedded = value.platform_state(true);
        assert_eq!(embedded.mode, HostMode::Embedded);
        assert_eq!(embedded.launcher, HostRole::Active);
        assert_eq!(embedded.shell, HostRole::Unsupported);
        assert!(embedded.validate().is_ok());
    }

    #[test]
    fn platform_catalog_rejects_duplicates_and_control_characters() {
        let duplicate = r#"{"kind":"applications","applications":[{"id":"org.example/.Main","title":"Editor"},{"id":"org.example/.Main","title":"Other"}]}"#;
        assert!(response(duplicate).is_err());
        assert!(response(r#"{"kind":"applications","applications":[{"id":"org.example/.Main","title":"bad\nlabel"}]}"#).is_err());
        assert!(matches!(
            response(
                r#"{"kind":"applications","applications":[{"id":"org.example/.Main","title":"Editor"}]}"#
            ),
            Ok(Some(PlatformResponse::Applications(_)))
        ));
    }

    #[test]
    fn location_bounds_and_operation_states_are_distinct() {
        assert!(response(r#"{"kind":"pending"}"#).unwrap().is_none());
        assert!(response(r#"{"kind":"location","latitude":91,"longitude":0}"#).is_err());
        assert!(response(r#"{"kind":"location","latitude":0,"longitude":181}"#).is_err());
        assert!(response(r#"{"kind":"error","message":"denied"}"#).is_err());
        assert!(matches!(
            response(r#"{"kind":"location","latitude":50.09,"longitude":18.22}"#),
            Ok(Some(PlatformResponse::Location { .. }))
        ));
    }

    #[test]
    fn unavailable_observations_do_not_become_grants_or_battery_values() {
        let observation: Observation = serde_json::from_str("{}").unwrap();
        let environment = observation.environment();
        assert_eq!(environment.notification_permission, HostPermission::Unknown);
        assert_eq!(environment.battery_percent, None);
        assert_eq!(environment.online, None);
        assert_eq!(environment.media_session_supported, None);
        assert!(response(&" ".repeat(MAX_RESPONSE_BYTES + 1)).is_err());
    }
}
