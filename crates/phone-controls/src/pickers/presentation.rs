//! A native Popup presents a phone page while the native picker owns its value.
//! All callbacks are weak, popup placement settles after native flyout setup,
//! and privacy/navigation teardown bypasses decorative closing motion.
use super::templates;
use ferroui_base::animation::easings::{Easing, IEasing, LinearEasing, SharedEasing};
use ferroui_base::animation::{Animation, Cue, KeyFrame, SignedDuration};
use ferroui_base::input::{KeyModifiers, NavigationMethod};
use ferroui_base::media::{ITransform, Rotate3DTransform};
use ferroui_base::reactive::{IDisposable, Subscription};
use ferroui_base::styling::Setter;
use ferroui_base::threading::{CancellationToken, Dispatcher, DispatcherPriority};
use ferroui_base::{Rect, Ref, RelativePoint, Thickness, Visual, WeakRef};
use ferroui_controls::primitives::popup_positioning::{
    PopupAnchor, PopupGravity, PopupPositionerConstraintAdjustment,
};
use ferroui_controls::primitives::{Popup, TemplatedControl};
use ferroui_controls::{Border, Control, DatePickerPresenter, PlacementMode, TopLevel};
use std::{
    cell::{Cell, RefCell},
    rc::{Rc, Weak},
    sync::Arc,
};

ferroui_base::ferro_thread_local! {
    static OPEN: RefCell<Vec<Weak<PhonePicker>>> = const { RefCell::new(Vec::new()) };
}

pub fn has_open_phone_picker() -> bool {
    active().is_some()
}

/// Consume Back before the shell closes an editor or pops its journal.
pub fn dismiss_phone_picker() -> bool {
    let Some(picker) = active() else {
        return false;
    };
    if let Some(presenter) = picker.presenter.upgrade() {
        if let Some(bar) = presenter
            .get_template_descendants()
            .iter()
            .find(|part| part.has_name("PhonePickerApplicationBar"))
            .and_then(|part| part.cast::<Border>())
        {
            if bar.classes().contains("expanded") {
                templates::set_bar_expanded(&bar, false);
                return true;
            }
        }
    }
    if let Some(popup) = picker.popup.upgrade() {
        popup.close();
    }
    true
}

/// Remove native popup surfaces immediately before lock, alarms or navigation.
/// The native selected value is unchanged; this never invokes confirmation.
pub fn close_phone_pickers() {
    let pickers = OPEN.with(|open| {
        open.borrow()
            .iter()
            .filter_map(Weak::upgrade)
            .collect::<Vec<_>>()
    });
    for picker in pickers {
        picker.close_immediately();
    }
    OPEN.with(|open| open.borrow_mut().clear());
}

fn active() -> Option<Rc<PhonePicker>> {
    OPEN.with(|open| {
        let mut open = open.borrow_mut();
        open.retain(|entry| entry.upgrade().is_some_and(|picker| picker.is_open()));
        open.last().and_then(Weak::upgrade)
    })
}

pub(super) struct PhonePicker {
    owner: WeakRef<TemplatedControl>,
    popup: WeakRef<Popup>,
    presenter: WeakRef<TemplatedControl>,
    generation: Cell<u64>,
    closing: Cell<bool>,
    force: Cell<bool>,
    restore_focus: Cell<bool>,
    cancellation: RefCell<Option<CancellationToken>>,
    subscriptions: RefCell<Vec<Subscription>>,
    property_subscriptions: RefCell<Vec<Rc<dyn IDisposable>>>,
    target_subscription: RefCell<Option<Rc<dyn IDisposable>>>,
}

impl PhonePicker {
    pub(super) fn attach(
        owner: &Ref<TemplatedControl>,
        popup: &Ref<Popup>,
        presenter: &Ref<TemplatedControl>,
    ) -> Rc<Self> {
        let state = Rc::new(Self {
            owner: owner.downgrade(),
            popup: popup.downgrade(),
            presenter: presenter.downgrade(),
            generation: Cell::new(0),
            closing: Cell::new(false),
            force: Cell::new(false),
            restore_focus: Cell::new(true),
            cancellation: RefCell::new(None),
            subscriptions: RefCell::new(Vec::new()),
            property_subscriptions: RefCell::new(Vec::new()),
            target_subscription: RefCell::new(None),
        });
        presenter.classes().add("phone-picker-presenter");
        presenter.set_template(Some(templates::presenter_template(
            presenter.cast::<DatePickerPresenter>().is_some(),
        )));
        presenter.set_margin(Thickness::uniform(0.0));
        presenter.set_min_width(0.0);
        presenter.set_min_height(0.0);
        popup.set_should_use_overlay_layer(true);
        // This is a phone page: the persistent hardware Back/Home strip must
        // remain reachable, and tapping status chrome must not commit or close.
        popup.set_is_light_dismiss_enabled(false);
        popup.set_overlay_dismiss_event_pass_through(false);

        let weak = Rc::downgrade(&state);
        let opened = popup.opened(move || {
            if let Some(state) = weak.upgrade() {
                state.opened();
            }
        });
        let weak = Rc::downgrade(&state);
        let closed = popup.closed(move || {
            if let Some(state) = weak.upgrade() {
                state.closed();
            }
        });
        let weak = Rc::downgrade(&state);
        let closing = popup.closing(move |event| {
            let Some(state) = weak.upgrade() else {
                return;
            };
            if state.force.get() || !crate::motion_enabled() {
                return;
            }
            let Some(presenter) = state.presenter.upgrade() else {
                return;
            };
            if !presenter.is_attached_to_visual_tree()
                || !state
                    .owner
                    .upgrade()
                    .is_some_and(|owner| owner.is_effectively_visible())
            {
                return;
            }
            event.set_cancel(true);
            if !state.closing.replace(true) {
                state.animate(true);
            }
        });
        state
            .subscriptions
            .borrow_mut()
            .extend([opened, closed, closing]);
        let weak = Rc::downgrade(&state);
        let property = presenter.property_changed(move |change| {
            if matches!(
                change.property().name(),
                "Date"
                    | "Time"
                    | "ClockIdentifier"
                    | "UseSeconds"
                    | "DayFormat"
                    | "MonthFormat"
                    | "YearFormat"
                    | "MinYear"
                    | "MaxYear"
                    | "FontSize"
            ) {
                if let Some(state) = weak.upgrade() {
                    if state.is_open() {
                        state.reflow();
                    }
                }
            }
        });
        state.property_subscriptions.borrow_mut().push(property);
        state
    }

    fn is_open(&self) -> bool {
        self.popup.upgrade().is_some_and(|popup| popup.is_open())
    }

    fn opened(self: &Rc<Self>) {
        self.restore_focus.set(true);
        self.closing.set(false);
        self.cancel_animation();
        OPEN.with(|open| {
            let mut open = open.borrow_mut();
            open.retain(|entry| {
                entry
                    .upgrade()
                    .is_some_and(|picker| picker.is_open() && !Rc::ptr_eq(&picker, self))
            });
            open.push(Rc::downgrade(self));
        });
        if let Some(presenter) = self.presenter.upgrade() {
            presenter.set_opacity(0.0);
        }
        // Native DatePicker/TimePicker computes a button-relative offset after
        // publishing Opened. Replace it once that synchronous operation ends.
        let generation = self.generation.get();
        let weak = Rc::downgrade(self);
        Dispatcher::ui_thread().post_local(
            move || {
                let Some(state) = weak.upgrade() else {
                    return;
                };
                if generation != state.generation.get() || !state.is_open() {
                    return;
                }
                state.reflow();
                if let Some(presenter) = state.presenter.upgrade() {
                    presenter.set_opacity(1.0);
                }
                state.animate(false);
            },
            DispatcherPriority::LOADED,
        );
    }

    fn target(&self) -> Option<Ref<Control>> {
        let owner = self.owner.upgrade()?;
        owner
            .get_visual_ancestors()
            .find_map(|part| {
                part.cast::<Control>()
                    .filter(|part| part.has_name("PhonePage"))
            })
            .or_else(|| TopLevel::get_top_level(Some(&owner)).map(|root| root.upcast()))
    }

    fn reflow(self: &Rc<Self>) {
        let (Some(popup), Some(presenter), Some(target)) = (
            self.popup.upgrade(),
            self.presenter.upgrade(),
            self.target(),
        ) else {
            return;
        };
        let size = target.bounds().size();
        if !size.width.is_finite()
            || !size.height.is_finite()
            || size.width <= 0.0
            || size.height <= 0.0
        {
            return;
        }
        presenter.set_width(size.width);
        presenter.set_height(size.height);
        presenter.set_max_width(size.width);
        presenter.set_max_height(size.height);
        popup.set_width(size.width);
        popup.set_height(size.height);
        popup.set_placement_target(target.clone());
        popup.set_placement(PlacementMode::AnchorAndGravity);
        popup.set_placement_anchor(PopupAnchor::TOP_LEFT);
        popup.set_placement_gravity(PopupGravity::BOTTOM_RIGHT);
        popup.set_placement_rect(Some(Rect::new(0.0, 0.0, 0.0, 0.0)));
        popup.set_placement_constraint_adjustment(PopupPositionerConstraintAdjustment::NONE);
        popup.set_horizontal_offset(0.0);
        popup.set_vertical_offset(0.0);
        templates::reflow(&presenter, size.width, size.height);
        if self.target_subscription.borrow().is_none() {
            let weak = Rc::downgrade(self);
            let subscription = target.property_changed(move |change| {
                if change.property() == Visual::bounds_property().as_property() {
                    if let Some(state) = weak.upgrade() {
                        if state.is_open() {
                            state.reflow();
                        }
                    }
                }
            });
            *self.target_subscription.borrow_mut() = Some(subscription);
        }
    }

    fn surface(&self) -> Option<Ref<Control>> {
        self.presenter
            .upgrade()?
            .get_template_descendants()
            .into_iter()
            .find(|part| part.has_name("PhonePickerSurface"))
            .and_then(|part| part.cast::<Control>())
    }

    fn animate(self: &Rc<Self>, closing: bool) {
        let Some(surface) = self.surface() else {
            if closing {
                self.finish_close();
            }
            return;
        };
        let current = surface.render_transform().and_then(|transform| {
            transform
                .as_object()
                .and_then(|object| object.downcast_ref::<Rotate3DTransform>())
                .map(|rotate| rotate.angle_x())
        });
        self.cancel_animation();
        surface.set_render_transform(None);
        if !crate::motion_enabled() || !surface.is_attached_to_visual_tree() {
            if closing {
                self.finish_close();
            }
            return;
        }
        let rotation = Rotate3DTransform::new();
        let width = self
            .presenter
            .upgrade()
            .map_or(surface.bounds().width, |presenter| presenter.width());
        rotation.set_depth(width.max(1.0) * (1000.0 / 480.0));
        surface.set_render_transform_origin(RelativePoint::CENTER);
        let transform: Rc<dyn ITransform> = rotation.clone().into();
        surface.set_render_transform(Some(transform.clone()));
        surface.set_is_hit_test_visible(false);
        let animation = Animation::new();
        animation.set_duration(SignedDuration::from_seconds(0.2));
        if closing {
            animation.set_easing(Easing::new(PickerCloseEase));
        } else {
            animation.set_easing(LinearEasing::new());
        }
        for (cue, angle) in [
            (
                0.0,
                if closing {
                    current.unwrap_or(0.0)
                } else {
                    -50.0
                },
            ),
            (1.0, if closing { 90.0 } else { 0.0 }),
        ] {
            let frame = KeyFrame::new();
            frame.set_cue(Cue::new(cue));
            frame
                .setters()
                .add(Setter::new(Rotate3DTransform::angle_x_property(), angle));
            animation.children().add(frame);
        }
        let cancellation = CancellationToken::new();
        *self.cancellation.borrow_mut() = Some(cancellation.clone());
        let generation = self.generation.get();
        let weak = Rc::downgrade(self);
        let task = animation.run_async(&surface, cancellation);
        task.on_completed(move || {
            let Some(state) = weak.upgrade() else {
                return;
            };
            if generation != state.generation.get() {
                return;
            }
            state.cancellation.borrow_mut().take();
            if let Some(surface) = state.surface() {
                if surface
                    .render_transform()
                    .is_some_and(|value| Rc::ptr_eq(&value, &transform))
                {
                    surface.set_render_transform(None);
                }
                surface.set_is_hit_test_visible(true);
            }
            if closing {
                state.finish_close();
            }
        });
    }

    fn cancel_animation(&self) {
        self.generation.set(self.generation.get().wrapping_add(1));
        let cancellation = self.cancellation.borrow_mut().take();
        if let Some(cancellation) = cancellation {
            cancellation.cancel();
        }
        if let Some(surface) = self.surface() {
            surface.set_render_transform(None);
            surface.set_is_hit_test_visible(true);
        }
    }

    fn finish_close(&self) {
        self.force.set(true);
        if let Some(popup) = self.popup.upgrade() {
            popup.close();
        }
        self.force.set(false);
    }

    pub(super) fn close_immediately(&self) {
        self.restore_focus.set(false);
        self.cancel_animation();
        self.finish_close();
    }

    fn closed(&self) {
        self.cancel_animation();
        self.closing.set(false);
        if let Some(subscription) = self.target_subscription.borrow_mut().take() {
            subscription.dispose();
        }
        if let Some(presenter) = self.presenter.upgrade() {
            presenter.set_opacity(1.0);
            for part in presenter.get_template_descendants() {
                if let Some(panel) =
                    part.cast::<ferroui_controls::primitives::DateTimePickerPanel>()
                {
                    panel.classes().remove("expanded");
                }
                if part.has_name("PhonePickerApplicationBar") {
                    if let Some(bar) = part.cast::<Border>() {
                        templates::set_bar_expanded(&bar, false);
                    }
                }
            }
        }
        if self.restore_focus.get() {
            if let Some(owner) = self.owner.upgrade().filter(|owner| {
                owner.is_attached_to_visual_tree()
                    && owner.is_effectively_visible()
                    && owner.is_effectively_enabled()
            }) {
                // WinUI's native picker delegates focus to its flyout button;
                // the outer DatePicker/TimePicker is not necessarily focusable.
                // Restore the actual field target after normal dismissal.
                if !owner.focus_with(NavigationMethod::Unspecified, KeyModifiers::NONE) {
                    if let Some(button) = owner
                        .get_template_descendants()
                        .into_iter()
                        .find(|part| part.has_name("PART_FlyoutButton"))
                        .and_then(|part| part.cast::<ferroui_controls::Button>())
                    {
                        button.focus_with(NavigationMethod::Unspecified, KeyModifiers::NONE);
                    }
                }
            }
        }
    }
}

impl Drop for PhonePicker {
    fn drop(&mut self) {
        self.close_immediately();
        if let Some(subscription) = self.target_subscription.get_mut().take() {
            subscription.dispose();
        }
        for subscription in self.property_subscriptions.get_mut().drain(..) {
            subscription.dispose();
        }
    }
}

#[derive(Clone, Copy)]
struct PickerCloseEase;
impl IEasing for PickerCloseEase {
    fn ease(&self, progress: f64) -> f64 {
        ((2.0 * progress.clamp(0.0, 1.0)).exp() - 1.0) / (2.0_f64.exp() - 1.0)
    }
    fn to_shared(&self) -> Arc<SharedEasing> {
        Arc::new(*self)
    }
}
