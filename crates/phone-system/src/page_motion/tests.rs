use super::animation::{PhoneExponentialOut, turnstile_transform};
use super::*;
use ferroui_base::Point;
use ferroui_base::animation::easings::IEasing;
use phone_domain::AppId;
use std::rc::Rc;

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
            let expected =
                (width * cos - width * 0.5) / (1.0 - z / (width * 1_000.0 / 480.0)) + width * 0.5;
            assert!(
                (right - expected).abs() < 0.001,
                "camera projection differs: {right} vs {expected}"
            );
        }
    }
}
