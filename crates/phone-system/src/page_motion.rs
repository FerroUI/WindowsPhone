//! Phone navigation motion, separate from application route and document state.
//!
//! The 350 ms / -80° forward / +50° backward entrance and exponential factor 6
//! follow Microsoft's Windows Phone Toolkit Turnstile storyboards. Perspective
//! uses FerroUI's projective transform and Silverlight's centered, 1000-unit
//! camera scaled from the 480-unit phone reference. Original-device frame
//! comparisons are still required to establish complete motion fidelity.
use ferroui_base::animation::easings::{Easing, IEasing, SharedEasing};
use ferroui_base::animation::{Animation, Cue, KeyFrame, SignedDuration};
use ferroui_base::media::{ITransform, Rotate3DTransform, TranslateTransform};
use ferroui_base::styling::Setter;
use ferroui_base::threading::CancellationToken;
use ferroui_base::{Ref, RelativePoint, RelativeUnit};
use ferroui_controls::Control;
use phone_domain::Route;
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::sync::Arc;

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
}

/// Identifies a route visit, not just its route. A delayed Loaded callback from
/// an old visit cannot consume the entrance of a later visit to the same page.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct NavigationVisit(u64);

struct PendingEntrance {
    visit: NavigationVisit,
    entrance: Entrance,
    width: f64,
}

impl NavigationMotion {
    pub(crate) fn reset(&self) {
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
        });
        visit
    }

    /// Loaded is deferred, whereas reactive route cleanup can render a page
    /// several times synchronously. Reconciliation replaces the callback on the
    /// retained control each time. Keep the plan until that callback actually
    /// runs; then even reattachment or a later same-route render cannot replay it.
    pub(crate) fn take_for_load(&self, visit: NavigationVisit, width: f64) -> Entrance {
        let mut pending = self.pending.borrow_mut();
        if !pending
            .as_ref()
            .is_some_and(|pending| pending.visit == visit)
        {
            return Entrance::None;
        }
        let plan = pending.take().expect("matching pending entrance");
        match plan.entrance {
            // A resize before Loaded preserves the fraction already dragged,
            // rather than carrying a stale pixel distance into the new layout.
            Entrance::Slide(distance) if width.is_finite() && width > 0.0 => {
                Entrance::Slide(distance * width / plan.width)
            }
            entrance => entrance,
        }
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

#[derive(Clone, Copy)]
struct PhoneExponentialOut;

impl IEasing for PhoneExponentialOut {
    fn ease(&self, progress: f64) -> f64 {
        let progress = progress.clamp(0.0, 1.0);
        1.0 - (((1.0 - progress) * 6.0).exp() - 1.0) / (6.0_f64.exp() - 1.0)
    }

    fn to_shared(&self) -> Arc<SharedEasing> {
        Arc::new(*self)
    }
}

pub(crate) fn enter(target: &Ref<Control>, entrance: Entrance) {
    if entrance == Entrance::Reveal {
        phone_controls::reveal(target);
        return;
    }
    if entrance == Entrance::None
        || !phone_controls::motion_enabled()
        || !target.is_attached_to_visual_tree()
        || target.render_transform().is_some()
    {
        return;
    }
    let old_origin = target.render_transform_origin();
    let animation = Animation::new();
    animation.set_duration(SignedDuration::from_seconds(0.35));
    animation.set_easing(Easing::new(PhoneExponentialOut));
    let transform: Rc<dyn ITransform> = match entrance {
        Entrance::Slide(distance) => {
            let transform = TranslateTransform::new();
            for (cue, value) in [(0.0, distance), (1.0, 0.0)] {
                let frame = KeyFrame::new();
                frame.set_cue(Cue::new(cue));
                frame
                    .setters()
                    .add(Setter::new(TranslateTransform::x_property(), value));
                animation.children().add(frame);
            }
            transform.into()
        }
        Entrance::Turnstile { backward } => {
            let transform = turnstile_transform(target.bounds().width);
            // PlaneProjection's camera stays centered even when the rotation
            // hinge is at the left edge. Shifting the render origin to the hinge
            // would also shift that camera and produce the wrong perspective.
            target.set_render_transform_origin(RelativePoint::new(
                0.5,
                0.5,
                RelativeUnit::Relative,
            ));
            for (cue, value) in [(0.0, if backward { 50.0 } else { -80.0 }), (1.0, 0.0)] {
                let frame = KeyFrame::new();
                frame.set_cue(Cue::new(cue));
                frame
                    .setters()
                    .add(Setter::new(Rotate3DTransform::angle_y_property(), value));
                animation.children().add(frame);
            }
            transform.into()
        }
        Entrance::None | Entrance::Reveal => return,
    };
    target.set_render_transform(Some(transform.clone()));
    let cancellation = CancellationToken::new();
    let cancel = cancellation.clone();
    let unloaded = target.add_handler(Control::unloaded_event(), move |_, _| cancel.cancel());
    let weak = target.downgrade();
    let task = animation.run_async(target, cancellation);
    task.on_completed(move || {
        if let Some(control) = weak.upgrade() {
            control.remove_handler(Control::unloaded_event(), unloaded);
            if control
                .render_transform()
                .is_some_and(|current| Rc::ptr_eq(&current, &transform))
            {
                control.set_render_transform(None);
                control.set_render_transform_origin(old_origin);
            }
        }
    });
}

fn turnstile_transform(width: f64) -> Ref<Rotate3DTransform> {
    let width = viewport_width(width);
    let transform = Rotate3DTransform::new();
    transform.set_center_x(-width * 0.5);
    transform.set_depth(width * (1_000.0 / 480.0));
    transform
}

#[cfg(test)]
mod tests {
    use super::*;
    use ferroui_base::Point;
    use phone_domain::AppId;

    fn load(motion: &NavigationMotion, route: &Route, width: f64) -> Entrance {
        motion.take_for_load(motion.observe(route, width), width)
    }

    #[test]
    fn starts_without_motion_and_unchanged_routes_do_not_replay() {
        let motion = NavigationMotion::default();
        assert_eq!(load(&motion, &Route::Start, 400.0), Entrance::None);
        assert_eq!(load(&motion, &Route::Start, 400.0), Entrance::None);
        assert_eq!(
            load(&motion, &Route::App(AppId::People), 400.0),
            Entrance::Turnstile { backward: false }
        );
        assert_eq!(
            load(&motion, &Route::App(AppId::People), 400.0),
            Entrance::None
        );
    }

    #[test]
    fn back_and_parent_navigation_have_the_opposite_turnstile() {
        let motion = NavigationMotion::default();
        let app = Route::App(AppId::Settings);
        load(&motion, &app, 400.0);
        load(&motion, &Route::Setting("theme".into()), 400.0);
        assert_eq!(
            load(&motion, &app, 400.0),
            Entrance::Turnstile { backward: true }
        );
        motion.begin_back(&app);
        assert_eq!(
            load(&motion, &Route::App(AppId::People), 400.0),
            Entrance::Turnstile { backward: true }
        );
    }

    #[test]
    fn dismissing_an_overlay_does_not_reverse_the_next_launch() {
        let motion = NavigationMotion::default();
        load(&motion, &Route::Start, 400.0);
        motion.begin_back(&Route::Start);
        motion.finish_back(&Route::Start, &Route::Start);
        assert_eq!(
            load(&motion, &Route::App(AppId::People), 400.0),
            Entrance::Turnstile { backward: false }
        );
    }

    #[test]
    fn start_and_apps_continue_the_manipulated_distance_in_both_directions() {
        let motion = NavigationMotion::default();
        load(&motion, &Route::Start, 400.0);
        motion.swiped(&Route::Start, -150.0, 400.0);
        assert_eq!(load(&motion, &Route::Apps, 320.0), Entrance::Slide(200.0));
        motion.swiped(&Route::Apps, 160.0, 320.0);
        assert_eq!(load(&motion, &Route::Start, 320.0), Entrance::Slide(-160.0));
        assert_eq!(load(&motion, &Route::Apps, 320.0), Entrance::Slide(320.0));
    }

    #[test]
    fn restoration_discards_pending_back_and_swipe_motion() {
        let motion = NavigationMotion::default();
        load(&motion, &Route::Start, 400.0);
        motion.begin_back(&Route::Start);
        motion.swiped(&Route::Start, -100.0, 400.0);
        motion.reset();
        assert_eq!(load(&motion, &Route::Start, 400.0), Entrance::None);
        assert_eq!(load(&motion, &Route::Apps, 400.0), Entrance::Slide(400.0));
    }

    #[test]
    fn preloaded_reconciliation_retains_the_visit_until_the_native_loaded_handler_runs() {
        use ferroui_base::interactivity::RoutedEventArgs;
        use ferroui_controls::StackPanel;
        use ferroui_view::prelude::*;

        let owner = Owner::new_root();
        let motion = Rc::new(NavigationMotion::default());
        let entered = Rc::new(RefCell::new(Vec::new()));
        let renders = Rc::new(Cell::new(0));
        let (route, reflow, panel) = owner.with(|| {
            let route = signal(Route::Start);
            let reflow = signal(0_u32);
            let query = signal(String::from("pending search"));
            let motion = motion.clone();
            let entered = entered.clone();
            let renders = renders.clone();
            let panel = stack_panel()
                .child(rerender(move || {
                    reflow.get();
                    let route = route.get();
                    let query_value = query.get();
                    renders.set(renders.get() + 1);
                    let visit = motion.observe(&route, 400.0);
                    // The same ordering as shell::view: preparing navigation,
                    // then route cleanup writes a signal read by this render.
                    if matches!(route, Route::App(_)) && !query_value.is_empty() {
                        query.set(String::new());
                    }
                    let motion = motion.clone();
                    let entered = entered.clone();
                    border()
                        .name("PhonePage")
                        .key(format!("page-{route:?}"))
                        .on_loaded(move |_| {
                            entered
                                .borrow_mut()
                                .push(motion.take_for_load(visit, 400.0));
                        })
                }))
                .into_element()
                .mount()
                .cast::<StackPanel>()
                .unwrap();
            (route, reflow, panel)
        });
        let initial = panel.children().get(0);
        route.set(Route::App(AppId::People));
        assert_eq!(renders.get(), 3, "route cleanup rerenders before Loaded");
        let page = panel.children().get(0);
        assert!(!initial.ptr_eq(&page), "a new route mounts its keyed page");
        reflow.set(1);
        assert!(page.ptr_eq(&panel.children().get(0)));
        assert!(
            entered.borrow().is_empty(),
            "rendering cannot consume motion"
        );
        page.raise_event(&RoutedEventArgs::with_event_and_source(
            Control::loaded_event(),
            page.clone(),
        ));
        assert_eq!(
            *entered.borrow(),
            [Entrance::Turnstile { backward: false }],
            "the reconciler's latest native handler consumes the original plan"
        );
        reflow.set(2);
        page.raise_event(&RoutedEventArgs::with_event_and_source(
            Control::loaded_event(),
            page.clone(),
        ));
        assert_eq!(entered.borrow()[1], Entrance::None, "no same-visit replay");
        owner.dispose();
    }

    #[test]
    fn stale_loaded_callbacks_cannot_consume_a_later_visit_or_restored_document() {
        let motion = NavigationMotion::default();
        motion.observe(&Route::Start, 400.0);
        let app = Route::App(AppId::People);
        let stale = motion.observe(&app, 400.0);
        motion.observe(&Route::Start, 400.0);
        let current = motion.observe(&app, 400.0);
        assert_ne!(stale, current, "returning to the same route is a new visit");
        assert_eq!(motion.take_for_load(stale, 400.0), Entrance::None);
        assert_eq!(
            motion.take_for_load(current, 400.0),
            Entrance::Turnstile { backward: false }
        );
        let before_reset = motion.observe(&Route::Start, 400.0);
        motion.reset();
        motion.observe(&Route::Start, 400.0);
        assert_eq!(motion.take_for_load(before_reset, 400.0), Entrance::None);
        assert_eq!(
            load(&motion, &app, 400.0),
            Entrance::Turnstile { backward: false }
        );
    }

    #[test]
    fn preloaded_resize_retains_the_dragged_fraction_and_consumes_it_once() {
        let motion = NavigationMotion::default();
        motion.observe(&Route::Start, 400.0);
        motion.swiped(&Route::Start, -150.0, 400.0);
        let visit = motion.observe(&Route::Apps, 400.0);
        assert_eq!(motion.observe(&Route::Apps, 320.0), visit);
        assert_eq!(motion.take_for_load(visit, 320.0), Entrance::Slide(200.0));
        assert_eq!(motion.take_for_load(visit, 400.0), Entrance::None);
    }

    #[test]
    fn exponential_matches_phone_factor_and_settles_exactly() {
        let curve = PhoneExponentialOut;
        assert_eq!(curve.ease(0.0), 0.0);
        assert_eq!(curve.ease(1.0), 1.0);
        assert!((curve.ease(0.5) - 0.9525741268224333).abs() < 1e-12);
        let mut previous = 0.0;
        for step in 0..=100 {
            let value = curve.ease(step as f64 / 100.0);
            assert!(value >= previous && value <= 1.0);
            previous = value;
        }
    }

    #[test]
    fn native_projection_keeps_the_left_hinge_and_centered_camera() {
        for width in [320.0, 400.0, 480.0] {
            let transform = turnstile_transform(width);
            for angle in [-80.0_f64, -25.0, 50.0, 0.0] {
                transform.set_angle_y(angle);
                let matrix = transform.value();
                let left = matrix.transform(Point::new(-width * 0.5, 0.0)).x + width * 0.5;
                assert!(left.abs() < 0.001, "left hinge moved: {left}");
                let right = matrix.transform(Point::new(width * 0.5, 0.0)).x + width * 0.5;
                // Independent camera projection of the rotated right edge.
                let (sin, cos) = angle.to_radians().sin_cos();
                let z = -width * sin;
                let expected = (width * cos - width * 0.5) / (1.0 - z / (width * 1_000.0 / 480.0))
                    + width * 0.5;
                assert!(
                    (right - expected).abs() < 0.001,
                    "camera projection differs: {right} vs {expected}"
                );
            }
        }
    }
}
