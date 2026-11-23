//! Default-handler launch with no shell interpolation. Callers validate URLs or
//! supply an app-owned path. Success means the launch request was accepted.
use std::ffi::OsStr;

#[cfg(target_os = "windows")]
pub fn open_external(target: &OsStr) -> Result<(), String> {
    use std::os::windows::ffi::OsStrExt;
    #[link(name = "shell32")]
    extern "system" {
        fn ShellExecuteW(
            window: *mut std::ffi::c_void,
            operation: *const u16,
            file: *const u16,
            parameters: *const u16,
            directory: *const u16,
            show: i32,
        ) -> isize;
    }
    let mut target: Vec<_> = target.encode_wide().collect();
    if target.contains(&0) {
        return Err("The launch target contains an invalid character.".into());
    }
    target.push(0);
    let verb: Vec<_> = "open\0".encode_utf16().collect();
    // UTF-16 buffers remain valid for the synchronous ShellExecuteW call.
    let result = unsafe {
        ShellExecuteW(
            std::ptr::null_mut(),
            verb.as_ptr(),
            target.as_ptr(),
            std::ptr::null(),
            std::ptr::null(),
            1,
        )
    };
    if result <= 32 {
        Err(format!(
            "No application accepted this request (Windows code {result})."
        ))
    } else {
        Ok(())
    }
}

#[cfg(not(target_os = "windows"))]
pub fn open_external(target: &OsStr) -> Result<(), String> {
    use std::process::{Command, Stdio};
    #[cfg(target_os = "macos")]
    let program = "open";
    #[cfg(not(target_os = "macos"))]
    let program = "xdg-open";
    let mut child = Command::new(program)
        .arg(target)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|error| format!("No application could open this request: {error}"))?;
    // A default media handler can remain alive until its player closes. Reap it
    // outside the UI and file-I/O queue so thumbnails/imports never wait for it.
    std::thread::spawn(move || {
        let _ = child.wait();
    });
    Ok(())
}
