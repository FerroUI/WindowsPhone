use super::animation::{DotEase, PERIOD, STAGGER};
use super::*;
use ferroui_base::animation::easings::IEasing;
use ferroui_base::layout::Orientation;
use ferroui_base::threading::{CancellationToken, Dispatcher};
use ferroui_base::{AnyValue, ObjectType};
use ferroui_controls::{AppBuilder, Application, Control, Window};
use ferroui_headless::{
    FerroHeadlessPlatform, FerroHeadlessPlatformExtensions, FerroHeadlessPlatformOptions,
    HeadlessUnitTestSession,
};
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
