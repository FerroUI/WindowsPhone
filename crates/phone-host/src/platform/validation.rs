use super::*;
use std::collections::BTreeSet;
pub const MAX_DEVICE_APPLICATIONS: usize = 4096;
fn text(value: &str, limit: usize, empty: bool) -> bool {
    value.len() <= limit
        && (empty || !value.trim().is_empty())
        && !value.chars().any(char::is_control)
}
pub fn validate_applications(apps: &[DeviceApplication]) -> Result<(), String> {
    if apps.len() > MAX_DEVICE_APPLICATIONS {
        return Err("The device application catalog exceeds its limit.".into());
    }
    let mut ids = BTreeSet::new();
    for app in apps {
        if !text(&app.id, 1024, false) || !text(&app.title, 512, false) || !ids.insert(&app.id) {
            return Err(
                "The device application catalog contains an invalid or duplicate entry.".into(),
            );
        }
    }
    Ok(())
}
pub(super) fn request(request: &PlatformRequest) -> Result<(), String> {
    match request {
        PlatformRequest::Device { request } => request.validate(),
        PlatformRequest::LaunchApplication { id } if !text(id, 1024, false) => {
            Err("Invalid device application identity.".into())
        }
        PlatformRequest::ShareText {
            title,
            text: body,
            url,
        } if !text(title, 512, true)
            || body.len() > 65536
            || body.contains('\0')
            || !text(url, 4096, true) =>
        {
            Err("Shared content exceeds the platform contract.".into())
        }
        PlatformRequest::Notify { title, body }
            if !text(title, 512, false) || body.len() > 4096 || body.contains('\0') =>
        {
            Err("Invalid notification content.".into())
        }
        _ => Ok(()),
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn catalog_identity_is_bounded_and_unambiguous() {
        let app = DeviceApplication {
            id: "package/activity".into(),
            title: "Editor".into(),
        };
        assert!(validate_applications(&[app.clone()]).is_ok());
        assert!(validate_applications(&[app.clone(), app]).is_err());
        assert!(validate_applications(&[DeviceApplication {
            id: "bad\nidentity".into(),
            title: "Title".into()
        }])
        .is_err());
        assert!(PlatformRequest::LaunchApplication {
            id: "x".repeat(1025)
        }
        .validate()
        .is_err());
    }
    #[test]
    fn fullscreen_does_not_grant_a_shell_role() {
        let mut state = PlatformState {
            platform: HostPlatform::Linux,
            mode: HostMode::Shell,
            features: vec![HostFeature::ShellSession],
            shell: HostRole::Available,
            ..Default::default()
        };
        assert!(state.validate().is_err());
        state.shell = HostRole::Active;
        assert!(state.validate().is_ok());
        state.platform = HostPlatform::Ios;
        assert!(state.validate().is_err());
        state.platform = HostPlatform::Browser;
        assert!(state.validate().is_err());
        for platform in [HostPlatform::Browser, HostPlatform::Ios] {
            for feature in [
                HostFeature::InstalledApps,
                HostFeature::LauncherRole,
                HostFeature::ShellSession,
            ] {
                let invalid = PlatformState {
                    platform,
                    features: vec![feature],
                    ..Default::default()
                };
                assert!(invalid.validate().is_err());
            }
        }
    }
}
