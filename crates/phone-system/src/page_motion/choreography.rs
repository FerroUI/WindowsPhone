use super::{Entrance, NavigationMotion, NavigationVisit, PendingEntrance, animation, snapshot};
use crate::previews::Preview;
use ferroui_base::animation::IClock;
use ferroui_base::interactivity::RoutedEventHandlerToken;
use ferroui_base::threading::CancellationToken;
use ferroui_base::{Ref, WeakRef};
use ferroui_controls::{Border, Control, Image};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

#[cfg(test)]
#[path = "choreography_tests.rs"]
mod tests;

#[derive(Default)]
pub(super) struct TransitionHost {
    pub(super) page: RefCell<Option<WeakRef<Control>>>,
    page_visit: Cell<Option<NavigationVisit>>,
    outgoing: RefCell<Option<WeakRef<Border>>>,
    active: RefCell<Option<Active>>,
    pub(super) eligible: Cell<bool>,
    viewport: Cell<Option<(f64, f64)>>,
}

struct Active {
    visit: NavigationVisit,
    cancellation: CancellationToken,
    page: WeakRef<Control>,
    opacity: f64,
    enabled: bool,
    hit_test_visible: bool,
    unloaded: RoutedEventHandlerToken,
}

impl TransitionHost {
    /// Take ownership out before cancellation: animation completion callbacks
    /// run synchronously and must see that this visit has already retired.
    pub(super) fn cancel(&self) -> bool {
        let active = self.active.borrow_mut().take();
        let had_active = active.is_some();
        if let Some(active) = active {
            active.cancellation.cancel();
            if let Some(page) = active.page.upgrade() {
                page.remove_handler(Control::unloaded_event(), active.unloaded);
                page.set_opacity(active.opacity);
                page.set_is_enabled(active.enabled);
                page.set_is_hit_test_visible(active.hit_test_visible);
            }
        }
        self.clear_image();
        had_active
    }

    fn clear_image(&self) {
        if let Some(border) = self.outgoing.borrow().as_ref().and_then(WeakRef::upgrade) {
            border.set_opacity(0.0);
            if let Some(image) = border.child().and_then(|child| child.cast::<Image>()) {
                image.set_source(None);
            }
        }
    }

    fn current(&self, visit: NavigationVisit) -> bool {
        self.active
            .borrow()
            .as_ref()
            .is_some_and(|active| active.visit == visit && !active.cancellation.is_cancelled())
    }

    fn show_image(&self, preview: Preview) -> Option<Ref<Control>> {
        let border = self.outgoing.borrow().as_ref().and_then(WeakRef::upgrade)?;
        let image = border.child()?.cast::<Image>()?;
        image.set_source(Some(preview.image));
        border.set_opacity(1.0);
        Some(border.upcast())
    }
}

impl NavigationMotion {
    pub(crate) fn route_departing(&self, next: &phone_domain::Route) -> bool {
        self.route
            .borrow()
            .as_ref()
            .is_some_and(|current| current != next)
    }

    /// Called before route cleanup/reconciliation, while the old native page
    /// still exists. Same-route updates never restart or recapture a transition.
    pub(crate) fn prepare(
        &self,
        route: &phone_domain::Route,
        width: f64,
        height: f64,
        eligible: bool,
    ) -> NavigationVisit {
        self.prepare_with_capture(route, width, height, eligible, || {
            let page = self
                .host
                .page
                .borrow()
                .as_ref()
                .and_then(WeakRef::upgrade)?;
            snapshot::capture(&page)
        })
    }

    fn prepare_with_capture(
        &self,
        route: &phone_domain::Route,
        width: f64,
        height: f64,
        eligible: bool,
        capture: impl FnOnce() -> Option<Preview>,
    ) -> NavigationVisit {
        let changed = self.route.borrow().as_ref() != Some(route);
        let old_visit = NavigationVisit(self.generation.get());
        let previous_eligible = self.host.eligible.replace(eligible);
        let resized = self
            .host
            .viewport
            .replace(Some((width, height)))
            .is_some_and(|previous| previous != (width, height));
        let interrupted = if changed || resized || !eligible {
            self.host.cancel()
        } else {
            false
        };
        let visit = self.observe(route, width);
        if !eligible || (!changed && interrupted) {
            self.pending.borrow_mut().take();
        } else if resized {
            // Layout can change after capture but before the destination's
            // deferred Loaded event. No active animation exists to cancel in
            // that interval. Discard the old-size image while preserving the
            // incoming plan, which resolves against the new page geometry.
            if let Some(plan) = self.pending.borrow_mut().as_mut() {
                plan.outgoing.take();
            }
        } else if changed
            && previous_eligible
            && !interrupted
            && self.host.page_visit.get() == Some(old_visit)
            && self
                .pending
                .borrow()
                .as_ref()
                .is_some_and(|plan| matches!(plan.entrance, Entrance::Turnstile { .. }))
        {
            // At most one image. An unfinished incoming page is never used as
            // an outgoing reference, and capture failure changes no app state.
            let image = ferroui_view::untrack(capture);
            if let Some(plan) = self
                .pending
                .borrow_mut()
                .as_mut()
                .filter(|plan| plan.visit == visit)
            {
                plan.outgoing = image;
            }
        }
        visit
    }

    pub(crate) fn attach_outgoing(&self, border: &Ref<Border>) {
        *self.host.outgoing.borrow_mut() = Some(border.downgrade());
        border.set_is_hit_test_visible(false);
        border.set_is_enabled(false);
        if self.host.active.borrow().is_none() {
            self.host.clear_image();
        }
    }

    pub(crate) fn loaded(self: &Rc<Self>, page: &Ref<Control>, visit: NavigationVisit) {
        if visit.0 != self.generation.get() {
            return;
        }
        *self.host.page.borrow_mut() = Some(page.downgrade());
        self.host.page_visit.set(Some(visit));
        let Some(plan) = self.take_plan(visit, page.bounds().width) else {
            return;
        };
        if !phone_controls::motion_enabled() || !page.is_attached_to_visual_tree() {
            return;
        }
        self.start(page, plan, None);
    }

    fn start(
        self: &Rc<Self>,
        page: &Ref<Control>,
        plan: PendingEntrance,
        clock: Option<Rc<dyn IClock>>,
    ) {
        if plan.entrance == Entrance::None {
            return;
        }
        self.host.cancel();
        let visit = plan.visit;
        let cancellation = CancellationToken::new();
        let weak = Rc::downgrade(self);
        let unloaded = page.add_handler(Control::unloaded_event(), move |_, _| {
            if let Some(motion) = weak.upgrade() {
                motion.cancel_visit(visit);
            }
        });
        *self.host.active.borrow_mut() = Some(Active {
            visit,
            cancellation: cancellation.clone(),
            page: page.downgrade(),
            opacity: page.opacity(),
            enabled: page.is_enabled(),
            hit_test_visible: page.is_hit_test_visible(),
            unloaded,
        });
        if let (Entrance::Turnstile { backward }, Some(preview)) = (plan.entrance, plan.outgoing) {
            let width = preview.logical_size.width;
            if let Some(outgoing) = self.host.show_image(preview) {
                // The destination exists and its route/data are already current,
                // but input cannot fall through the retiring image into it.
                page.set_opacity(0.0);
                page.set_is_enabled(false);
                page.set_is_hit_test_visible(false);
                let weak = Rc::downgrade(self);
                let page = page.downgrade();
                let task = animation::exit(&outgoing, backward, width, cancellation, clock.clone());
                task.on_completed(move || {
                    if let (Some(motion), Some(page)) = (weak.upgrade(), page.upgrade()) {
                        if motion.host.current(visit) {
                            motion.host.clear_image();
                            motion.start_incoming(&page, visit, plan.entrance, clock);
                        }
                    }
                });
                return;
            }
        }
        // No render target or outgoing surface: preserve the existing incoming
        // animation without inventing an image or delaying route mutation.
        self.start_incoming(page, visit, plan.entrance, clock);
    }

    fn start_incoming(
        self: &Rc<Self>,
        page: &Ref<Control>,
        visit: NavigationVisit,
        entrance: Entrance,
        clock: Option<Rc<dyn IClock>>,
    ) {
        let cancellation = {
            let active = self.host.active.borrow();
            let Some(active) = active.as_ref().filter(|active| active.visit == visit) else {
                return;
            };
            page.set_opacity(active.opacity);
            page.set_is_enabled(active.enabled);
            page.set_is_hit_test_visible(active.hit_test_visible);
            active.cancellation.clone()
        };
        let weak = Rc::downgrade(self);
        let task = animation::enter(page, entrance, cancellation, clock);
        task.on_completed(move || {
            if let Some(motion) = weak.upgrade() {
                motion.cancel_visit(visit);
            }
        });
    }

    fn cancel_visit(&self, visit: NavigationVisit) {
        if self.host.current(visit) {
            self.host.cancel();
        }
    }

    pub(crate) fn cancel(&self) {
        self.pending.borrow_mut().take();
        self.host.cancel();
        self.host.eligible.set(false);
    }
}

impl Drop for NavigationMotion {
    fn drop(&mut self) {
        self.host.cancel();
    }
}
