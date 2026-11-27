use super::*;
use ferroui_base::animation::{PlayState, SignedDuration};
use ferroui_base::interactivity::RoutedEventArgs;
use ferroui_base::media::{DrawingContext, IImage, Rotate3DTransform, TranslateTransform};
use ferroui_base::reactive::{IDisposable, IObservable, IObserver};
use ferroui_base::{Rect, Size};
use ferroui_reactive::subjects::Subject;
use phone_domain::{AppId, Route};

struct TestClock {
    ticks: Rc<Subject<SignedDuration>>,
    state: Cell<PlayState>,
}
impl TestClock {
    fn new() -> Rc<Self> {
        Rc::new(Self {
            ticks: Subject::new(),
            state: Cell::new(PlayState::Run),
        })
    }
    fn step(&self, seconds: f64) {
        self.ticks.on_next(SignedDuration::from_seconds(seconds));
    }
}
impl IObservable<SignedDuration> for TestClock {
    fn subscribe(&self, observer: Rc<dyn IObserver<SignedDuration>>) -> Rc<dyn IDisposable> {
        self.ticks.subscribe(observer)
    }
}
impl IClock for TestClock {
    fn play_state(&self) -> PlayState {
        self.state.get()
    }
    fn set_play_state(&self, state: PlayState) {
        self.state.set(state);
    }
}

struct Snapshot(Rc<Cell<usize>>);
impl Drop for Snapshot {
    fn drop(&mut self) {
        self.0.set(self.0.get() + 1);
    }
}
impl IImage for Snapshot {
    fn size(&self) -> Size {
        Size::new(320.0, 660.0)
    }
    fn draw(&self, _: &mut DrawingContext<'_>, _: Rect, _: Rect) {}
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}
fn preview(dropped: &Rc<Cell<usize>>) -> Preview {
    Preview {
        image: Rc::new(Snapshot(dropped.clone())),
        logical_size: Size::new(320.0, 660.0),
    }
}
fn page() -> Ref<Control> {
    let page = Border::new();
    page.measure(Size::new(320.0, 660.0));
    page.arrange(Rect::new(0.0, 0.0, 320.0, 660.0));
    page.upcast()
}
fn setup() -> (
    Rc<NavigationMotion>,
    Ref<Border>,
    Ref<Image>,
    Ref<Control>,
    Rc<TestClock>,
) {
    setup_at(Route::App(AppId::Settings))
}
fn setup_at(
    route: Route,
) -> (
    Rc<NavigationMotion>,
    Ref<Border>,
    Ref<Image>,
    Ref<Control>,
    Rc<TestClock>,
) {
    let motion = Rc::new(NavigationMotion::default());
    let outgoing = Border::new();
    let image = Image::new();
    outgoing.set_child(image.clone());
    outgoing.measure(Size::new(320.0, 660.0));
    outgoing.arrange(Rect::new(0.0, 0.0, 320.0, 660.0));
    motion.attach_outgoing(&outgoing);
    let initial = page();
    let visit = motion.prepare(&route, 320.0, 740.0, true);
    motion.loaded(&initial, visit);
    (motion, outgoing, image, initial, TestClock::new())
}

fn slide_x(page: &Ref<Control>) -> f64 {
    page.render_transform()
        .and_then(|value| {
            value.as_object().and_then(|value| {
                value
                    .downcast_ref::<TranslateTransform>()
                    .map(|translation| translation.x())
            })
        })
        .expect("native slide translation")
}

#[test]
fn shell_paging_continues_the_pointer_with_two_adjacent_surfaces_and_one_clock() {
    for (from, to, direction) in [
        (Route::Start, Route::Apps, 1.0),
        (Route::Apps, Route::Start, -1.0),
    ] {
        for dragged in [0.0, 110.0, 250.0] {
            let (motion, outgoing, image, _initial, clock) = setup_at(from.clone());
            let dropped = Rc::new(Cell::new(0));
            if dragged > 0.0 {
                motion.swiped(&from, -direction * dragged, 320.0);
            }
            let visit =
                motion.prepare_with_capture(&to, 320.0, 740.0, true, || Some(preview(&dropped)));
            let incoming = page();
            motion.start(
                &incoming,
                motion.take_plan(visit, 320.0).unwrap(),
                Some(clock.clone()),
            );
            let outgoing = outgoing.upcast::<Control>();
            assert!((slide_x(&outgoing) + direction * dragged).abs() < 1e-9);
            assert!((slide_x(&incoming) - direction * (320.0 - dragged)).abs() < 1e-9);
            assert!(image.source().is_some());
            assert!(incoming.is_enabled() && !incoming.is_hit_test_visible());
            assert_eq!(
                ferroui_base::input::KeyboardNavigation::get_tab_navigation(&incoming),
                ferroui_base::input::KeyboardNavigationMode::None
            );
            for time in [0.0, 0.04, 0.12, 0.25, 0.349] {
                clock.step(time);
                assert!(
                    (slide_x(&incoming) - slide_x(&outgoing) - direction * 320.0).abs() < 1e-8,
                    "the moving edges remain adjacent at {time}, drag {dragged}"
                );
                assert_eq!(incoming.opacity(), 1.0);
                assert_eq!(outgoing.opacity(), 1.0);
                assert!(image.source().is_some());
                assert_eq!(dropped.get(), 0);
            }
            clock.step(0.35);
            assert!(incoming.render_transform().is_none());
            assert!(outgoing.render_transform().is_none());
            assert!(image.source().is_none());
            assert_eq!(dropped.get(), 1);
            assert_eq!(outgoing.opacity(), 0.0);
            assert!(incoming.is_enabled() && incoming.is_hit_test_visible());
            assert_eq!(
                ferroui_base::input::KeyboardNavigation::get_tab_navigation(&incoming),
                ferroui_base::input::KeyboardNavigationMode::Continue
            );
            assert!(motion.host.active.borrow().is_none());
        }
    }
}

#[test]
fn interrupted_shell_paging_releases_the_image_without_late_input_or_motion() {
    for reason in ["privacy", "resize", "unload", "reset", "reverse"] {
        let (motion, outgoing, image, _initial, clock) = setup_at(Route::Start);
        let dropped = Rc::new(Cell::new(0));
        motion.swiped(&Route::Start, -110.0, 320.0);
        let visit = motion
            .prepare_with_capture(&Route::Apps, 320.0, 740.0, true, || Some(preview(&dropped)));
        let incoming = page();
        motion.start(
            &incoming,
            motion.take_plan(visit, 320.0).unwrap(),
            Some(clock.clone()),
        );
        clock.step(0.0);
        clock.step(0.1);
        match reason {
            "privacy" => {
                motion.prepare(&Route::Apps, 320.0, 740.0, false);
            }
            "resize" => {
                motion.prepare(&Route::Apps, 480.0, 740.0, true);
            }
            "unload" => incoming.raise_event(&RoutedEventArgs::with_event_and_source(
                Control::unloaded_event(),
                incoming.clone(),
            )),
            "reverse" => {
                let back = motion.prepare_with_capture(&Route::Start, 320.0, 740.0, true, || {
                    panic!("an unfinished page cannot become a snapshot")
                });
                let plan = motion.take_plan(back, 320.0).unwrap();
                assert_eq!(plan.entrance, Entrance::Slide(-320.0));
                assert!(plan.outgoing.is_none());
            }
            _ => motion.reset(),
        }
        clock.step(1.0);
        assert!(image.source().is_none(), "{reason}");
        assert_eq!(dropped.get(), 1, "{reason}");
        assert_eq!(outgoing.opacity(), 0.0, "{reason}");
        assert_eq!(incoming.opacity(), 1.0, "{reason}");
        assert!(
            incoming.is_enabled() && incoming.is_hit_test_visible(),
            "{reason}"
        );
        assert!(incoming.render_transform().is_none(), "{reason}");
        assert!(outgoing.render_transform().is_none(), "{reason}");
        assert!(motion.host.active.borrow().is_none(), "{reason}");
    }
}

#[test]
fn completed_shell_pan_has_no_redundant_capture_or_settle() {
    let (motion, outgoing, image, _initial, clock) = setup_at(Route::Start);
    motion.swiped(&Route::Start, -320.0, 320.0);
    let visit = motion.prepare_with_capture(&Route::Apps, 320.0, 740.0, true, || {
        panic!("a page already at its destination does not need an outgoing image")
    });
    let incoming = page();
    motion.start(
        &incoming,
        motion.take_plan(visit, 320.0).unwrap(),
        Some(clock.clone()),
    );
    assert!(incoming.render_transform().is_none());
    assert!(incoming.is_enabled() && incoming.is_hit_test_visible());
    assert!(image.source().is_none() && outgoing.render_transform().is_none());
    assert!(motion.host.active.borrow().is_none());
}

#[test]
fn resize_before_shell_paging_load_releases_the_snapshot_and_preserves_pan_fraction() {
    let (motion, outgoing, image, _initial, clock) = setup_at(Route::Start);
    let dropped = Rc::new(Cell::new(0));
    motion.swiped(&Route::Start, -160.0, 320.0);
    let visit =
        motion.prepare_with_capture(&Route::Apps, 320.0, 740.0, true, || Some(preview(&dropped)));
    assert_eq!(motion.prepare(&Route::Apps, 480.0, 820.0, true), visit);
    assert_eq!(dropped.get(), 1);
    let plan = motion.take_plan(visit, 480.0).unwrap();
    assert_eq!(plan.entrance, Entrance::Slide(240.0));
    assert!(plan.outgoing.is_none());
    let incoming = page();
    motion.start(&incoming, plan, Some(clock.clone()));
    assert_eq!(slide_x(&incoming), 240.0);
    assert!(incoming.is_enabled() && !incoming.is_hit_test_visible());
    assert_eq!(
        ferroui_base::input::KeyboardNavigation::get_tab_navigation(&incoming),
        ferroui_base::input::KeyboardNavigationMode::None
    );
    assert!(image.source().is_none());
    assert_eq!(outgoing.opacity(), 0.0);
    clock.step(0.0);
    clock.step(0.35);
    assert!(incoming.render_transform().is_none());
    assert!(incoming.is_enabled() && incoming.is_hit_test_visible());
    assert_eq!(
        ferroui_base::input::KeyboardNavigation::get_tab_navigation(&incoming),
        ferroui_base::input::KeyboardNavigationMode::Continue
    );
}
fn angle(page: &Ref<Control>) -> f64 {
    page.render_transform()
        .and_then(|value| {
            value.as_object().and_then(|value| {
                value
                    .downcast_ref::<Rotate3DTransform>()
                    .map(|rotation| rotation.angle_y())
            })
        })
        .expect("native turnstile projection")
}

#[test]
fn outgoing_and_incoming_have_separate_native_clocks_opacity_and_input_ownership() {
    for backward in [false, true] {
        let (motion, outgoing, image, _initial, clock) = setup();
        let dropped = Rc::new(Cell::new(0));
        let route = Route::Setting("ringtones+sounds".into());
        if backward {
            motion.begin_back(&Route::App(AppId::Settings));
        }
        let visit =
            motion.prepare_with_capture(&route, 320.0, 740.0, true, || Some(preview(&dropped)));
        let incoming = page();
        let plan = motion.take_plan(visit, 320.0).unwrap();
        assert!(plan.outgoing.is_some());
        motion.start(&incoming, plan, Some(clock.clone()));
        assert!(image.source().is_some());
        assert!(!outgoing.is_hit_test_visible() && !outgoing.is_enabled());
        assert_eq!(incoming.opacity(), 0.0);
        assert!(!incoming.is_enabled() && !incoming.is_hit_test_visible());
        clock.step(0.0);
        clock.step(0.125);
        assert!(angle(&outgoing.clone().upcast()) * if backward { -1.0 } else { 1.0 } > 1.0);
        assert_eq!(outgoing.opacity(), 1.0);
        assert!(
            incoming.render_transform().is_none(),
            "incoming waits for retirement"
        );
        clock.step(0.24);
        assert!((outgoing.opacity() - 1.0).abs() < 1e-9);
        clock.step(0.245);
        assert!(
            (outgoing.opacity() - 0.5).abs() < 1e-9,
            "the last 10 ms are a separate linear opacity track"
        );
        clock.step(0.25);
        assert!(image.source().is_none());
        assert_eq!(dropped.get(), 1, "image released before the incoming phase");
        assert_eq!(outgoing.opacity(), 0.0);
        assert!(incoming.is_enabled() && incoming.is_hit_test_visible());
        clock.step(0.251);
        assert!((angle(&incoming) - if backward { 50.0 } else { -80.0 }).abs() < 1e-9);
        clock.step(0.256);
        assert!((incoming.opacity() - 0.5).abs() < 1e-8);
        clock.step(0.426);
        assert!(angle(&incoming).abs() > 0.0);
        assert_eq!(incoming.opacity(), 1.0);
        clock.step(0.602);
        assert!(incoming.render_transform().is_none());
        assert!(outgoing.render_transform().is_none());
        assert!(motion.host.active.borrow().is_none());
    }
}

#[test]
fn reflow_does_not_recapture_or_restart_and_rapid_back_retires_the_old_visit() {
    let (motion, outgoing, image, _initial, clock) = setup();
    let dropped = Rc::new(Cell::new(0));
    let detail = Route::Setting("ringtones+sounds".into());
    let visit =
        motion.prepare_with_capture(&detail, 320.0, 740.0, true, || Some(preview(&dropped)));
    let incoming = page();
    motion.start(
        &incoming,
        motion.take_plan(visit, 320.0).unwrap(),
        Some(clock.clone()),
    );
    clock.step(0.0);
    clock.step(0.1);
    let reflow = motion.prepare_with_capture(&detail, 320.0, 740.0, true, || {
        panic!("same route recaptured")
    });
    assert_eq!(visit, reflow);
    assert!(motion.host.current(visit));
    motion.begin_back(&detail);
    let back =
        motion.prepare_with_capture(&Route::App(AppId::Settings), 320.0, 740.0, true, || {
            panic!("unfinished incoming page captured")
        });
    assert_ne!(visit, back);
    assert_eq!(dropped.get(), 1);
    assert!(image.source().is_none() && outgoing.render_transform().is_none());
    assert!(incoming.is_enabled() && incoming.is_hit_test_visible());
    let back_page = page();
    let plan = motion.take_plan(back, 320.0).unwrap();
    assert!(plan.outgoing.is_none());
    assert_eq!(plan.entrance, Entrance::Turnstile { backward: true });
    motion.start(&back_page, plan, Some(clock.clone()));
    clock.step(0.11);
    clock.step(0.46);
    clock.step(1.0);
    assert!(back_page.render_transform().is_none());
    assert!(
        image.source().is_none(),
        "retired completion cannot reattach an image"
    );
    assert_eq!(outgoing.opacity(), 0.0);
}

#[test]
fn privacy_resize_unload_and_reset_release_the_image_and_restore_native_input() {
    for reason in ["privacy", "resize", "unload", "reset"] {
        let (motion, outgoing, image, _initial, clock) = setup();
        let dropped = Rc::new(Cell::new(0));
        let route = Route::Setting("ringtones+sounds".into());
        let visit =
            motion.prepare_with_capture(&route, 320.0, 740.0, true, || Some(preview(&dropped)));
        let incoming = page();
        motion.start(
            &incoming,
            motion.take_plan(visit, 320.0).unwrap(),
            Some(clock.clone()),
        );
        clock.step(0.0);
        clock.step(0.1);
        match reason {
            "privacy" => {
                motion.prepare(&route, 320.0, 740.0, false);
            }
            "resize" => {
                motion.prepare(&route, 480.0, 740.0, true);
            }
            "unload" => incoming.raise_event(&RoutedEventArgs::with_event_and_source(
                Control::unloaded_event(),
                incoming.clone(),
            )),
            _ => motion.reset(),
        }
        clock.step(1.0);
        assert_eq!(dropped.get(), 1, "{reason}");
        assert!(image.source().is_none(), "{reason}");
        assert_eq!(outgoing.opacity(), 0.0, "{reason}");
        assert_eq!(incoming.opacity(), 1.0, "{reason}");
        assert!(
            incoming.is_enabled() && incoming.is_hit_test_visible(),
            "{reason}"
        );
        assert!(incoming.render_transform().is_none(), "{reason}");
        assert!(motion.host.active.borrow().is_none(), "{reason}");
    }
}

#[test]
fn unavailable_capture_enters_without_a_phantom_outgoing_phase() {
    let (motion, outgoing, image, _initial, clock) = setup();
    let visit =
        motion.prepare_with_capture(&Route::Setting("theme".into()), 320.0, 740.0, true, || None);
    let incoming = page();
    motion.start(
        &incoming,
        motion.take_plan(visit, 320.0).unwrap(),
        Some(clock.clone()),
    );
    assert!(image.source().is_none());
    assert_eq!(outgoing.opacity(), 0.0);
    assert!(incoming.is_enabled() && incoming.is_hit_test_visible());
    clock.step(0.0);
    assert_eq!(angle(&incoming), -80.0);
    clock.step(0.35);
    assert!(incoming.render_transform().is_none());
}

#[test]
fn resize_before_loaded_drops_only_the_stale_image_and_preserves_the_incoming_visit() {
    let (motion, outgoing, image, _initial, clock) = setup();
    let dropped = Rc::new(Cell::new(0));
    let route = Route::Setting("theme".into());
    let visit = motion.prepare_with_capture(&route, 320.0, 740.0, true, || Some(preview(&dropped)));
    assert_eq!(dropped.get(), 0);
    assert!(
        motion.host.active.borrow().is_none(),
        "Loaded has not started motion"
    );
    assert_eq!(
        motion.prepare_with_capture(&route, 480.0, 820.0, true, || {
            panic!("resize cannot recapture an old page")
        }),
        visit
    );
    assert_eq!(
        dropped.get(),
        1,
        "the stale pending image is released immediately"
    );
    let plan = motion
        .take_plan(visit, 480.0)
        .expect("same incoming visit survives");
    assert!(plan.outgoing.is_none());
    assert_eq!(plan.entrance, Entrance::Turnstile { backward: false });
    let incoming = page();
    incoming.measure(Size::new(480.0, 740.0));
    incoming.arrange(Rect::new(0.0, 0.0, 480.0, 740.0));
    motion.start(&incoming, plan, Some(clock.clone()));
    clock.step(0.0);
    assert_eq!(angle(&incoming), -80.0);
    assert!(image.source().is_none());
    assert_eq!(outgoing.opacity(), 0.0);
    assert!(incoming.is_enabled() && incoming.is_hit_test_visible());
    clock.step(0.35);
    assert!(incoming.render_transform().is_none());
    assert!(
        motion.take_plan(visit, 480.0).is_none(),
        "the visit cannot replay"
    );
}

#[test]
fn retained_declarative_reflow_preserves_animation_owned_opacity_input_and_source() {
    use ferroui_controls::StackPanel;
    use ferroui_view::prelude::*;

    let (motion, _old_outgoing, _old_image, _initial, clock) = setup();
    let owner = Owner::new_root();
    let (reflow, root) = owner.with(|| {
        let reflow = signal(0_u32);
        let attach = motion.clone();
        let root = stack_panel().child(rerender(move || {
            reflow.get();
            let attach = attach.clone();
            rsx! {Grid {
                Border {name:"PhonePage",opacity:1.0,is_enabled:true,is_hit_test_visible:true}
                Border {name:"PhoneOutgoingPage",opacity:0.0,is_enabled:false,is_hit_test_visible:false,
                    with:move |border|attach.attach_outgoing(border),
                    Image {name:"PhoneOutgoingImage",is_hit_test_visible:false}
                }
            }}
        })).into_element().mount().cast::<StackPanel>().unwrap();
        (reflow, root)
    });
    let incoming = root
        .find_descendant_of_type_where::<Border>(false, |part| part.has_name("PhonePage"))
        .unwrap()
        .upcast::<Control>();
    let outgoing = root
        .find_descendant_of_type_where::<Border>(false, |part| part.has_name("PhoneOutgoingPage"))
        .unwrap();
    let image = outgoing.child().unwrap().cast::<Image>().unwrap();
    for surface in [incoming.clone(), outgoing.clone().upcast::<Control>()] {
        surface.measure(Size::new(320.0, 660.0));
        surface.arrange(Rect::new(0.0, 0.0, 320.0, 660.0));
    }
    let dropped = Rc::new(Cell::new(0));
    let route = Route::Setting("theme".into());
    let visit = motion.prepare_with_capture(&route, 320.0, 740.0, true, || Some(preview(&dropped)));
    motion.start(
        &incoming,
        motion.take_plan(visit, 320.0).unwrap(),
        Some(clock.clone()),
    );
    clock.step(0.0);
    clock.step(0.1);
    let outgoing_angle = angle(&outgoing.clone().upcast());
    reflow.set(1);
    assert!(
        incoming.ptr_eq(
            &root
                .find_descendant_of_type_where::<Border>(false, |part| part.has_name("PhonePage"))
                .unwrap()
                .upcast::<Control>()
        )
    );
    assert_eq!(
        incoming.opacity(),
        0.0,
        "unchanged opacity literal must not reveal the incoming page"
    );
    assert!(!incoming.is_enabled() && !incoming.is_hit_test_visible());
    assert_eq!(
        outgoing.opacity(),
        1.0,
        "unchanged outgoing opacity literal must not hide its image"
    );
    assert_eq!(angle(&outgoing.clone().upcast()), outgoing_angle);
    assert!(image.source().is_some());
    assert_eq!(dropped.get(), 0);
    clock.step(0.25);
    clock.step(0.251);
    let incoming_angle = angle(&incoming);
    reflow.set(2);
    assert_eq!(
        angle(&incoming),
        incoming_angle,
        "reconciliation does not restart the native rotation"
    );
    assert!(incoming.is_enabled() && incoming.is_hit_test_visible());
    assert!(image.source().is_none());
    assert_eq!(dropped.get(), 1);
    clock.step(0.7);
    assert!(incoming.render_transform().is_none());
    assert!(outgoing.render_transform().is_none());
    assert_eq!(outgoing.opacity(), 0.0);
    owner.dispose();
}
