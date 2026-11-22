use super::{
    drag,
    motion::{Axis, Motion},
};
use crate::session::Session;
use ferroui_base::input::{
    ContextRequestedEventArgs, IPointer, InputElement, MouseButton, Pointer, PointerEventArgs,
    PointerPressedEventArgs, PointerReleasedEventArgs, PointerType,
};
use ferroui_base::reactive::Subscription;
use ferroui_base::threading::{DispatcherPriority, DispatcherTimer};
use ferroui_base::{Ref, WeakRef};
use ferroui_controls::{Control, Grid, TextBox};
use phone_domain::{AppId, Id, Route};
use phone_state::PhoneViewModel;
use std::{
    cell::{Cell, RefCell},
    collections::BTreeMap,
    rc::{Rc, Weak},
    time::{Duration, Instant},
};

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Mode {
    Pending,
    Native,
    Scroll,
    Swipe,
    OpenNotifications,
    CloseNotifications,
    Unlock,
    Drag,
    Held,
    DismissNotification,
    Canceled,
}
#[derive(Clone)]
pub(super) struct Hover {
    pub target: Id,
    pub center: bool,
    pub after: bool,
    pub entered: Instant,
}
pub(super) struct Contact {
    pub pointer: Rc<dyn IPointer>,
    pub motion: Motion,
    pub route: Route,
    pub mode: Mode,
    pub tile: Option<Id>,
    pub notification: Option<Id>,
    pub back: bool,
    pub context: Option<WeakRef<Control>>,
    pub held: bool,
    pub hover: Option<Hover>,
    pub notification_open: bool,
    pub locked: bool,
}
pub(super) struct Controller {
    pub root: WeakRef<Grid>,
    pub vm: Weak<PhoneViewModel>,
    pub ui: Weak<Session>,
    pub contacts: RefCell<BTreeMap<i32, Contact>>,
    pointers: RefCell<Vec<Weak<dyn IPointer>>>,
    pub drag_visual: RefCell<Option<drag::VisualDrag>>,
    hold_timer: RefCell<Option<Subscription>>,
    drag_timer: RefCell<Option<Subscription>>,
    transitioning: Cell<bool>,
}
impl Controller {
    pub fn new(root: &Ref<Grid>, vm: &Rc<PhoneViewModel>, ui: &Rc<Session>) -> Rc<Self> {
        Rc::new(Self {
            root: root.downgrade(),
            vm: Rc::downgrade(vm),
            ui: Rc::downgrade(ui),
            contacts: RefCell::new(BTreeMap::new()),
            pointers: RefCell::new(Vec::new()),
            drag_visual: RefCell::new(None),
            hold_timer: RefCell::new(None),
            drag_timer: RefCell::new(None),
            transitioning: Cell::new(false),
        })
    }
    pub fn cancel(&self) -> bool {
        self.hold_timer.borrow_mut().take();
        self.drag_timer.borrow_mut().take();
        let contacts = self.contacts.take();
        let pointers = self
            .pointers
            .take()
            .into_iter()
            .filter_map(|pointer| pointer.upgrade())
            .collect::<Vec<_>>();
        let active = !contacts.is_empty() || !pointers.is_empty();
        self.clear_feedback();
        for pointer in pointers {
            pointer.capture(None);
        }
        for contact in contacts.into_values() {
            contact.pointer.capture(None);
        }
        active
    }
    pub fn lost(&self, id: i32) {
        if self.transitioning.get() {
            return;
        }
        let contact = self.contacts.borrow_mut().remove(&id);
        if contact.is_some() {
            self.hold_timer.borrow_mut().take();
            self.drag_timer.borrow_mut().take();
            self.clear_feedback();
        }
    }
    fn clear_feedback(&self) {
        drag::clear(self);
        if let Some(ui) = self.ui.upgrade() {
            if ui.drop_target.get_untracked().is_some() {
                ui.drop_target.set(None);
            }
        }
    }
    fn capture(&self, pointer: &Rc<dyn IPointer>) {
        if let Some(root) = self.root.upgrade() {
            self.transitioning.set(true);
            pointer.capture(Some(&root.upcast::<InputElement>()));
            if let Some(pointer) = pointer.as_any().downcast_ref::<Pointer>() {
                pointer.set_is_gesture_recognition_skipped(true);
            }
            self.transitioning.set(false);
        }
    }
    pub fn pressed(self: &Rc<Self>, event: &PointerPressedEventArgs) {
        if !event.properties().is_left_button_pressed {
            return;
        }
        let (Some(root), Some(vm), Some(ui)) =
            (self.root.upgrade(), self.vm.upgrade(), self.ui.upgrade())
        else {
            return;
        };
        let point = event.get_position(Some(&root));
        let pointer = event.pointer();
        let source = event.source().and_then(|source| source.cast::<Control>());
        if let Some(source) = &source {
            InputElement::set_is_holding_enabled(source, true);
            InputElement::set_is_hold_with_mouse_enabled(source, true);
        }
        let ancestors = source
            .as_ref()
            .map(|source| {
                source
                    .get_self_and_visual_ancestors()
                    .filter_map(|visual| visual.cast::<Control>())
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        let tile = ancestors.iter().find_map(|control| {
            control
                .name()
                .and_then(|name| name.strip_prefix("Tile-").and_then(|id| id.parse().ok()))
        });
        let notification = ancestors.iter().find_map(|control| {
            control.name().and_then(|name| {
                name.strip_prefix("Notification-")
                    .and_then(|id| id.parse().ok())
            })
        });
        let back = ancestors
            .iter()
            .any(|control| control.has_name("HardwareBack"));
        let context = ancestors
            .iter()
            .find(|control| control.name().is_some_and(|name| name.starts_with("App-")))
            .map(|control| control.downgrade());
        let editing = ancestors.iter().any(|control| {
            control.cast::<TextBox>().is_some()
                || control
                    .name()
                    .is_some_and(|name| name.starts_with("Hardware") && !back)
        });
        let native = editing
            || ui.menu.get_untracked()
            || ui.dialog.get_untracked().is_some()
            || ui.switcher_open.get_untracked()
            || ui.alphabet.get_untracked();
        let single = self.contacts.borrow().is_empty()
            && (pointer.is_primary() || pointer.type_() == PointerType::Mouse);
        let mut previous = if single {
            Vec::new()
        } else {
            self.hold_timer.borrow_mut().take();
            self.drag_timer.borrow_mut().take();
            let mut contacts = self.contacts.borrow_mut();
            contacts
                .values_mut()
                .map(|contact| {
                    contact.mode = Mode::Canceled;
                    contact.pointer.clone()
                })
                .collect::<Vec<_>>()
        };
        {
            let mut pointers = self.pointers.borrow_mut();
            pointers.retain(|old| old.upgrade().is_some_and(|old| old.id() != pointer.id()));
            if !single {
                for old in pointers.iter().filter_map(Weak::upgrade) {
                    if !previous.iter().any(|other| other.id() == old.id()) {
                        previous.push(old);
                    }
                }
            }
            pointers.push(Rc::downgrade(pointer));
        }
        self.contacts.borrow_mut().insert(
            pointer.id(),
            Contact {
                pointer: pointer.clone(),
                motion: Motion::new(point.x, point.y, event.timestamp()),
                route: vm.route(),
                mode: if !single {
                    Mode::Canceled
                } else if native {
                    Mode::Native
                } else {
                    Mode::Pending
                },
                tile,
                notification,
                back,
                context: context.clone(),
                held: false,
                hover: None,
                notification_open: ui.notification_open.get_untracked()
                    || vm.route() == Route::Notifications,
                locked: ui.locked.get_untracked() || vm.route() == Route::Lock,
            },
        );
        for old in previous {
            old.capture(None);
        }
        if !single {
            event.set_handled(true);
            self.capture(pointer);
            self.clear_feedback();
            return;
        }
        if !native && (tile.is_some() || back || context.is_some()) {
            let weak = Rc::downgrade(self);
            let id = pointer.id();
            *self.hold_timer.borrow_mut() = Some(DispatcherTimer::run_once(
                move || {
                    if let Some(controller) = weak.upgrade() {
                        controller.hold(id);
                    }
                },
                Duration::from_millis(500),
                DispatcherPriority::INPUT,
            ));
        }
    }
    pub fn hold(self: &Rc<Self>, id: i32) {
        let (Some(vm), Some(ui)) = (self.vm.upgrade(), self.ui.upgrade()) else {
            return;
        };
        let plan = {
            let mut contacts = self.contacts.borrow_mut();
            let Some(contact) = contacts.get_mut(&id) else {
                return;
            };
            if contact.held
                || contact.mode != Mode::Pending
                || !contact.motion.is_tap()
                || contact.route != vm.route()
            {
                return;
            }
            contact.held = true;
            let owned = contact.tile.is_some() || contact.back || contact.context.is_some();
            if owned {
                contact.mode = if contact.tile.is_some() {
                    Mode::Drag
                } else {
                    Mode::Held
                };
            }
            (
                contact.pointer.clone(),
                contact.tile,
                contact.back,
                contact.context.clone(),
                owned,
            )
        };
        self.hold_timer.borrow_mut().take();
        if plan.4 {
            self.capture(&plan.0);
        }
        if let Some(id) = plan.1 {
            ui.edit_tile.set(Some(id));
            self.start_drag_timer();
        } else if plan.2 {
            ui.switcher_open.set(true);
        } else if let Some(context) = plan.3.and_then(|weak| weak.upgrade()) {
            context.raise_event(&ContextRequestedEventArgs::new());
        }
    }
    pub fn moved(self: &Rc<Self>, event: &PointerEventArgs) {
        let (Some(root), Some(vm), Some(ui)) =
            (self.root.upgrade(), self.vm.upgrade(), self.ui.upgrade())
        else {
            return;
        };
        let point = event.get_position(Some(&root));
        let (capture, owned, dragging, cancel_hold) = {
            let mut contacts = self.contacts.borrow_mut();
            let Some(contact) = contacts.get_mut(&event.pointer().id()) else {
                return;
            };
            contact.motion.update(point.x, point.y, event.timestamp());
            if contact.route != vm.route() || contact.motion.axis == Axis::Invalid {
                contact.mode = Mode::Canceled;
            }
            let old = contact.mode;
            if contact.mode == Mode::Pending {
                let (dx, dy) = contact.motion.delta();
                if contact.tile.is_some()
                    && ui.edit_tile.get_untracked().is_some()
                    && !contact.motion.is_tap()
                {
                    contact.mode = Mode::Drag;
                } else if contact.pointer.type_() != PointerType::Mouse {
                    contact.mode = match contact.motion.axis {
                        Axis::Vertical if contact.locked && dy < 0.0 => Mode::Unlock,
                        Axis::Vertical
                            if !contact.notification_open
                                && contact.motion.origin.1 <= 32.0
                                && dy > 0.0 =>
                        {
                            Mode::OpenNotifications
                        }
                        Axis::Vertical
                            if contact.notification_open
                                && contact.motion.origin.1 >= root.bounds().height - 54.0 - 76.0
                                && dy < 0.0 =>
                        {
                            Mode::CloseNotifications
                        }
                        Axis::Vertical => Mode::Scroll,
                        Axis::Horizontal
                            if contact.notification_open && contact.notification.is_some() =>
                        {
                            Mode::DismissNotification
                        }
                        Axis::Horizontal
                            if !contact.notification_open
                                && !contact.locked
                                && !ui.menu.get_untracked()
                                && can_swipe(&contact.route, &ui) =>
                        {
                            let _ = dx;
                            Mode::Swipe
                        }
                        _ => Mode::Pending,
                    };
                }
            }
            let owned = matches!(
                contact.mode,
                Mode::Drag
                    | Mode::Swipe
                    | Mode::OpenNotifications
                    | Mode::CloseNotifications
                    | Mode::Unlock
                    | Mode::Held
                    | Mode::DismissNotification
            );
            (
                old != contact.mode && owned,
                owned,
                contact.mode == Mode::Drag,
                !contact.motion.is_tap(),
            )
        };
        if cancel_hold {
            self.hold_timer.borrow_mut().take();
        }
        if capture {
            self.capture(event.pointer());
        }
        if owned {
            event.set_handled(true);
        }
        if dragging {
            self.start_drag_timer();
            drag::update(self, false);
        }
    }
    pub fn released(self: &Rc<Self>, event: &PointerReleasedEventArgs) {
        if event.initial_press_mouse_button() != MouseButton::Left {
            return;
        }
        self.moved(event);
        self.hold_timer.borrow_mut().take();
        self.drag_timer.borrow_mut().take();
        let contact = self.contacts.borrow_mut().remove(&event.pointer().id());
        self.pointers.borrow_mut().retain(|pointer| {
            pointer
                .upgrade()
                .is_some_and(|pointer| pointer.id() != event.pointer().id())
        });
        let Some(contact) = contact else {
            if event.pointer().type_() != PointerType::Mouse {
                event.set_handled(true);
                event.pointer().capture(None);
            }
            return;
        };
        self.clear_feedback();
        let suppressed = contact.mode != Mode::Native
            && (contact.held || !contact.motion.is_tap() || contact.mode == Mode::Canceled);
        if suppressed {
            event.set_handled(true);
            contact.pointer.capture(None);
        }
        let (Some(vm), Some(ui)) = (self.vm.upgrade(), self.ui.upgrade()) else {
            return;
        };
        if contact.route != vm.route() {
            return;
        }
        match contact.mode {
            Mode::Drag if contact.motion.distance >= 8.0 => drag::finish(&vm, &ui, &contact),
            Mode::OpenNotifications if contact.motion.swipe(Axis::Vertical) => {
                ui.notification_open.set(true)
            }
            Mode::CloseNotifications if contact.motion.swipe(Axis::Vertical) => {
                ui.notification_open.set(false);
                if vm.route() == Route::Notifications {
                    vm.back();
                }
            }
            Mode::Unlock if contact.motion.swipe(Axis::Vertical) => {
                ui.locked.set(false);
                if vm.route() == Route::Lock {
                    vm.back();
                }
            }
            Mode::DismissNotification if contact.motion.swipe(Axis::Horizontal) => {
                if let Some(id) = contact.notification {
                    vm.dismiss_notification(id);
                }
            }
            Mode::Swipe if contact.motion.swipe(Axis::Horizontal) => {
                complete_swipe(&vm, &ui, &contact.route, contact.motion.delta().0)
            }
            _ => {}
        }
    }
    fn start_drag_timer(self: &Rc<Self>) {
        if self.drag_timer.borrow().is_some() {
            return;
        }
        let weak = Rc::downgrade(self);
        *self.drag_timer.borrow_mut() = Some(DispatcherTimer::run(
            move || {
                weak.upgrade()
                    .is_some_and(|controller| drag::update(&controller, true))
            },
            Duration::from_millis(40),
            DispatcherPriority::BACKGROUND,
        ));
    }
}

fn pivot_count(route: &Route, ui: &Session) -> usize {
    match route {
        Route::App(AppId::Phone) if ui.pivot.get_untracked() < 2 => 2,
        Route::App(AppId::People | AppId::Settings | AppId::Music | AppId::Video) => 2,
        Route::App(AppId::Calendar | AppId::Outlook) => 3,
        Route::App(AppId::Photos) if ui.selected.get_untracked().is_none() => 3,
        _ => 0,
    }
}
fn can_swipe(route: &Route, ui: &Session) -> bool {
    matches!(route, Route::Start | Route::Folder(_) | Route::Apps)
        || pivot_count(route, ui) > 0
        || (*route == Route::App(AppId::Photos)
            && ui.selected.get_untracked().is_some()
            && ui.photo_zoom.get_untracked() <= 1.0)
}
fn complete_swipe(vm: &PhoneViewModel, ui: &Session, route: &Route, delta: f64) {
    match route {
        Route::Start | Route::Folder(_) if delta < 0.0 => {
            ui.edit_tile.set(None);
            ui.folder.set(None);
            vm.navigate(Route::Apps);
        }
        Route::Apps if delta > 0.0 => vm.home(),
        Route::App(AppId::Photos) if ui.selected.get_untracked().is_some() => {
            let data = vm.data();
            let photos = &data.photos;
            if let Some(index) = photos
                .iter()
                .position(|photo| Some(photo.id) == ui.selected.get_untracked())
            {
                let next = if delta < 0.0 {
                    (index + 1).min(photos.len().saturating_sub(1))
                } else {
                    index.saturating_sub(1)
                };
                if let Some(photo) = photos.get(next) {
                    ui.selected.set(Some(photo.id));
                }
            }
        }
        _ => {
            let count = pivot_count(route, ui);
            if count > 0 {
                let current = ui.pivot.get_untracked().min(count - 1);
                ui.pivot.set(if delta < 0.0 {
                    (current + 1) % count
                } else {
                    (current + count - 1) % count
                });
            }
        }
    }
}
