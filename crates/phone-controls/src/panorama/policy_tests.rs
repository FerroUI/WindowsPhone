use super::*;
use ferroui_base::animation::{IClock, PlayState};
use ferroui_base::data::BindingPriority;
use ferroui_base::input::{SwipeGestureEndedEventArgs, SwipeGestureEventArgs};
use ferroui_base::reactive::{IDisposable, IObservable, IObserver, LightweightSubject};
use ferroui_base::threading::{CancellationToken, Dispatcher};
use ferroui_base::{Ref, Vector};
use ferroui_controls::{Application, Control, Grid, ScrollViewer, Window};
use ferroui_headless::FerroHeadlessPlatform;
use std::{cell::Cell, time::Duration};

struct Clock {
    ticks: LightweightSubject<SignedDuration>,
    seconds: Cell<f64>,
    play: Cell<PlayState>,
}

impl Clock {
    fn new() -> Rc<Self> {
        Rc::new(Self {
            ticks: LightweightSubject::new(),
            seconds: Cell::new(0.0),
            play: Cell::new(PlayState::Run),
        })
    }

    fn advance(&self, frames: usize) {
        for _ in 0..frames {
            self.seconds.set(self.seconds.get() + 0.02);
            self.ticks
                .on_next(SignedDuration::from_seconds(self.seconds.get()));
            frame();
        }
    }
}

impl IObservable<SignedDuration> for Clock {
    fn subscribe(&self, observer: Rc<dyn IObserver<SignedDuration>>) -> Rc<dyn IDisposable> {
        self.ticks.subscribe(observer)
    }
}

impl IClock for Clock {
    fn play_state(&self) -> PlayState {
        self.play.get()
    }

    fn set_play_state(&self, value: PlayState) {
        self.play.set(value);
    }
}

fn frame() {
    Dispatcher::ui_thread().run_jobs(None);
    FerroHeadlessPlatform::force_render_timer_tick(1);
    Dispatcher::ui_thread().run_jobs(None);
}

fn scene(motion: bool, run: impl FnOnce(Ref<Carousel>, Signal<usize>, Rc<Clock>) + Send + 'static) {
    let session = super::tests::session();
    session
        .dispatch(
            move || {
                struct RestoreMotion;
                impl Drop for RestoreMotion {
                    fn drop(&mut self) {
                        crate::set_motion_enabled(true);
                    }
                }
                let _restore = RestoreMotion;
                crate::set_motion_enabled(motion);
                crate::install_theme(&Application::current().unwrap());
                let selection = signal(0usize);
                let pages = ["all", "favorites", "groups"]
                    .into_iter()
                    .enumerate()
                    .map(|(index, label)| {
                        let mut rows = stack_panel();
                        for n in 0..30 {
                            rows = rows.child(rsx! {Button {
                                content:format!("Contact {n}"),height:60.0
                            }});
                        }
                        PanoramaPage::new(
                            label,
                            rsx! {ScrollViewer {
                                name:format!("PolicyScroll{index}"),{rows}
                            }},
                        )
                    })
                    .collect();
                let (control, owner) =
                    Element::from(panorama_pages("PolicyPanorama", "people", selection, pages))
                        .mount_root();
                let host = control.cast::<Carousel>().unwrap();
                let clock = Clock::new();
                let window = Window::new();
                let _mounted = super::tests::MountedWindow::new(window.clone(), owner);
                window.set_clock(Some(clock.clone()));
                window.set_width(400.0);
                window.set_height(720.0);
                window.set_content(Some(Control::boxed(control)));
                window.show();
                for _ in 0..5 {
                    frame();
                }
                run(host, selection, clock);
            },
            CancellationToken::none(),
        )
        .wait_timeout(Duration::from_secs(20))
        .expect("retained Panorama policy test finishes");
}

fn child<T: ferroui_base::ObjectType>(root: &Control, name: &str) -> Ref<T> {
    root.find_descendant_of_type_where::<Control>(false, |control| control.has_name(name))
        .and_then(|control| control.cast::<T>())
        .expect(name)
}

fn input_mode(host: &Carousel, native: bool) {
    assert_eq!(host.is_swipe_enabled(), native);
    assert_eq!(host.classes().contains("phone-native-input"), native);
    assert_eq!(host.classes().contains("phone-panorama-static"), !native);
    assert_eq!(
        host.items_panel_root()
            .unwrap()
            .gesture_recognizers()
            .count(),
        usize::from(native),
        "suppressed input has no native swipe recognizer"
    );
}

#[test]
fn retained_policy_preserves_template_scroll_and_live_caller_bindings() {
    scene(true, |host, selection, _| {
        let root = child::<Grid>(&host, "PanoramaRoot");
        let panel = host.items_panel_root().unwrap();
        let scroll = child::<ScrollViewer>(&host, "PolicyScroll0");
        let transition = host.page_transition().unwrap();
        scroll.set_offset(Vector::new(0.0, 240.0));
        frame();
        let saved = scroll.offset().y;
        assert!(saved > 0.0);

        for enabled in [false, true, false, true] {
            crate::set_motion_enabled(enabled);
            frame();
            input_mode(&host, enabled);
            assert!(!host.is_swiping());
            assert_eq!(selection.get_untracked(), 0);
            assert!(Ref::ptr_eq(&root, &child::<Grid>(&host, "PanoramaRoot")));
            assert!(Ref::ptr_eq(&panel, &host.items_panel_root().unwrap()));
            assert!(Ref::ptr_eq(
                &scroll,
                &child::<ScrollViewer>(&host, "PolicyScroll0")
            ));
            assert_eq!(scroll.offset().y, saved);
            if enabled {
                assert!(Rc::ptr_eq(&host.page_transition().unwrap(), &transition));
            } else {
                assert!(host.page_transition().is_none());
            }
        }

        let caller = Rc::new(LightweightSubject::<bool>::new());
        let swipe_binding = host.bind(
            Carousel::is_swipe_enabled_property(),
            caller.clone(),
            BindingPriority::LocalValue,
        );
        caller.on_next(false);
        frame();
        input_mode(&host, false);
        assert!(host.page_transition().is_some());

        crate::set_motion_enabled(false);
        caller.on_next(true);
        host.classes().add("phone-native-input");
        frame();
        input_mode(&host, false);
        crate::set_motion_enabled(true);
        frame();
        input_mode(&host, true);
        host.classes().remove("phone-native-input");
        host.classes().add("phone-panorama-static");
        input_mode(&host, true);

        crate::set_motion_enabled(false);
        caller.on_next(false);
        crate::set_motion_enabled(true);
        frame();
        input_mode(&host, false);
        assert!(caller.has_observers(), "the caller's binding stays alive");

        let transitions = Rc::new(LightweightSubject::<Option<Rc<dyn IPageTransition>>>::new());
        let transition_binding = host.bind(
            Carousel::page_transition_property(),
            transitions.clone(),
            BindingPriority::LocalValue,
        );
        crate::set_motion_enabled(false);
        let replacement: Rc<dyn IPageTransition> = Rc::new(PageSlide::with_duration(
            SignedDuration::from_milliseconds(180.0),
            SlideAxis::Horizontal,
        ));
        transitions.on_next(Some(replacement.clone()));
        assert!(host.page_transition().is_none());
        crate::set_motion_enabled(true);
        assert!(Rc::ptr_eq(&host.page_transition().unwrap(), &replacement));
        input_mode(&host, false);
        crate::set_motion_enabled(false);
        transitions.on_next(None);
        crate::set_motion_enabled(true);
        assert!(host.page_transition().is_none());
        assert!(transitions.has_observers());
        caller.on_next(true);
        frame();
        input_mode(&host, true);
        assert!(Ref::ptr_eq(&root, &child::<Grid>(&host, "PanoramaRoot")));
        swipe_binding.dispose();
        transition_binding.dispose();
    });
}

#[test]
fn suppression_cancels_held_and_completing_native_swipes_without_a_delayed_commit() {
    scene(true, |host, selection, clock| {
        let panel = host.items_panel_root().unwrap();
        let root = child::<Grid>(&host, "PanoramaRoot");
        for (gesture, release) in [(71, false), (72, true)] {
            panel.raise_event(&SwipeGestureEventArgs::new(
                gesture,
                Vector::new(220.0, 0.0),
                Vector::default(),
            ));
            frame();
            assert!(host.is_swiping());
            assert!(panel.as_scrollable().unwrap().offset().x > 0.5);
            if release {
                panel.raise_event(&SwipeGestureEndedEventArgs::new(gesture, Vector::default()));
                clock.advance(2);
                assert!(host.is_swiping(), "native completion is still running");
            }
            assert_eq!(selection.get_untracked(), 0);
            crate::set_motion_enabled(false);
            frame();
            input_mode(&host, false);
            assert!(host.page_transition().is_none());
            assert!(!host.is_swiping());
            assert!(super::admission::is_settled(&host));
            assert!(host.container_from_index(0).unwrap().is_hit_test_visible());
            panel.raise_event(&SwipeGestureEndedEventArgs::new(gesture, Vector::default()));
            clock.advance(30);
            assert_eq!(
                selection.get_untracked(),
                0,
                "a canceled completion cannot commit"
            );
            assert!(!host.is_swiping());
            assert!(super::admission::is_settled(&host));
            crate::set_motion_enabled(true);
            frame();
            input_mode(&host, true);
            assert!(Ref::ptr_eq(&panel, &host.items_panel_root().unwrap()));
            assert!(Ref::ptr_eq(&root, &child::<Grid>(&host, "PanoramaRoot")));
        }

        panel.raise_event(&SwipeGestureEventArgs::new(
            73,
            Vector::new(220.0, 0.0),
            Vector::default(),
        ));
        panel.raise_event(&SwipeGestureEndedEventArgs::new(73, Vector::default()));
        clock.advance(30);
        assert_eq!(
            selection.get_untracked(),
            1,
            "native input re-arms after suppression"
        );
        assert!(!host.is_swiping());
        assert!(super::admission::is_settled(&host));
        assert!(host.container_from_index(1).unwrap().is_hit_test_visible());
    });
}

#[test]
fn initially_reduced_policy_restores_defaults_and_releases_weak_lifetimes() {
    scene(false, |host, _, _| {
        let root = child::<Grid>(&host, "PanoramaRoot");
        input_mode(&host, false);
        assert!(host.page_transition().is_none());
        crate::set_motion_enabled(true);
        frame();
        input_mode(&host, true);
        assert!(host.page_transition().is_some());
        assert!(Ref::ptr_eq(&root, &child::<Grid>(&host, "PanoramaRoot")));

        crate::set_motion_enabled(false);
        let detached = Carousel::new();
        detached.set_is_swipe_enabled(true);
        let transition: Rc<dyn IPageTransition> = Rc::new(PageSlide::with_duration(
            SignedDuration::from_milliseconds(240.0),
            SlideAxis::Horizontal,
        ));
        detached.set_page_transition(Some(transition.clone()));
        let classes = detached.classes().listener_count();
        let properties = detached.property_changed_subscriber_count();
        let policy = super::policy::Policy::attach(&detached);
        let weak = Rc::downgrade(&policy);
        assert!(!detached.is_swipe_enabled());
        assert!(detached.page_transition().is_none());
        assert_eq!(detached.classes().listener_count(), classes + 1);
        assert_eq!(detached.property_changed_subscriber_count(), properties + 1);
        drop(policy);
        assert!(
            weak.upgrade().is_none(),
            "the registry does not own policies"
        );
        assert_eq!(detached.classes().listener_count(), classes);
        assert_eq!(detached.property_changed_subscriber_count(), properties);
        assert!(detached.is_swipe_enabled());
        assert!(Rc::ptr_eq(
            &detached.page_transition().unwrap(),
            &transition
        ));
        super::policy::refresh_motion();
        assert!(
            detached.is_swipe_enabled(),
            "a disposed policy cannot reapply suppression"
        );

        let owner = detached.downgrade();
        let policy = super::policy::Policy::attach(&detached);
        drop(detached);
        assert!(
            owner.upgrade().is_none(),
            "the policy does not own its Carousel"
        );
        super::policy::refresh_motion();
        drop(policy);
    });
}
