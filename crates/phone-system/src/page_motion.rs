//! Phone page choreography keeps navigation/lifecycle changes synchronous.
//! A bounded, non-interactive image of the outgoing settled page survives only
//! for its 250 ms turnstile; the destination remains the sole live app tree.
//! Incoming motion then uses the original Toolkit 350 ms turnstile. Host/image
//! unavailability falls back to incoming-only motion; privacy changes cancel.
mod animation;
mod choreography;
mod snapshot;
#[cfg(test)]
mod tests;

use crate::previews::Preview;
use phone_domain::Route;
use std::cell::{Cell, RefCell};

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) enum Entrance {
    #[default]
    None,
    Slide(f64),
    Turnstile {
        backward: bool,
    },
    Reveal,
}

#[derive(Default)]
pub(crate) struct NavigationMotion {
    route: RefCell<Option<Route>>,
    generation: Cell<u64>,
    pending: RefCell<Option<PendingEntrance>>,
    back_from: RefCell<Option<Route>>,
    swipe: RefCell<Option<(Route, f64, f64)>>,
    host: choreography::TransitionHost,
}

/// Identifies a route visit, not just its route. A delayed Loaded callback from
/// an old visit cannot consume the entrance of a later visit to the same page.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct NavigationVisit(u64);

struct PendingEntrance {
    visit: NavigationVisit,
    entrance: Entrance,
    width: f64,
    outgoing: Option<Preview>,
}

impl NavigationMotion {
    pub(crate) fn reset(&self) {
        self.host.cancel();
        self.host.page.borrow_mut().take();
        self.host.eligible.set(false);
        self.route.borrow_mut().take();
        self.pending.borrow_mut().take();
        self.back_from.borrow_mut().take();
        self.swipe.borrow_mut().take();
    }

    pub(crate) fn begin_back(&self, route: &Route) {
        *self.back_from.borrow_mut() = Some(route.clone());
    }

    pub(crate) fn finish_back(&self, from: &Route, to: &Route) {
        // Dismissing a menu, picker or keyboard must not reverse the animation
        // of an unrelated future forward navigation.
        if from == to {
            self.back_from.borrow_mut().take();
        }
    }

    pub(crate) fn swiped(&self, route: &Route, delta: f64, width: f64) {
        if delta.is_finite() && width.is_finite() && width > 0.0 {
            *self.swipe.borrow_mut() = Some((route.clone(), delta, width));
        }
    }

    pub(crate) fn observe(&self, route: &Route, width: f64) -> NavigationVisit {
        let previous = self.route.replace(Some(route.clone()));
        if previous.as_ref() == Some(route) {
            return NavigationVisit(self.generation.get());
        }
        let visit = NavigationVisit(self.generation.get().wrapping_add(1));
        self.generation.set(visit.0);
        let width = viewport_width(width);
        let entrance = self.plan(previous, route, width);
        *self.pending.borrow_mut() = Some(PendingEntrance {
            visit,
            entrance,
            width,
            outgoing: None,
        });
        visit
    }

    /// Loaded is deferred, whereas reactive route cleanup can render a page
    /// several times synchronously. Reconciliation replaces the callback on the
    /// retained control each time. Keep the plan until that callback actually
    /// runs; then even reattachment or a later same-route render cannot replay it.
    #[cfg(test)]
    pub(crate) fn take_for_load(&self, visit: NavigationVisit, width: f64) -> Entrance {
        self.take_plan(visit, width)
            .map_or(Entrance::None, |plan| plan.entrance)
    }

    fn take_plan(&self, visit: NavigationVisit, width: f64) -> Option<PendingEntrance> {
        let mut pending = self.pending.borrow_mut();
        if !pending
            .as_ref()
            .is_some_and(|pending| pending.visit == visit)
        {
            return None;
        }
        let mut plan = pending.take().expect("matching pending entrance");
        // A resize before Loaded preserves the manipulated fraction.
        if let Entrance::Slide(distance) = plan.entrance {
            if width.is_finite() && width > 0.0 {
                plan.entrance = Entrance::Slide(distance * width / plan.width);
            }
        }
        Some(plan)
    }

    fn plan(&self, previous: Option<Route>, route: &Route, width: f64) -> Entrance {
        let explicit_back = self.back_from.borrow_mut().take() == previous;
        let swipe = self.swipe.borrow_mut().take();
        let Some(previous) = previous else {
            // The initial frame must be immediately usable, including reduced
            // motion, app restoration and startup verification.
            return Entrance::None;
        };
        let slide_direction = match (&previous, route) {
            (Route::Start | Route::Folder(_), Route::Apps) => Some(1.0),
            (Route::Apps, Route::Start | Route::Folder(_)) => Some(-1.0),
            _ => None,
        };
        if let Some(direction) = slide_direction {
            let remaining = swipe
                .filter(|(from, delta, _)| *from == previous && delta.signum() == -direction)
                .map(|(_, delta, old_width)| {
                    // Carry the fraction manipulated before release into the
                    // new layout, including an orientation/viewport change.
                    width * (1.0 - delta.abs() / old_width).clamp(0.0, 1.0)
                })
                .unwrap_or(width);
            return Entrance::Slide(direction * remaining);
        }
        if matches!(route, Route::Lock | Route::Notifications | Route::Switcher)
            || matches!(
                previous,
                Route::Lock | Route::Notifications | Route::Switcher
            )
        {
            return Entrance::Reveal;
        }
        let parent = matches!(route, Route::App(app) if previous.app() == Some(*app));
        Entrance::Turnstile {
            backward: explicit_back || parent || matches!(route, Route::Start),
        }
    }
}

fn viewport_width(width: f64) -> f64 {
    if width.is_finite() && width > 0.0 {
        width
    } else {
        400.0
    }
}
