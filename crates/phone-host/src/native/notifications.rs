//! Device banners. Call blocking operations on a host worker, never the UI thread.
#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "macos")]
mod macos;
#[cfg(windows)]
mod windows;

use crate::HostPermission;

pub fn permission(request: bool) -> Result<HostPermission, String> {
    #[cfg(target_os = "linux")]
    {
        let _ = request;
        linux::permission()
    }
    #[cfg(target_os = "macos")]
    {
        macos::permission(request)
    }
    #[cfg(windows)]
    {
        let _ = request;
        windows::permission()
    }
    #[cfg(not(any(target_os = "linux", target_os = "macos", windows)))]
    {
        let _ = request;
        Ok(HostPermission::Unsupported)
    }
}

/// Success means that the host accepted the request. Focus Assist, Do Not
/// Disturb and desktop policy may still suppress the visible banner.
pub fn show(window_handle: Option<isize>, title: &str, body: &str) -> Result<(), String> {
    validate(title, body)?;
    #[cfg(target_os = "linux")]
    {
        let _ = window_handle;
        linux::show(title, body)
    }
    #[cfg(target_os = "macos")]
    {
        let _ = window_handle;
        macos::show(title, body)
    }
    #[cfg(windows)]
    {
        windows::show(
            window_handle.ok_or("The native window is unavailable.")?,
            title,
            body,
        )
    }
    #[cfg(not(any(target_os = "linux", target_os = "macos", windows)))]
    {
        let _ = (window_handle, title, body);
        Err("Device notifications are unavailable on this host.".into())
    }
}

pub fn close(window_handle: Option<isize>) {
    #[cfg(windows)]
    if let Some(handle) = window_handle {
        windows::close(handle);
    }
    #[cfg(not(windows))]
    let _ = window_handle;
}

fn validate(title: &str, body: &str) -> Result<(), String> {
    if title.len() > 1024
        || body.len() > 16 * 1024
        || title
            .chars()
            .chain(body.chars())
            .any(|c| c.is_control() && c != '\n' && c != '\t')
    {
        return Err("Notification text is too long or contains invalid control characters.".into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn notification_text_has_explicit_limits_and_no_embedded_nul() {
        assert!(validate("Alarm", "Meeting\nRoom 1").is_ok());
        assert!(validate("Alarm\0hidden", "").is_err());
        assert!(validate(&"a".repeat(1025), "").is_err());
    }
}
