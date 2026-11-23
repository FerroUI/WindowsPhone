#[cfg(target_os = "ios")]
use crate::pending::{Kind, Pending, Reply, Session};
use phone_host::HostPermission;
use std::sync::atomic::{AtomicI32, Ordering};
#[cfg(target_os = "ios")]
use std::{
    ffi::{CStr, CString, c_char},
    rc::Rc,
};

static PERMISSION: AtomicI32 = AtomicI32::new(0);

pub fn notification_permission() -> HostPermission {
    match PERMISSION.load(Ordering::Relaxed) {
        1 => HostPermission::Prompt,
        2 => HostPermission::Granted,
        3 => HostPermission::Denied,
        _ => HostPermission::Unknown,
    }
}

#[cfg(target_os = "ios")]
type Callback = unsafe extern "C" fn(u64, i32, f64, f64, *const c_char);

#[cfg(target_os = "ios")]
unsafe extern "C" {
    fn wp_ios_cancel(id: u64);
    fn wp_ios_settings(id: u64, completion: Callback, page: i32) -> i32;
    fn wp_ios_haptic(kind: i32) -> i32;
    fn wp_ios_observe(battery: *mut f64, charging: *mut i32, power_saver: *mut i32);
    fn wp_ios_monitor_power(enabled: i32);
    fn wp_ios_data_path(buffer: *mut c_char, length: usize) -> i32;
    fn wp_ios_share(id: u64, completion: Callback, text: *const c_char, url: *const c_char) -> i32;
    fn wp_ios_share_file(id: u64, completion: Callback, path: *const c_char) -> i32;
    fn wp_ios_notification_permission(id: u64, completion: Callback, request: i32) -> i32;
    fn wp_ios_notify(
        id: u64,
        completion: Callback,
        title: *const c_char,
        body: *const c_char,
    ) -> i32;
    fn wp_ios_locate(id: u64, completion: Callback) -> i32;
    fn wp_ios_device(id: u64, completion: Callback, command: *const c_char) -> i32;
    fn wp_ios_device_monitor(enabled: i32);
    fn wp_ios_device_shutdown();
    fn wp_ios_device_poll(buffer: *mut c_char, capacity: usize) -> usize;
    fn wp_ios_set_notification_policy(policy: u32);
}

#[cfg(target_os = "ios")]
unsafe extern "C" fn completed(id: u64, code: i32, first: f64, second: f64, text: *const c_char) {
    // Every native bridge call invokes this synchronously with a valid NUL-terminated
    // NSString UTF-8 view. The callback retains no pointer after returning.
    let message = if text.is_null() {
        String::new()
    } else {
        let bytes = unsafe { CStr::from_ptr(text) }.to_bytes();
        let maximum = if code == 4 {
            crate::device::MAX_REPLY_BYTES
        } else {
            4096
        };
        if bytes.len() > maximum {
            crate::pending::complete(
                id,
                Err("The iOS provider response exceeds its data limit.".into()),
            );
            return;
        }
        String::from_utf8_lossy(bytes).into_owned()
    };
    let result = if code == 0 {
        Err(message)
    } else {
        Ok(Reply {
            code,
            first,
            second,
            message,
        })
    };
    crate::pending::complete_with(id, result, || {
        if code == 2 {
            PERMISSION.store(first as i32, Ordering::Relaxed);
        }
    });
}

pub fn cancel(id: u64) {
    #[cfg(target_os = "ios")]
    unsafe {
        wp_ios_cancel(id);
    }
    #[cfg(not(target_os = "ios"))]
    let _ = id;
}

#[cfg(target_os = "ios")]
fn started(task: Pending, admitted: i32) -> Pending {
    if admitted != 1 {
        crate::pending::complete(
            task.id,
            Err("The iOS operation is unavailable or its queue is full.".into()),
        );
    }
    task
}

#[cfg(target_os = "ios")]
pub fn settings(session: &Rc<Session>, page: i32) -> Result<Pending, String> {
    let task = session.start(Kind::Settings)?;
    let result = unsafe { wp_ios_settings(task.id, completed, page) };
    Ok(started(task, result))
}

#[cfg(target_os = "ios")]
pub fn share(session: &Rc<Session>, text: &str, url: &str) -> Result<Pending, String> {
    let text = CString::new(text).map_err(|_| "Share text contains a NUL character.")?;
    let url = CString::new(url).map_err(|_| "Share URL contains a NUL character.")?;
    let task = session.start(Kind::Share)?;
    let result = unsafe { wp_ios_share(task.id, completed, text.as_ptr(), url.as_ptr()) };
    Ok(started(task, result))
}

#[cfg(target_os = "ios")]
pub fn share_file(session: &Rc<Session>, path: &std::path::Path) -> Result<Pending, String> {
    let path = path
        .to_str()
        .filter(|path| path.len() <= 4096)
        .ok_or("The media path is invalid or too long.")?;
    let path = CString::new(path).map_err(|_| "The media path contains a NUL character.")?;
    let task = session.start(Kind::Share)?;
    let result = unsafe { wp_ios_share_file(task.id, completed, path.as_ptr()) };
    Ok(started(task, result))
}

#[cfg(target_os = "ios")]
pub fn permission(session: &Rc<Session>, request: bool) -> Result<Pending, String> {
    let task = session.start(Kind::Permission)?;
    let result = unsafe { wp_ios_notification_permission(task.id, completed, i32::from(request)) };
    Ok(started(task, result))
}

#[cfg(target_os = "ios")]
pub fn notify(session: &Rc<Session>, title: &str, body: &str) -> Result<Pending, String> {
    let title = CString::new(title).map_err(|_| "Notification title contains a NUL character.")?;
    let body = CString::new(body).map_err(|_| "Notification body contains a NUL character.")?;
    let task = session.start(Kind::Notify)?;
    let result = unsafe { wp_ios_notify(task.id, completed, title.as_ptr(), body.as_ptr()) };
    Ok(started(task, result))
}

#[cfg(target_os = "ios")]
pub fn locate(session: &Rc<Session>) -> Result<Pending, String> {
    let task = session.start(Kind::Location)?;
    let result = unsafe { wp_ios_locate(task.id, completed) };
    Ok(started(task, result))
}

#[cfg(target_os = "ios")]
pub fn device(session: &Rc<Session>, command: &str, observation: bool) -> Result<Pending, String> {
    let command =
        CString::new(command).map_err(|_| "The device command contains a NUL character.")?;
    let task = session.start(if observation {
        Kind::DeviceRead
    } else {
        Kind::Device
    })?;
    let result = unsafe { wp_ios_device(task.id, completed, command.as_ptr()) };
    Ok(started(task, result))
}

pub fn monitor_device(enabled: bool) {
    #[cfg(target_os = "ios")]
    unsafe {
        wp_ios_device_monitor(i32::from(enabled));
    }
    #[cfg(not(target_os = "ios"))]
    let _ = enabled;
}

pub fn close_device() {
    #[cfg(target_os = "ios")]
    unsafe {
        wp_ios_device_shutdown();
    }
}

pub fn set_notification_policy(policy: &phone_host::platform::DeviceNotificationPolicy) {
    #[cfg(target_os = "ios")]
    unsafe {
        wp_ios_set_notification_policy(
            u32::from(policy.enabled)
                | (u32::from(policy.message_center) << 1)
                | (u32::from(policy.message_banners) << 2)
                | (u32::from(policy.message_sound) << 3)
                | (u32::from(policy.message_vibration) << 4)
                | (u32::from(policy.lock_screen) << 5),
        );
    }
    #[cfg(not(target_os = "ios"))]
    let _ = policy;
}

#[cfg(target_os = "ios")]
pub fn device_events() -> Vec<phone_host::platform::PlatformResponse> {
    // A native event is consumed only after its entire bounded payload fits.
    let mut events = Vec::new();
    for _ in 0..16 {
        let length = unsafe { wp_ios_device_poll(std::ptr::null_mut(), 0) };
        if length == 0 || length > crate::device::MAX_REPLY_BYTES {
            break;
        }
        let mut buffer = vec![0_u8; length + 1];
        let copied = unsafe { wp_ios_device_poll(buffer.as_mut_ptr().cast(), buffer.len()) };
        if copied == 0 || copied > length {
            break;
        }
        if let Ok(text) = std::str::from_utf8(&buffer[..copied]) {
            if let Ok(response) = crate::device::decode(text) {
                events.push(response);
            }
        }
    }
    events
}

#[cfg(target_os = "ios")]
pub fn haptic(kind: i32) -> bool {
    unsafe { wp_ios_haptic(kind) == 1 }
}

pub fn monitor_power(enabled: bool) {
    #[cfg(target_os = "ios")]
    unsafe {
        wp_ios_monitor_power(i32::from(enabled));
    }
    #[cfg(not(target_os = "ios"))]
    let _ = enabled;
}

pub fn power() -> (Option<u8>, Option<bool>, Option<bool>) {
    #[cfg(target_os = "ios")]
    {
        let (mut battery, mut charging, mut saver) = (-1.0_f64, -1, -1);
        unsafe {
            wp_ios_observe(&mut battery, &mut charging, &mut saver);
        }
        (
            (battery.is_finite() && (0.0..=1.0).contains(&battery))
                .then(|| (battery * 100.0).round() as u8),
            (charging >= 0).then_some(charging == 1),
            (saver >= 0).then_some(saver == 1),
        )
    }
    #[cfg(not(target_os = "ios"))]
    {
        (None, None, None)
    }
}

pub fn data_path() -> Result<std::path::PathBuf, String> {
    #[cfg(target_os = "ios")]
    {
        let mut buffer = [0; 4096];
        if unsafe { wp_ios_data_path(buffer.as_mut_ptr(), buffer.len()) } != 1 {
            return Err("Could not create the app's iOS Application Support directory.".into());
        }
        let directory = unsafe { CStr::from_ptr(buffer.as_ptr()) }
            .to_str()
            .map_err(|_| "The app data directory is not UTF-8.")?;
        Ok(std::path::PathBuf::from(directory).join("phone.json"))
    }
    #[cfg(not(target_os = "ios"))]
    {
        Err("The iOS app container is unavailable on this platform.".into())
    }
}
