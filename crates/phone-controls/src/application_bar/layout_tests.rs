//! The mounted bar must retain its target slots when command captions expand.
use super::*;
use ferroui_base::input::{Key, PhysicalKey, RawInputModifiers};
use ferroui_base::threading::{CancellationToken, Dispatcher};
use ferroui_base::{Point, Ref};
use ferroui_controls::{AppBuilder, Application, Window};
use ferroui_headless::{
    FerroHeadlessPlatform, FerroHeadlessPlatformExtensions, FerroHeadlessPlatformOptions,
    HeadlessUnitTestSession, HeadlessWindowExtensions,
};
use std::time::Duration;

fn frame() {
    Dispatcher::ui_thread().run_jobs(None);
    FerroHeadlessPlatform::force_render_timer_tick(1);
    Dispatcher::ui_thread().run_jobs(None);
}

fn geometry(root: &Control) -> Vec<(Ref<Button>, f64, f64)> {
    let mut commands: Vec<_> = root
        .get_visual_descendants()
        .filter_map(|child| child.cast::<Button>())
        .filter(|button| button.classes().contains("phone-action"))
        .map(|button| {
            let point = button.translate_point(Point::new(0.0, 0.0), root).unwrap();
            let width = button.bounds().width;
            (button, point.x, width)
        })
        .collect();
    commands.sort_by(|left, right| left.1.total_cmp(&right.1));
    commands
}

#[test]
fn icon_slots_remain_centered_equal_and_stationary_when_captions_expand() {
    let session = HeadlessUnitTestSession::start_new(|| {
        AppBuilder::configure::<Application>().use_headless(FerroHeadlessPlatformOptions::default())
    });
    session.dispatch(|| {
        crate::set_motion_enabled(false);
        crate::install_theme(&Application::current().unwrap());
        EXPANSION.with(|slot| slot.borrow_mut().take());
        for count in 1..=4 {
            let labels = ["add", "save a very long copy", "delete", "search"];
            let actions = labels[..count].iter().map(|label| action(Icon::Plus, label, || {})).collect();
            let (root, owner) = app_bar(actions, false, vec![]).mount_root();
            let window = Window::new();
            window.set_height(200.0);
            window.set_content(Some(Control::boxed(root.clone())));
            window.show();
            for width in [320.0, 375.0, 400.0, 480.0] {
                window.set_width(width); frame();
                let collapsed = geometry(&root);
                assert_eq!(collapsed.len(), count);
                let group_width = (width - 88.0).min(count as f64 * 76.0);
                let expected_width = group_width / count as f64;
                let expected_start = (width - group_width) / 2.0;
                for (index, (_, x, slot_width)) in collapsed.iter().enumerate() {
                    assert!((slot_width - expected_width).abs() <= 1.0,
                        "collapsed command keeps its bounded slot: count={count} width={width}, actual={slot_width}, expected={expected_width}");
                    assert!((x - expected_start - index as f64 * expected_width).abs() <= 1.0,
                        "primary command group stays centered and equally spaced");
                }
                let more = root.find_descendant_of_type_where::<Button>(false, |button| button.has_name("AppBarMore")).unwrap();
                assert!(more.focus());
                window.key_press(Key::Space, RawInputModifiers::NONE, PhysicalKey::Space, None);
                window.key_release(Key::Space, RawInputModifiers::NONE, PhysicalKey::Space, None); frame();
                assert!(root.classes().contains("expanded"));
                let expanded = geometry(&root);
                assert_eq!(collapsed.len(), expanded.len());
                for ((original, x, width), (current, expanded_x, expanded_width)) in collapsed.iter().zip(&expanded) {
                    assert!(Ref::ptr_eq(original, current), "caption expansion retains the actual command control");
                    assert_eq!((*x, *width), (*expanded_x, *expanded_width), "revealing captions must not move or resize an action target");
                }
                window.key_press(Key::Space, RawInputModifiers::NONE, PhysicalKey::Space, None);
                window.key_release(Key::Space, RawInputModifiers::NONE, PhysicalKey::Space, None); frame();
                assert!(root.classes().contains("collapsed"));
            }
            window.close(); owner.dispose();
        }
    }, CancellationToken::none()).wait_timeout(Duration::from_secs(30)).expect("application bar geometry regression completes");
    session.dispose();
}
