use crate::HostPermission;
use std::ffi::{c_char, c_int, CStr, CString};

unsafe extern "C" {
    fn wp_notification_permission(request: c_int, error: *mut c_char, length: usize) -> c_int;
    fn wp_notification_show(
        title: *const c_char,
        body: *const c_char,
        error: *mut c_char,
        length: usize,
    ) -> c_int;
}

fn failure(error: &[c_char]) -> String {
    // The native boundary always zero-terminates, including its error paths.
    let message = unsafe {
        CStr::from_ptr(error.as_ptr())
            .to_string_lossy()
            .into_owned()
    };
    if message.is_empty() {
        "The macOS notification provider did not return a usable result.".into()
    } else {
        message
    }
}

pub(super) fn permission(request: bool) -> Result<HostPermission, String> {
    let mut error = [0; 512];
    match unsafe { wp_notification_permission(i32::from(request), error.as_mut_ptr(), error.len()) }
    {
        0 => Ok(HostPermission::Unsupported),
        1 => Ok(HostPermission::Prompt),
        2 => Ok(HostPermission::Granted),
        3 => Ok(HostPermission::Denied),
        _ => Err(failure(&error)),
    }
}

pub(super) fn show(title: &str, body: &str) -> Result<(), String> {
    let title = CString::new(title).map_err(|e| e.to_string())?;
    let body = CString::new(body).map_err(|e| e.to_string())?;
    let mut error = [0; 512];
    let result = unsafe {
        wp_notification_show(
            title.as_ptr(),
            body.as_ptr(),
            error.as_mut_ptr(),
            error.len(),
        )
    };
    if result == 1 {
        Ok(())
    } else {
        Err(failure(&error))
    }
}
