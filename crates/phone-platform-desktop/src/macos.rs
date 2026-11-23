use super::{application, Application, HostFeature, HostMode, HostRole, PlatformState};
use phone_host::platform::{ready, PlatformResponse, PlatformTask, SystemSettingsPage};
use std::{
    ffi::{c_char, c_void, CStr, CString},
    future::Future,
    pin::Pin,
    sync::{
        atomic::{AtomicBool, AtomicUsize, Ordering},
        Arc, Mutex,
    },
    task::{Context, Poll, Waker},
};

extern "C" {
    fn phone_desktop_free(value: *mut c_void);
    fn phone_desktop_catalog(error: *mut *mut c_char) -> *mut c_char;
    fn phone_desktop_launch(
        path: *const c_char,
        context: *mut c_void,
        completion: unsafe extern "C" fn(*mut c_void, i32, *const c_char),
    );
    fn phone_desktop_open_settings() -> i32;
}
struct NativeString(*mut c_char);
impl Drop for NativeString {
    fn drop(&mut self) {
        if !self.0.is_null() {
            unsafe {
                phone_desktop_free(self.0.cast());
            }
        }
    }
}

pub(super) fn applications() -> Result<Vec<Application>, String> {
    let mut error = std::ptr::null_mut();
    let output = NativeString(unsafe { phone_desktop_catalog(&mut error) });
    let error = NativeString(error);
    if output.0.is_null() {
        return Err(if error.0.is_null() {
            "The macOS application catalog is unavailable.".into()
        } else {
            unsafe { CStr::from_ptr(error.0) }
                .to_string_lossy()
                .into_owned()
        });
    }
    let entries: serde_json::Value =
        serde_json::from_slice(unsafe { CStr::from_ptr(output.0) }.to_bytes())
            .map_err(|error| error.to_string())?;
    let entries = entries
        .as_array()
        .ok_or("The native application catalog has an invalid shape.")?;
    Ok(entries
        .iter()
        .filter_map(|entry| {
            let identity = entry.get("identity")?.as_str()?;
            application("macos", identity, entry.get("title")?.as_str()?, identity)
        })
        .collect())
}

struct Completion {
    result: Mutex<(Option<Result<PlatformResponse, String>>, Option<Waker>)>,
    cancelled: AtomicBool,
}
struct LaunchTask(Arc<Completion>);
impl Future for LaunchTask {
    type Output = Result<PlatformResponse, String>;
    fn poll(self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<Self::Output> {
        let mut state = self
            .0
            .result
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        if let Some(result) = state.0.take() {
            return Poll::Ready(result);
        }
        state.1 = Some(context.waker().clone());
        Poll::Pending
    }
}
impl Drop for LaunchTask {
    fn drop(&mut self) {
        self.0.cancelled.store(true, Ordering::Release);
    }
}
static PENDING: AtomicUsize = AtomicUsize::new(0);

unsafe extern "C" fn completed(context: *mut c_void, accepted: i32, error: *const c_char) {
    // NSWorkspace invokes completion exactly once. It owns this box until then,
    // even when the Rust future was cancelled or the surface was disposed.
    let completion = unsafe { Box::from_raw(context.cast::<Arc<Completion>>()) };
    PENDING.fetch_sub(1, Ordering::AcqRel);
    if completion.cancelled.load(Ordering::Acquire) {
        return;
    }
    let result = if accepted != 0 {
        Ok(PlatformResponse::Accepted {
            message: "macOS activated the application.".into(),
        })
    } else {
        Err(if error.is_null() {
            "macOS could not activate the application.".into()
        } else {
            unsafe { CStr::from_ptr(error) }
                .to_string_lossy()
                .into_owned()
        })
    };
    let waker = {
        let mut state = completion
            .result
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        state.0 = Some(result);
        state.1.take()
    };
    if let Some(waker) = waker {
        waker.wake();
    }
}

pub(super) fn launch(target: String) -> PlatformTask {
    let target = match CString::new(target) {
        Ok(target) => target,
        Err(_) => return ready(Err("Invalid application identity.".into())),
    };
    if PENDING
        .try_update(Ordering::AcqRel, Ordering::Acquire, |value| {
            (value < 4).then_some(value + 1)
        })
        .is_err()
    {
        return ready(Err(
            "Application activation is busy. Try again shortly.".into()
        ));
    }
    let completion = Arc::new(Completion {
        result: Mutex::new((None, None)),
        cancelled: AtomicBool::new(false),
    });
    let context = Box::into_raw(Box::new(completion.clone())).cast();
    unsafe {
        phone_desktop_launch(target.as_ptr(), context, completed);
    }
    Box::pin(LaunchTask(completion))
}

pub(super) fn open_settings(page: SystemSettingsPage) -> PlatformTask {
    let section = match page {
        SystemSettingsPage::Application => "Applications",
        SystemSettingsPage::Network => "Network",
        SystemSettingsPage::Notifications => "Notifications",
        SystemSettingsPage::DefaultApps => "the relevant default-app setting",
        _ => "the requested device category",
    };
    ready(if unsafe { phone_desktop_open_settings() } != 0 {
        Ok(PlatformResponse::Accepted { message: format!("Opened System Settings. Choose {section}; macOS does not expose a stable public deep link for this page.") })
    } else {
        Err("macOS could not open System Settings.".into())
    })
}

pub(super) fn state(mode: HostMode) -> PlatformState {
    PlatformState { platform: phone_host::HostPlatform::Macos, mode,
        launcher: HostRole::Unsupported, shell: HostRole::Unsupported,
        features: vec![HostFeature::InstalledApps, HostFeature::SystemSettings],
        detail: "NSWorkspace activates user-facing applications discovered in the system, local and user Applications directories. macOS supports an ordinary application or launcher window; this adapter does not replace Finder, Dock, WindowServer or the login session.".into() }
}
