use super::activity::ActivityMotion;
use super::animation::{DotEase, PERIOD, STAGGER};
use super::*;
use ferroui_base::animation::easings::IEasing;
use ferroui_base::animation::{IClock, PlayState, SignedDuration};
use ferroui_base::layout::Orientation;
use ferroui_base::reactive::{IDisposable, IObservable, IObserver, LightweightSubject};
use ferroui_base::rendering::composition::server::ServerCompositionVisual;
use ferroui_base::rendering::composition::ElementComposition;
use ferroui_base::threading::{CancellationToken, Dispatcher};
use ferroui_base::{AnyValue, ObjectType};
use ferroui_controls::{AppBuilder, Application, Control, Window};
use ferroui_headless::{
    FerroHeadlessPlatform, FerroHeadlessPlatformExtensions, FerroHeadlessPlatformOptions,
    HeadlessUnitTestSession,
};
use std::cell::Cell;
use std::time::Duration;

#[track_caller]
fn close(actual: f64, expected: f64) {
    assert!((actual - expected).abs() < 1e-9, "{actual} != {expected}");
}

#[test]
fn original_progress_motion_preserves_stagger_key_times_and_exponential_segments() {
    for index in 0..DOT_COUNT {
        let position = DotEase {
            index,
            opacity: false,
        };
        let opacity = DotEase {
            index,
            opacity: true,
        };
        let start = index as f64 * STAGGER;
        for (seconds, x) in [(0.0, 0.0), (0.5, 0.33), (2.0, 0.66), (2.5, 1.0)] {
            close(position.ease((start + seconds) / PERIOD), x);
        }
        // Exponent one; these are not a cubic or generic ease-in/out.
        let exponential_mid = (0.5_f64.exp() - 1.0) / (1.0_f64.exp() - 1.0);
        close(
            position.ease((start + 0.25) / PERIOD),
            0.33 * (1.0 - exponential_mid),
        );
        close(position.ease((start + 1.25) / PERIOD), 0.495);
        close(
            position.ease((start + 2.25) / PERIOD),
            0.66 + 0.34 * exponential_mid,
        );
        close(opacity.ease((start + 0.001) / PERIOD), 1.0);
        close(opacity.ease((start + 2.499) / PERIOD), 1.0);
        close(opacity.ease((start + 2.501) / PERIOD), 0.0);
        if index > 0 {
            close(opacity.ease((start - 0.001) / PERIOD), 0.0);
        }
        close(opacity.ease(1.0), 0.0);
        let compositor = position.to_shared();
        for step in 0..=440 {
            let phase = step as f64 / 440.0;
            close(compositor.ease(phase), position.ease(phase));
            assert!((0.0..=1.0).contains(&position.ease(phase)));
        }
    }
}

fn frame() {
    Dispatcher::ui_thread().run_jobs(None);
    FerroHeadlessPlatform::force_render_timer_tick(1);
    Dispatcher::ui_thread().run_jobs(None);
}

fn part<T: ObjectType>(owner: &ProgressBar, name: &str) -> Ref<T> {
    owner
        .get_template_descendants()
        .into_iter()
        .find(|part| part.has_name(name))
        .and_then(|part| part.cast::<T>())
        .unwrap_or_else(|| panic!("missing native progress part {name}"))
}

fn animation_count(root: &Grid) -> usize {
    let tag = root.tag().expect("template-owned motion controller");
    (&*tag as &dyn AnyValue)
        .downcast_ref::<ProgressMotion>()
        .expect("progress controller tag")
        .animation_count()
}

fn activity(root: &Grid) -> Rc<ActivityMotion> {
    let tag = root.tag().expect("template-owned motion controller");
    (&*tag as &dyn AnyValue)
        .downcast_ref::<ProgressMotion>()
        .unwrap()
        .activity()
        .expect("activity-only wrapper")
        .clone()
}

/// Only the native animation clock is stepped here; no production timing hook.
struct ManualClock {
    ticks: LightweightSubject<SignedDuration>,
    play: Cell<PlayState>,
}

impl ManualClock {
    fn new() -> Rc<Self> {
        Rc::new(Self {
            ticks: LightweightSubject::new(),
            play: Cell::new(PlayState::Run),
        })
    }

    fn step(&self, seconds: f64) {
        self.ticks.on_next(SignedDuration::from_seconds(seconds));
        Dispatcher::ui_thread().run_jobs(None);
    }
}

impl IObservable<SignedDuration> for ManualClock {
    fn subscribe(&self, observer: Rc<dyn IObserver<SignedDuration>>) -> Rc<dyn IDisposable> {
        self.ticks.subscribe(observer)
    }
}

impl IClock for ManualClock {
    fn play_state(&self) -> PlayState {
        self.play.get()
    }
    fn set_play_state(&self, value: PlayState) {
        self.play.set(value);
    }
}

fn native_activity() -> (Ref<Grid>, Rc<ActivityMotion>, Rc<ManualClock>) {
    let root = Grid::new();
    let clock = ManualClock::new();
    root.set_clock(Some(clock.clone()));
    let activity = ActivityMotion::new(&root, Rc::new(|| {}));
    assert!(ElementComposition::get_element_visual(&root).is_none());
    (root, activity, clock)
}

#[test]
fn activity_wrapper_native_clock_preserves_original_linear_fades_and_delayed_dot_stop() {
    let session = HeadlessUnitTestSession::start_new(|| {
        AppBuilder::configure::<Application>().use_headless(FerroHeadlessPlatformOptions::default())
    });
    session
        .dispatch(
            || {
                let (root, activity, clock) = native_activity();
                activity.update(false, true);
                close(root.opacity(), 0.0);
                assert!(!activity.actual() && !activity.is_fading());
                activity.update(true, true);
                assert!(
                    activity.actual() && activity.is_fading(),
                    "dots start before the fade completes"
                );
                clock.step(0.0);
                clock.step(0.125);
                close(root.opacity(), 0.5);
                clock.step(0.249);
                assert!(activity.is_fading());
                clock.step(0.251);
                close(root.opacity(), 1.0);
                assert!(activity.actual() && !activity.is_fading());

                activity.update(false, true);
                assert!(
                    activity.actual(),
                    "completion requests Hidden without stopping the dots"
                );
                clock.step(1.0);
                clock.step(1.375);
                close(root.opacity(), 0.5);
                clock.step(1.749);
                assert!(activity.actual() && activity.is_fading());
                clock.step(1.751);
                close(root.opacity(), 0.0);
                assert!(!activity.actual() && !activity.is_fading());
                clock.step(2.0);
                assert!(
                    !clock.ticks.has_observers(),
                    "finite native animation releases its clock"
                );
            },
            CancellationToken::none(),
        )
        .wait_timeout(Duration::from_secs(10))
        .unwrap();
    session.dispose();
}

#[test]
fn activity_wrapper_reverses_native_fades_and_discards_canceled_completion() {
    let session = HeadlessUnitTestSession::start_new(|| {
        AppBuilder::configure::<Application>().use_headless(FerroHeadlessPlatformOptions::default())
    });
    session
        .dispatch(
            || {
                let (root, activity, clock) = native_activity();
                activity.update(false, true);
                activity.update(true, true);
                clock.step(0.0);
                clock.step(0.125);
                close(root.opacity(), 0.5);
                activity.update(false, true);
                close(root.opacity(), 0.5);
                clock.step(0.125);
                clock.step(0.5);
                close(root.opacity(), 0.25);
                activity.update(true, true);
                close(root.opacity(), 0.25);
                clock.step(0.5);
                clock.step(0.625);
                close(root.opacity(), 0.625);
                clock.step(1.5);
                close(root.opacity(), 1.0);
                assert!(
                    activity.actual() && !activity.is_fading(),
                    "old exits cannot hide renewed work"
                );

                activity.update(false, true);
                clock.step(1.5);
                clock.step(1.7);
                activity.update(false, false);
                assert!(!activity.actual() && !activity.is_fading());
                close(root.opacity(), 0.0);
                activity.update(true, false);
                close(root.opacity(), 1.0);
                assert!(activity.actual() && !activity.is_fading());
                activity.update(true, true);
                assert!(
                    !activity.is_fading(),
                    "visibility/motion restoration reflects the current request"
                );
                activity.update(false, true);
                activity.detach();
                clock.step(4.0);
                close(root.opacity(), 0.0);
                assert!(!activity.actual() && !activity.is_fading());
                assert!(!clock.ticks.has_observers());
                activity.update(true, true);
                close(root.opacity(), 1.0);
                assert!(
                    !activity.is_fading(),
                    "reattachment has the original no-transition initial state"
                );
                activity.update(false, true);
                drop(activity);
                clock.step(5.0);
                assert!(
                    !clock.ticks.has_observers(),
                    "dropping the template cancels a pending exit"
                );
            },
            CancellationToken::none(),
        )
        .wait_timeout(Duration::from_secs(10))
        .unwrap();
    session.dispose();
}

fn advance(duration: Duration) {
    let until = std::time::Instant::now() + duration;
    while std::time::Instant::now() < until {
        frame();
        std::thread::sleep(Duration::from_millis(5));
    }
    frame();
}

fn composed_opacity(root: &Grid) -> f64 {
    let visual = ElementComposition::get_element_visual(root).expect("mounted compositor visual");
    visual.compositor().with_server(|server| {
        f64::from(
            server
                .get::<ServerCompositionVisual>(visual.server())
                .expect("server visual")
                .opacity(),
        )
    })
}

#[test]
fn mounted_activity_retains_compositor_motion_through_exit_and_cleans_up_every_lifecycle_path() {
    let session = HeadlessUnitTestSession::start_new(|| {
        AppBuilder::configure::<Application>().use_headless(FerroHeadlessPlatformOptions::default())
    });
    session
        .dispatch(
            || {
                crate::install_theme(&Application::current().unwrap());
                crate::set_motion_enabled(true);
                let bar = ProgressBar::new();
                bar.classes().add("phone-progress");
                bar.classes().add("phone-activity-progress");
                bar.set_width(400.0);
                bar.set_opacity(0.65);
                bar.set_range_value(37.0);
                let window = Window::new();
                window.set_width(400.0);
                window.set_height(240.0);
                window.set_content(Some(Control::boxed(bar.clone())));
                window.show();
                for _ in 0..3 {
                    frame();
                }
                let root = part::<Grid>(&bar, "PhoneProgressRoot");
                let activity = activity(&root);
                close(root.opacity(), 0.0);
                assert!(!part::<Border>(&bar, "PhoneProgressTrack").is_visible());
                assert!(!part::<Border>(&bar, "PART_Indicator").is_visible());

                bar.set_is_indeterminate(true);
                frame();
                assert!(activity.is_fading());
                advance(Duration::from_millis(90));
                let entering = composed_opacity(&root);
                assert!(
                    entering > 0.0 && entering < 1.0,
                    "compositor entry interpolates: {entering}"
                );
                close(root.opacity(), 0.0); // The UI property is not compositor readback.
                assert_eq!(animation_count(&root), 2 * DOT_COUNT);
                advance(Duration::from_millis(220));
                assert!(!activity.is_fading());
                close(composed_opacity(&root), 1.0);

                bar.set_is_indeterminate(false);
                frame();
                advance(Duration::from_millis(150));
                let exiting = composed_opacity(&root);
                assert!(exiting > 0.0 && exiting < 1.0);
                assert!(activity.actual() && activity.is_fading());
                assert!(
                    !bar.is_indeterminate(),
                    "the requested native property is never overwritten"
                );
                close(bar.opacity(), 0.65);
                close(bar.value(), 37.0);
                assert_eq!(
                    animation_count(&root),
                    2 * DOT_COUNT,
                    "dots continue through the wrapper exit"
                );
                bar.set_is_indeterminate(true);
                frame();
                assert!(
                    (composed_opacity(&root) - exiting).abs() < 0.1,
                    "a compositor reversal must continue from its current frame"
                );
                advance(Duration::from_millis(300));
                assert!(activity.actual() && !activity.is_fading());
                close(composed_opacity(&root), 1.0);

                bar.set_is_indeterminate(false);
                frame();
                crate::set_motion_enabled(false);
                assert!(!activity.actual() && !activity.is_fading());
                assert_eq!(animation_count(&root), 0);
                close(root.opacity(), 0.0);
                bar.set_is_indeterminate(true);
                assert!(activity.actual() && !activity.is_fading());
                close(root.opacity(), 1.0);
                assert_eq!(animation_count(&root), 0);
                crate::set_motion_enabled(true);
                frame();
                assert_eq!(animation_count(&root), 2 * DOT_COUNT);
                bar.set_is_indeterminate(false);
                bar.set_is_visible(false);
                assert!(!activity.actual() && !activity.is_fading());
                assert_eq!(animation_count(&root), 0);
                bar.set_is_visible(true);
                frame();
                close(root.opacity(), 0.0);
                bar.set_is_indeterminate(true);
                frame();
                assert!(activity.is_fading());
                window.set_content(None);
                frame();
                assert!(!activity.actual() && !activity.is_fading());
                assert_eq!(animation_count(&root), 0);
                window.set_content(Some(Control::boxed(bar.clone())));
                for _ in 0..3 {
                    frame();
                }
                assert!(activity.actual() && !activity.is_fading());
                close(root.opacity(), 1.0);
                assert_eq!(animation_count(&root), 2 * DOT_COUNT);
                bar.set_is_indeterminate(false);
                frame();
                advance(Duration::from_millis(800));
                assert!(!activity.actual() && !activity.is_fading());
                assert_eq!(animation_count(&root), 0);
                close(composed_opacity(&root), 0.0);
                assert!(
                    bar.is_visible(),
                    "the wrapper never changes application-owned visibility"
                );
                window.close();
            },
            CancellationToken::none(),
        )
        .wait_timeout(Duration::from_secs(15))
        .unwrap();
    session.dispose();
}

#[test]
fn native_progress_retains_range_and_template_and_owns_only_visible_animation_leases() {
    let session = HeadlessUnitTestSession::start_new(|| {
        AppBuilder::configure::<Application>().use_headless(FerroHeadlessPlatformOptions::default())
    });
    session
        .dispatch(
            || {
                let app = Application::current().unwrap();
                crate::set_motion_enabled(true);
                crate::install_theme(&app);
                let bar = ProgressBar::new();
                bar.classes().add("phone-progress");
                bar.set_width(400.0);
                bar.set_minimum(10.0);
                bar.set_maximum(110.0);
                bar.set_range_value(35.0);
                let window = Window::new();
                window.set_width(400.0);
                window.set_height(240.0);
                window.set_content(Some(Control::boxed(bar.clone())));
                window.show();
                for _ in 0..3 {
                    frame();
                }
                let root = part::<Grid>(&bar, "PhoneProgressRoot");
                let indicator = part::<Border>(&bar, "PART_Indicator");
                let track = part::<Border>(&bar, "PhoneProgressTrack");
                close(bar.percentage(), 25.0);
                close(indicator.bounds().width, root.bounds().width * 0.25);
                close(bar.height(), DOT_SIZE);
                close(indicator.bounds().height, DOT_SIZE.ceil());
                assert!(track.is_visible() && indicator.is_visible());
                assert_eq!(animation_count(&root), 0);

                bar.set_range_value(200.0);
                frame();
                close(bar.value(), 110.0);
                close(indicator.bounds().width, root.bounds().width);
                bar.set_range_value(-10.0);
                frame();
                close(bar.value(), 10.0);
                close(indicator.bounds().width, 0.0);

                bar.set_is_indeterminate(true);
                frame();
                assert_eq!(animation_count(&root), 2 * DOT_COUNT);
                assert!(!indicator.is_visible() && !track.is_visible());
                for index in 0..DOT_COUNT {
                    let dot = part::<Border>(&bar, &format!("PhoneProgressDot{index}"));
                    close(dot.width(), DOT_SIZE);
                    close(dot.height(), DOT_SIZE);
                    close(dot.bounds().width, DOT_SIZE.ceil());
                    close(dot.bounds().height, DOT_SIZE.ceil());
                    assert!(dot.render_transform().is_some());
                }

                crate::set_palette(&app, "#A200FF", true);
                frame();
                assert_eq!(root, part::<Grid>(&bar, "PhoneProgressRoot"));
                assert_eq!(indicator, part::<Border>(&bar, "PART_Indicator"));
                assert_eq!(animation_count(&root), 2 * DOT_COUNT);

                bar.set_is_visible(false);
                assert_eq!(
                    animation_count(&root),
                    0,
                    "hidden ancestors stop native animations"
                );
                bar.set_is_visible(true);
                frame();
                assert_eq!(animation_count(&root), 2 * DOT_COUNT);
                crate::set_motion_enabled(false);
                assert_eq!(
                    animation_count(&root),
                    0,
                    "preferences apply to retained templates immediately"
                );
                for index in 0..DOT_COUNT {
                    let dot = part::<Border>(&bar, &format!("PhoneProgressDot{index}"));
                    close(dot.opacity(), 1.0);
                }
                crate::set_motion_enabled(true);
                assert_eq!(animation_count(&root), 2 * DOT_COUNT);
                bar.set_is_enabled(false);
                assert_eq!(animation_count(&root), 0);
                bar.set_is_enabled(true);
                assert_eq!(animation_count(&root), 2 * DOT_COUNT);
                bar.set_width(320.0);
                frame();
                assert_eq!(
                    animation_count(&root),
                    2 * DOT_COUNT,
                    "resize replaces leases without accumulating"
                );

                bar.set_is_indeterminate(false);
                bar.set_range_value(35.0);
                bar.clear_value(Layoutable::width_property());
                bar.set_height(200.0);
                bar.set_orientation(Orientation::Vertical);
                frame();
                assert_eq!(animation_count(&root), 0);
                assert_eq!(indicator.vertical_alignment(), V::Bottom);
                close(indicator.bounds().height, root.bounds().height * 0.25);
                close(indicator.bounds().width, DOT_SIZE.ceil());
                bar.set_is_indeterminate(true);
                frame();
                assert_eq!(animation_count(&root), 2 * DOT_COUNT);
                window.set_content(None);
                frame();
                assert_eq!(
                    animation_count(&root),
                    0,
                    "detachment cancels every infinite animation"
                );
                window.close();
            },
            CancellationToken::none(),
        )
        .wait_timeout(Duration::from_secs(20))
        .expect("native progress contract completes");
    session.dispose();
}
