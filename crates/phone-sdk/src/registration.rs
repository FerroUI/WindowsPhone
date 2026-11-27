use crate::{AppContext, AppManifest, session::Session};
use ferroui_view::prelude::Element;
use phone_domain::{AppId, Route};
use phone_state::PhoneViewModel;
use std::rc::Rc;

pub type AppView = fn(Rc<PhoneViewModel>, Rc<Session>) -> Element;
pub type BackHandler = fn(&Rc<PhoneViewModel>, &Rc<Session>) -> bool;
pub type RouteHandler = fn(&Rc<PhoneViewModel>, &Rc<Session>, &Route, &Route);
pub type SessionHandler = fn(&Rc<Session>);
pub type SwipeHandler = fn(&Rc<PhoneViewModel>, &Rc<Session>, f64) -> bool;
pub type ActivationHandler =
    fn(&Rc<PhoneViewModel>, &Rc<Session>, serde_json::Value) -> Result<(), String>;
pub type LaunchPathHandler = fn(&str) -> Result<serde_json::Value, String>;
pub type TickHandler = fn(&Rc<PhoneViewModel>, &Rc<Session>, i64);
/// Return this application's complete current foreground-reminder schedule.
/// Providers read their own validated data without changing state or using hosts.
pub type ReminderProvider = fn(&PhoneViewModel) -> Result<Vec<crate::AppReminder>, String>;
pub type MediaEventHandler = fn(AppContext, &crate::media::PlaybackEvent);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AppLifecycle {
    Activated,
    Deactivated,
    Suspended,
    Resumed,
    Closing,
    Reset,
}

#[derive(Clone)]
pub struct AppRegistration {
    pub manifest: AppManifest,
    pub(crate) render: Rc<dyn Fn(AppContext) -> Element>,
    pub(crate) back: Option<BackHandler>,
    pub(crate) backspace_input: bool,
    pub(crate) route_changed: Option<RouteHandler>,
    pub(crate) reset: Option<SessionHandler>,
    pub(crate) deactivate: Option<SessionHandler>,
    pub(crate) swipe: Option<SwipeHandler>,
    pub(crate) can_swipe: Option<fn(&Session) -> bool>,
    pub(crate) pivot_count: Option<fn(&Session) -> usize>,
    pub(crate) route_pivot_count: Option<fn(&Route, &Session) -> usize>,
    pub(crate) overlay: Option<fn(&Session) -> bool>,
    pub(crate) host_navigation: Option<fn(&PhoneViewModel, &Session, &str)>,
    pub(crate) host_weather: Option<fn(&Session, String, f64, String)>,
    pub(crate) tick: Option<TickHandler>,
    pub(crate) reminders: Option<ReminderProvider>,
    pub(crate) activate: Option<ActivationHandler>,
    pub(crate) launch_path: Option<LaunchPathHandler>,
    pub(crate) tile: Option<fn(crate::AppTileContext) -> Element>,
    pub(crate) lifecycle: Option<fn(AppContext, AppLifecycle)>,
    pub(crate) media_event: Option<MediaEventHandler>,
}

impl AppRegistration {
    pub fn new(manifest: AppManifest, render: impl Fn(AppContext) -> Element + 'static) -> Self {
        Self {
            manifest,
            render: Rc::new(render),
            back: None,
            backspace_input: false,
            route_changed: None,
            reset: None,
            deactivate: None,
            swipe: None,
            can_swipe: None,
            pivot_count: None,
            route_pivot_count: None,
            overlay: None,
            host_navigation: None,
            host_weather: None,
            tick: None,
            reminders: None,
            lifecycle: None,
            media_event: None,
            activate: None,
            launch_path: None,
            tile: None,
        }
    }
    pub fn builtin(app: AppId, view: AppView) -> Self {
        Self::new(AppManifest::builtin(app), move |cx| view(cx.vm, cx.session))
    }
    pub fn with_activation(mut self, value: ActivationHandler) -> Self {
        self.activate = Some(value);
        self
    }
    /// Validate a built-in app's deep link before navigating, then deliver its
    /// typed payload through the existing bounded data-activation boundary.
    pub fn with_launch_path(mut self, value: LaunchPathHandler) -> Self {
        self.launch_path = Some(value);
        self
    }
    pub fn launch_payload(&self, path: &str) -> Result<Option<serde_json::Value>, String> {
        phone_domain::validate_app_path(path)?;
        if path.is_empty() || self.manifest.builtin.is_none() {
            return Ok(None);
        }
        let value = self
            .launch_path
            .ok_or("This app does not support this launch path.")?(path)?;
        phone_domain::validate_app_value(&value)?;
        if serde_json::to_vec(&value).map_err(|e| e.to_string())?.len() > 65536 {
            return Err("Launch data exceeds its limit.".into());
        }
        if self.activate.is_none() {
            return Err("The app has no data activation handler.".into());
        }
        Ok(Some(value))
    }
    pub fn with_tile(mut self, value: fn(crate::AppTileContext) -> Element) -> Self {
        self.tile = Some(value);
        self
    }
    pub fn with_back(mut self, value: BackHandler) -> Self {
        self.back = Some(value);
        self
    }
    /// Keep unmodified Backspace for a custom input surface. Hardware Back and
    /// Escape still use system navigation; TextBox input is already protected.
    pub fn with_backspace_input(mut self, value: bool) -> Self {
        self.backspace_input = value;
        self
    }
    pub fn with_route_changed(mut self, value: RouteHandler) -> Self {
        self.route_changed = Some(value);
        self
    }
    pub fn with_reset(mut self, value: SessionHandler) -> Self {
        self.reset = Some(value);
        self
    }
    pub fn with_deactivate(mut self, value: SessionHandler) -> Self {
        self.deactivate = Some(value);
        self
    }
    pub fn with_swipe(mut self, value: SwipeHandler) -> Self {
        self.swipe = Some(value);
        self
    }
    pub fn with_can_swipe(mut self, value: fn(&Session) -> bool) -> Self {
        self.can_swipe = Some(value);
        self
    }
    pub fn with_pivot_count(mut self, value: fn(&Session) -> usize) -> Self {
        self.pivot_count = Some(value);
        self
    }
    /// Declare the visible pivot count for the exact route being considered
    /// by the shell. Return zero on detail/editor pages that have no pivot.
    ///
    /// This callback takes precedence over `with_pivot_count`, including when
    /// it returns zero, regardless of builder order. Existing session-only
    /// registrations retain their behavior. A separate `with_can_swipe` policy
    /// remains available for custom gestures that are not pivot navigation.
    pub fn with_route_pivot_count(mut self, value: fn(&Route, &Session) -> usize) -> Self {
        self.route_pivot_count = Some(value);
        self
    }
    pub fn with_overlay(mut self, value: fn(&Session) -> bool) -> Self {
        self.overlay = Some(value);
        self
    }
    pub fn with_host_navigation(mut self, value: fn(&PhoneViewModel, &Session, &str)) -> Self {
        self.host_navigation = Some(value);
        self
    }
    pub fn with_host_weather(mut self, value: fn(&Session, String, f64, String)) -> Self {
        self.host_weather = Some(value);
        self
    }
    pub fn with_tick(mut self, value: TickHandler) -> Self {
        self.tick = Some(value);
        self
    }
    /// Opt into the shared foreground reminder service. Providers are collected
    /// independently of the active route, and require Notifications or All.
    /// This does not register an OS background agent or wake-up alarm.
    pub fn with_reminders(mut self, value: ReminderProvider) -> Self {
        self.reminders = Some(value);
        self
    }
    pub fn with_lifecycle(mut self, value: fn(AppContext, AppLifecycle)) -> Self {
        self.lifecycle = Some(value);
        self
    }
    /// Observe the current library player's progress while any app is visible.
    /// Delivery requires an enabled app declaring Media (or All), and remains
    /// scoped to this running shell and the current phone document.
    pub fn with_media_event(mut self, value: MediaEventHandler) -> Self {
        self.media_event = Some(value);
        self
    }
    pub(crate) fn context(&self, vm: &Rc<PhoneViewModel>, session: &Rc<Session>) -> AppContext {
        AppContext {
            vm: vm.clone(),
            session: session.clone(),
            id: self.manifest.id,
        }
    }
}
