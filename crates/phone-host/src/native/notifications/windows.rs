use windows::Win32::{
    Foundation::HWND,
    UI::{
        Shell::*,
        WindowsAndMessaging::{LoadIconW, IDI_INFORMATION},
    },
};

const ICON_ID: u32 = 0x5750;

pub(super) fn permission() -> Result<crate::HostPermission, String> {
    // Win32 balloons have no per-app authorization prompt. Report capability
    // only after querying the real shell's current notification availability.
    let state = unsafe { SHQueryUserNotificationState() }.map_err(|error| error.to_string())?;
    Ok(if state == QUNS_ACCEPTS_NOTIFICATIONS {
        crate::HostPermission::Granted
    } else {
        crate::HostPermission::Unknown
    })
}

fn wide<const N: usize>(value: &str) -> [u16; N] {
    let mut result = [0; N];
    let mut position = 0;
    for c in value.chars() {
        let mut bytes = [0; 2];
        let encoded = c.encode_utf16(&mut bytes);
        if position + encoded.len() >= N {
            break;
        }
        result[position..position + encoded.len()].copy_from_slice(encoded);
        position += encoded.len();
    }
    result
}

fn data(handle: isize) -> NOTIFYICONDATAW {
    NOTIFYICONDATAW {
        cbSize: std::mem::size_of::<NOTIFYICONDATAW>() as u32,
        hWnd: HWND(handle as *mut _),
        uID: ICON_ID,
        ..Default::default()
    }
}

pub(super) fn show(handle: isize, title: &str, body: &str) -> Result<(), String> {
    if handle == 0 {
        return Err("The native phone window is unavailable.".into());
    }
    let mut info = data(handle);
    info.uFlags = NIF_INFO | NIF_TIP | NIF_ICON;
    info.szInfoTitle = wide(title);
    info.szInfo = wide(body);
    info.szTip = wide("Windows Phone");
    info.dwInfoFlags = NIIF_INFO | NIIF_NOSOUND | NIIF_RESPECT_QUIET_TIME;
    // The shared system icon is borrowed; it must not be destroyed.
    unsafe {
        info.hIcon = LoadIconW(None, IDI_INFORMATION).map_err(|e| e.to_string())?;
        if !Shell_NotifyIconW(NIM_MODIFY, &info).as_bool()
            && !Shell_NotifyIconW(NIM_ADD, &info).as_bool()
        {
            return Err("Windows did not accept the notification-area banner.".into());
        }
    }
    Ok(())
}

pub(super) fn close(handle: isize) {
    if handle != 0 {
        unsafe {
            let _ = Shell_NotifyIconW(NIM_DELETE, &data(handle));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn truncation_keeps_unicode_scalar_boundaries() {
        assert_eq!(wide::<4>("a😀b"), [97, 0xd83d, 0xde00, 0]);
        assert_eq!(wide::<3>("a😀"), [97, 0, 0]);
    }
}
