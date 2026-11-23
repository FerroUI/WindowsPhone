//! Browser platform futures retain their completion identity, not a view model.
//! The JS operation starts synchronously in dispatch to preserve user activation.
use phone_host::{
    platform::{
        ready, HostLifecycle, HostMode, PlatformPlugin, PlatformRequest, PlatformResponse,
        PlatformState, PlatformTask,
    },
    HostEnvironment, HostPlatform,
};
use std::{
    cell::{Cell, RefCell},
    collections::{BTreeMap, BTreeSet},
    future::Future,
    pin::Pin,
    rc::{Rc, Weak},
    task::{Context, Poll, Waker},
};
use wasm_bindgen::prelude::*;

#[wasm_bindgen(raw_module = "./platform-bridge.js")]
extern "C" {
    #[wasm_bindgen(catch, js_name = platformMode)]
    fn browser_mode() -> Result<String, JsValue>;
    #[wasm_bindgen(catch, js_name = platformState)]
    fn browser_state(mode: &str) -> Result<String, JsValue>;
    #[wasm_bindgen(catch, js_name = platformEnvironment)]
    fn browser_environment() -> Result<String, JsValue>;
    #[wasm_bindgen(catch, js_name = startPlatformTask)]
    fn start(id: u32, mode: &str, request: &str) -> Result<(), JsValue>;
    #[wasm_bindgen(catch, js_name = cancelPlatformTask)]
    fn cancel(id: u32) -> Result<(), JsValue>;
    #[wasm_bindgen(catch, js_name = suspendPlatform)]
    fn suspend() -> Result<(), JsValue>;
    #[wasm_bindgen(catch, js_name = closePlatform)]
    fn close() -> Result<(), JsValue>;
}

struct Completion {
    result: Option<Result<PlatformResponse, String>>,
    waker: Option<Waker>,
}
#[derive(Default)]
struct Session {
    closed: Cell<bool>,
    pending: RefCell<BTreeSet<u32>>,
}

ferroui_base::ferro_thread_local! {
    static NEXT: Cell<u32> = const { Cell::new(0) };
    static TASKS: RefCell<BTreeMap<u32,Rc<RefCell<Completion>>>> = const { RefCell::new(BTreeMap::new()) };
}

pub struct BrowserPlatform {
    mode: HostMode,
    observed: RefCell<PlatformState>,
    session: Rc<Session>,
}
impl BrowserPlatform {
    pub fn new() -> Result<Self, String> {
        let mode = HostMode::parse(&browser_mode().map_err(js_error)?)?;
        let observed = read_state(mode)?;
        Ok(Self {
            mode,
            observed: RefCell::new(observed),
            session: Rc::default(),
        })
    }
    fn stop(&self) {
        if self.session.closed.replace(true) {
            return;
        }
        let pending = std::mem::take(&mut *self.session.pending.borrow_mut());
        for id in pending {
            complete(id, Err("The browser host has closed.".into()));
            TASKS.with(|tasks| tasks.borrow_mut().remove(&id));
            let _ = cancel(id);
        }
        let _ = close();
    }
}
impl Drop for BrowserPlatform {
    fn drop(&mut self) {
        self.stop();
    }
}

impl PlatformPlugin for BrowserPlatform {
    fn id(&self) -> &'static str {
        "ferroui.browser"
    }
    fn state(&self) -> PlatformState {
        if !self.session.closed.get() {
            if let Ok(state) = read_state(self.mode) {
                *self.observed.borrow_mut() = state;
            }
        }
        self.observed.borrow().clone()
    }
    fn environment(&self) -> HostEnvironment {
        browser_environment()
            .ok()
            .filter(|json| json.len() <= 8192)
            .and_then(|json| serde_json::from_str(&json).ok())
            .unwrap_or(HostEnvironment {
                platform: HostPlatform::Browser,
                ..Default::default()
            })
    }
    fn dispatch(&self, request: PlatformRequest) -> PlatformTask {
        if self.session.closed.get() {
            return ready(Err("The browser host has closed.".into()));
        }
        if let Err(error) = request.validate() {
            return ready(Err(error));
        }
        if request
            .feature()
            .is_some_and(|feature| !self.state().supports(feature))
        {
            return ready(Err(
                "This service is unavailable in the current browser context.".into(),
            ));
        }
        if TASKS.with(|tasks| tasks.borrow().len() >= 8) {
            return ready(Err(
                "Too many browser platform requests are in progress.".into()
            ));
        }
        let Some(id) = NEXT.with(|next| next.get().checked_add(1).inspect(|id| next.set(*id)))
        else {
            return ready(Err(
                "Browser platform request identities are exhausted. Reload the app.".into(),
            ));
        };
        let json = match serde_json::to_string(&request) {
            Ok(json) => json,
            Err(error) => return ready(Err(error.to_string())),
        };
        let completion = Rc::new(RefCell::new(Completion {
            result: None,
            waker: None,
        }));
        self.session.pending.borrow_mut().insert(id);
        TASKS.with(|tasks| tasks.borrow_mut().insert(id, completion.clone()));
        let task = BrowserTask {
            id,
            completion,
            session: Rc::downgrade(&self.session),
        };
        // Start before handing the future to Dispatcher: share/permission APIs
        // need the original input stack's transient browser activation.
        if let Err(error) = start(id, mode_name(self.mode), &json) {
            complete(id, Err(js_error(error)));
        }
        Box::pin(task)
    }
    fn lifecycle(&self, event: HostLifecycle) {
        match event {
            HostLifecycle::Suspended => {
                let _ = suspend();
            }
            HostLifecycle::Closing => self.stop(),
            HostLifecycle::Resumed => {}
        }
    }
}

struct BrowserTask {
    id: u32,
    completion: Rc<RefCell<Completion>>,
    session: Weak<Session>,
}
impl Future for BrowserTask {
    type Output = Result<PlatformResponse, String>;
    fn poll(self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<Self::Output> {
        if self
            .session
            .upgrade()
            .is_none_or(|session| session.closed.get())
        {
            return Poll::Ready(Err("The browser platform operation was cancelled.".into()));
        }
        let mut completion = self.completion.borrow_mut();
        if let Some(result) = completion.result.take() {
            return Poll::Ready(result);
        }
        completion.waker = Some(context.waker().clone());
        Poll::Pending
    }
}
impl Drop for BrowserTask {
    fn drop(&mut self) {
        TASKS.with(|tasks| tasks.borrow_mut().remove(&self.id));
        if let Some(session) = self.session.upgrade() {
            session.pending.borrow_mut().remove(&self.id);
        }
        let _ = cancel(self.id);
    }
}
fn complete(id: u32, result: Result<PlatformResponse, String>) -> bool {
    let Some(completion) = TASKS.with(|tasks| tasks.borrow().get(&id).cloned()) else {
        return false;
    };
    let waker = {
        let mut completion = completion.borrow_mut();
        if completion.result.is_some() {
            return false;
        }
        completion.result = Some(result);
        completion.waker.take()
    };
    if let Some(waker) = waker {
        waker.wake();
    }
    true
}
#[wasm_bindgen(js_name = completePlatformTask)]
pub fn complete_platform_task(id: u32, json: &str) -> bool {
    if json.len() > 8192 {
        return complete(
            id,
            Err("The browser platform response exceeded its limit.".into()),
        );
    }
    complete(
        id,
        serde_json::from_str(json)
            .map_err(|error| format!("Invalid browser platform response: {error}")),
    )
}
#[wasm_bindgen(js_name = failPlatformTask)]
pub fn fail_platform_task(id: u32, error: &str) -> bool {
    complete(id, Err(error.chars().take(2048).collect()))
}

fn mode_name(mode: HostMode) -> &'static str {
    match mode {
        HostMode::App => "app",
        HostMode::Embedded => "embedded",
        HostMode::Launcher => "launcher",
        HostMode::Shell => "shell",
    }
}
fn read_state(mode: HostMode) -> Result<PlatformState, String> {
    let json = browser_state(mode_name(mode)).map_err(js_error)?;
    if json.len() > 8192 {
        return Err("Invalid browser platform description.".into());
    }
    let state: PlatformState = serde_json::from_str(&json).map_err(|error| error.to_string())?;
    if state.platform != HostPlatform::Browser || state.mode != mode {
        return Err("The browser platform description does not match its host.".into());
    }
    state.validate()?;
    Ok(state)
}
fn js_error(error: JsValue) -> String {
    error
        .as_string()
        .unwrap_or_else(|| format!("Browser integration failed: {error:?}"))
}
