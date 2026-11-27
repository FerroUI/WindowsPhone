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

#[test]
fn minimized_bar_expands_native_commands_and_returns_to_its_original_extent() {
    let session = HeadlessUnitTestSession::start_new(|| {
        AppBuilder::configure::<Application>().use_headless(FerroHeadlessPlatformOptions::default())
    });
    let result = session
        .dispatch(
            || {
                crate::set_motion_enabled(false);
                crate::install_theme(&Application::current().unwrap());
                EXPANSION.with(|slot| slot.borrow_mut().take());
                let calls = Rc::new(std::cell::Cell::new(0));
                let invoked = calls.clone();
                let (root, owner) = rsx! {Grid {
                    Border {vertical_alignment:V::Bottom,
                        {minimized_app_bar(vec![
                            action(Icon::Plus,"new",move ||invoked.set(invoked.get()+1)),
                            action(Icon::Search,"search",||{}),
                            action(Icon::Refresh,"refresh",||{}),
                        ],false,vec![crate::flat("secondary command",||{})])}
                    }
                }}
                .mount_root();
                let window = Window::new();
                window.set_width(320.0);
                window.set_height(460.0);
                window.set_content(Some(Control::boxed(root.clone())));
                window.show();
                frame();
                let bar = root
                    .find_descendant_of_type_where::<Border>(false, |bar| {
                        bar.has_name("PhoneApplicationBar")
                    })
                    .unwrap();
                assert_eq!(bar.bounds().height, APP_BAR_MINIMIZED_HEIGHT);
                assert_eq!(APP_BAR_MINIMIZED_HEIGHT, 25.0);
                assert!(bar.classes().contains("minimized") && bar.classes().contains("collapsed"));
                assert!(
                    geometry(&root)
                        .iter()
                        .all(|(button, _, _)| !button.is_effectively_visible()),
                    "collapsed mini commands cannot take hidden input"
                );
                let more = root
                    .find_descendant_of_type_where::<Button>(false, |button| {
                        button.has_name("AppBarMore")
                    })
                    .unwrap();
                assert!(more.bounds().width >= 44.0);
                assert!(more.focus());
                let space = || {
                    window.key_press(
                        Key::Space,
                        RawInputModifiers::NONE,
                        PhysicalKey::Space,
                        None,
                    );
                    window.key_release(
                        Key::Space,
                        RawInputModifiers::NONE,
                        PhysicalKey::Space,
                        None,
                    );
                    frame();
                };
                space();
                assert!(bar.classes().contains("expanded"));
                assert!(bar.bounds().height > crate::APP_BAR_HEIGHT);
                let expanded = geometry(&root);
                assert_eq!(expanded.len(), 3);
                assert!(expanded.iter().all(
                        |(button, _, width)| button.is_effectively_visible() && *width >= 44.0
                    ));
                assert!(
                    root.find_descendant_of_type_where::<TextBlock>(false, |label| label
                        .has_name("AppBarLabel-search"))
                        .unwrap()
                        .is_effectively_visible()
                );
                assert!(
                    more.ptr_eq(
                        &root
                            .find_descendant_of_type_where::<Button>(false, |button| button
                                .has_name("AppBarMore"))
                            .unwrap()
                    ),
                    "the ellipsis retains native focus through expansion"
                );
                space();
                assert_eq!(bar.bounds().height, 25.0);
                space();
                let again = geometry(&root);
                for ((before, x, width), (after, next_x, next_width)) in expanded.iter().zip(&again)
                {
                    assert!(before.ptr_eq(after));
                    assert_eq!((*x, *width), (*next_x, *next_width));
                }
                let new = root
                    .find_descendant_of_type_where::<Button>(false, |button| {
                        button.has_name("AppBarAction-new")
                    })
                    .unwrap();
                assert!(new.focus());
                space();
                assert_eq!(calls.get(), 1);
                assert_eq!(
                    bar.bounds().height,
                    25.0,
                    "a command closes back to minimized size"
                );
                assert!(bar.classes().contains("collapsed"));
                window.close();
                window.set_content(None);
                owner.dispose();
            },
            CancellationToken::none(),
        )
        .wait_timeout(Duration::from_secs(30));
    session.dispose();
    result.expect("minimized native application-bar regression completes");
}
