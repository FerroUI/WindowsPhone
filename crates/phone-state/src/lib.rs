//! Retained ReactiveUI view model with granular DynamicData collection projections.
//! Navigation and clock updates never write persisted application data.
mod app_data;
mod calendar;
mod commands;
mod communication;
mod convenience;
mod documents;
mod media_library;
mod mutation;
mod navigation;
mod note_storage;
mod pim;
mod platform;
mod projection;
mod secondary_tiles;
mod services;
mod tiles;
mod validation;

use ferroui_dynamic_data::SourceCache;
use ferroui_reactive::{concurrency::ImmediateScheduler, ObservableExt};
use ferroui_reactiveui::{reactive_object, IReactiveObjectExt, ReactiveCommand, ReactiveObject};
pub use navigation::RecentApp;
pub use phone_domain::Action;
use phone_domain::*;
pub use services::*;
use std::{
    cell::{Cell, Ref, RefCell},
    collections::VecDeque,
    rc::Rc,
};

pub struct PhoneViewModel {
    reactive: ReactiveObject,
    revision: RefCell<u64>,
    document: RefCell<PhoneData>,
    document_epoch: Cell<u64>,
    schedule_revision: Cell<u64>,
    current_route: RefCell<Route>,
    registered_apps: RefCell<std::collections::BTreeSet<String>>,
    history: RefCell<Vec<Route>>,
    recent: RefCell<Vec<(AppId, Route)>>,
    extension_recent: RefCell<Vec<(String, Route)>>,
    recent_order: RefCell<Vec<RecentApp>>,
    viewport_size: Cell<(f64, f64)>,
    clock: Cell<i64>,
    utc_offset: Cell<i32>,
    status_text: RefCell<String>,
    services: RefCell<HostServices>,
    environment: RefCell<HostEnvironment>,
    platform_state: RefCell<phone_host::platform::PlatformState>,
    device_applications: RefCell<Vec<phone_host::platform::DeviceApplication>>,
    device_applications_revision: Cell<u64>,
    save: SaveCallback,
    applying: Cell<bool>,
    pending: RefCell<VecDeque<Action>>,
    pub contacts: SourceCache<Rc<Contact>, Id>,
    pub tiles: SourceCache<Rc<Tile>, Id>,
    pub notifications: SourceCache<Rc<Notification>, Id>,
    pub execute: ReactiveCommand<Option<Action>, ()>,
}
reactive_object!(PhoneViewModel);

impl PhoneViewModel {
    pub fn new(mut data: PhoneData, save: SaveCallback) -> Rc<Self> {
        let error = data.normalize().err();
        if error.is_some() {
            data = PhoneData::demo();
        }
        let vm = Rc::new_cyclic(|weak: &std::rc::Weak<Self>| {
            let receiver = weak.clone();
            Self {
                reactive: ReactiveObject::new(),
                revision: RefCell::new(0),
                document: RefCell::new(data),
                document_epoch: Cell::new(0),
                schedule_revision: Cell::new(0),
                current_route: RefCell::new(Route::Start),
                registered_apps: RefCell::new(Default::default()),
                history: RefCell::new(Vec::new()),
                recent: RefCell::new(Vec::new()),
                extension_recent: RefCell::new(Vec::new()),
                recent_order: RefCell::new(Vec::new()),
                viewport_size: Cell::new((400.0, 800.0)),
                clock: Cell::new(1_795_363_200),
                utc_offset: Cell::new(0),
                status_text: RefCell::new(error.unwrap_or_default()),
                services: RefCell::new(HostServices::default()),
                environment: RefCell::new(HostEnvironment::default()),
                platform_state: RefCell::new(Default::default()),
                device_applications: RefCell::new(Vec::new()),
                device_applications_revision: Cell::new(0),
                save,
                applying: Cell::new(false),
                pending: RefCell::new(VecDeque::new()),
                contacts: SourceCache::new(|value: &Rc<Contact>| value.id),
                tiles: SourceCache::new(|value: &Rc<Tile>| value.id),
                notifications: SourceCache::new(|value: &Rc<Notification>| value.id),
                execute: ReactiveCommand::create_param_with(
                    move |action: Option<Action>| {
                        if let Some(vm) = receiver.upgrade() {
                            vm.apply(action);
                        }
                    },
                    None,
                    Some(ImmediateScheduler::instance()),
                ),
            }
        });
        vm.synchronize();
        vm
    }
    pub fn data(&self) -> Ref<'_, PhoneData> {
        self.document.borrow()
    }
    pub fn revision(&self) -> u64 {
        *self.revision.borrow()
    }
    /// Invalidate transient object references when an entire backup replaces data.
    pub fn document_epoch(&self) -> u64 {
        self.document_epoch.get()
    }
    /// Calendar/alarm mutations invalidate foreground schedules, independently
    /// of navigation, clock ticks and unrelated document changes.
    pub fn schedule_revision(&self) -> u64 {
        self.schedule_revision.get()
    }
    pub fn route(&self) -> Route {
        self.current_route.borrow().clone()
    }
    pub fn now(&self) -> i64 {
        self.clock.get()
    }
    pub fn utc_offset(&self) -> i32 {
        self.utc_offset.get()
    }
    pub fn local_now(&self) -> i64 {
        self.now().saturating_add(i64::from(self.utc_offset.get()))
    }
    pub fn set_utc_offset(&self, seconds: i32) {
        let offset = seconds.clamp(-86400, 86400);
        if self.utc_offset.replace(offset) != offset {
            self.notify();
        }
    }
    pub fn viewport(&self) -> (f64, f64) {
        self.viewport_size.get()
    }
    pub fn set_viewport(&self, width: f64, height: f64) {
        if !width.is_finite() || !height.is_finite() {
            return;
        }
        let size = (width.max(1.0), height.max(1.0));
        if self.viewport_size.replace(size) != size {
            self.notify();
        }
    }
    pub fn set_clock(&self, unix_seconds: i64) {
        let old = self.clock.replace(unix_seconds);
        if old.div_euclid(60) != unix_seconds.div_euclid(60) {
            self.notify();
        }
    }
    pub fn status(&self) -> String {
        self.status_text.borrow().clone()
    }
    pub fn set_status(&self, text: String) {
        if *self.status_text.borrow() != text {
            *self.status_text.borrow_mut() = text;
            self.notify();
        }
    }
    pub fn clear_status(&self) {
        self.set_status(String::new());
    }
    pub fn set_services(&self, services: HostServices) {
        *self.services.borrow_mut() = services;
    }
    pub fn environment(&self) -> HostEnvironment {
        self.environment.borrow().clone()
    }
    pub fn set_environment(&self, mut environment: HostEnvironment) {
        environment.normalize();
        if *self.environment.borrow() != environment {
            *self.environment.borrow_mut() = environment;
            self.notify();
        }
    }
    pub fn motion_enabled(&self) -> bool {
        self.data().settings.motion_enabled() && self.environment.borrow().power_saver != Some(true)
    }
    pub fn live_tiles_enabled(&self) -> bool {
        self.data().settings.live_tiles && self.motion_enabled()
    }
    pub fn request(&self, request: HostRequest) {
        let callback = self.services.borrow().request.clone();
        callback(request);
    }
    pub fn dispatch(&self, action: Action) {
        if self.applying.get() {
            self.pending.borrow_mut().push_back(action);
            return;
        }
        let _subscription = self.execute.execute(Some(action)).subscribe_next(|_| {});
    }
    pub fn search(&self, query: &str) -> Vec<SearchResult> {
        self.data().search(query)
    }
    pub fn export_json(&self) -> Result<String, String> {
        self.data().to_json()
    }
    pub fn import_json(&self, json: &str) -> Result<(), String> {
        let data = PhoneData::from_json(json)?;
        self.replace_data(data)
    }
    pub fn replace_data(&self, data: PhoneData) -> Result<(), String> {
        self.replace_data_after_save(data, |normalized| {
            (self.save)(normalized);
            Ok(())
        })
    }
    /// Host restore transaction: validate and persist the complete candidate
    /// before changing any live state. A quota/disk failure keeps drafts,
    /// navigation, projections and identifiers untouched.
    pub fn replace_data_after_save(
        &self,
        mut data: PhoneData,
        save: impl FnOnce(&PhoneData) -> Result<(), String>,
    ) -> Result<(), String> {
        data.normalize()?;
        save(&data)?;
        *self.document.borrow_mut() = data;
        self.document_epoch
            .set(self.document_epoch.get().wrapping_add(1));
        self.history.borrow_mut().clear();
        self.recent.borrow_mut().clear();
        self.extension_recent.borrow_mut().clear();
        self.recent_order.borrow_mut().clear();
        *self.current_route.borrow_mut() = Route::Start;
        self.synchronize();
        self.notify();
        Ok(())
    }
    pub fn next_id(&self) -> Id {
        let id = self.document.borrow_mut().allocate_id();
        self.persist();
        id
    }
    pub(crate) fn notify(&self) {
        self.raise_and_set_if_changed(&self.revision, self.revision().wrapping_add(1), "Revision");
    }
    pub(crate) fn persist(&self) {
        // No document borrow is held across host code: callbacks can inspect the model.
        let snapshot = self.document.borrow().clone();
        (self.save)(&snapshot);
    }
}
