use super::*;
use ferroui_base::threading::{CancellationToken, Dispatcher};
use ferroui_base::BoxedValue;
use ferroui_controls::{AppBuilder, Application, Control, ItemsSource, Window};
use ferroui_headless::{
    FerroHeadlessPlatform, FerroHeadlessPlatformExtensions, FerroHeadlessPlatformOptions,
    HeadlessUnitTestSession,
};
use ferroui_view::prelude::*;

fn frame() {
    for _ in 0..4 {
        Dispatcher::ui_thread().run_jobs(None);
        FerroHeadlessPlatform::force_render_timer_tick(1);
        Dispatcher::ui_thread().run_jobs(None);
    }
}

fn mount(state: Rc<GroupedListState>) -> (Ref<Window>, Ref<ListBox>, Owner) {
    let source =
        ItemsSource::from_items((0..3000_usize).map(|index| Some(Rc::new(index) as BoxedValue)));
    let rows = template(|index: usize| {
        let height = if [0, 1000, 2000, 2999].contains(&index) {
            56.0
        } else {
            37.0 + (index % 5) as f64 * 13.0
        };
        rsx! {Border{height:height,TextBlock{text:index.to_string(),font_size:17.0}}}
    });
    let view = rsx! {ListBox{name:"GroupedNativeList",class:"phone-list",items_source:source,item_template:rows,
    with:move |list|attach_grouped_list(list,state.clone())}};
    let (root, owner) = view.mount_root();
    let list = root
        .cast::<ListBox>()
        .expect("the native ListBox is retained");
    let window = Window::new();
    window.set_width(320.0);
    window.set_height(300.0);
    window.set_content(Some(Control::boxed(root)));
    window.show();
    frame();
    (window, list, owner)
}

fn scroll(list: &ListBox) -> Ref<ScrollViewer> {
    list.scroll()
        .and_then(|scroll| scroll.cast::<ScrollViewer>())
        .expect("native list ScrollViewer")
}

fn row_y(list: &ListBox, index: i32) -> f64 {
    let viewport = scroll(list).presenter().expect("native scroll viewport");
    list.container_from_index(index)
        .expect("target is realized")
        .translate_point(Point::default(), &viewport)
        .expect("same native visual tree")
        .y
}

#[test]
fn native_group_jumps_stick_push_and_restore_without_materializing_the_catalog() {
    let session = HeadlessUnitTestSession::start_new(|| {
        AppBuilder::configure::<Application>().use_headless(FerroHeadlessPlatformOptions::default())
    });
    session
        .dispatch(
            || {
                crate::set_motion_enabled(false);
                crate::install_theme(&Application::current().unwrap());
                let state_owner = Owner::new_root();
                let state = state_owner.with(|| GroupedListState::new(56.0));
                state.set_groups(vec![
                    ListGroup::new("a", 0),
                    ListGroup::new("b", 1000),
                    ListGroup::new("c", 2000),
                    ListGroup::new("z", 2999),
                ]);
                let (window, list, owner) = mount(state.clone());
                assert_eq!(state.current_group().as_deref(), Some("a"));
                let selection = list.selected_index();

                assert!(state.jump("b"));
                frame();
                assert!(
                    row_y(&list, 1000).abs() < 1.0,
                    "jump aligns the measured header"
                );
                assert_eq!(state.current_group().as_deref(), Some("b"));
                assert_eq!(
                    list.selected_index(),
                    selection,
                    "group navigation does not select an application"
                );
                assert!(
                    list.get_realized_containers().len() < 100,
                    "3,000 variable-height rows stay virtualized"
                );
                assert!(
                    list.container_from_index(0).is_none(),
                    "the old header was recycled"
                );
                let before = scroll(&list).offset();
                assert!(!state.jump("missing"));
                assert_eq!(scroll(&list).offset(), before);

                assert!(state.jump("c"));
                frame();
                let viewer = scroll(&list);
                viewer.set_offset(Vector::new(0.0, viewer.offset().y - 26.0));
                frame();
                assert!((row_y(&list, 2000) - 26.0).abs() < 1.0);
                assert_eq!(state.current_group().as_deref(), Some("b"));
                assert!(
                    (state.translation().y() + 30.0).abs() < 1.0,
                    "the incoming real header pushes the sticky header"
                );

                assert!(state.jump("z"));
                frame();
                let y = row_y(&list, 2999);
                assert!(
                    y >= -1.0 && y + 56.0 <= scroll(&list).viewport().height + 1.0,
                    "the final group stays in the legal native viewport"
                );
                assert!(list.get_realized_containers().len() < 100);

                assert!(state.jump("b"));
                frame();
                let viewer = scroll(&list);
                viewer.set_offset(Vector::new(0.0, viewer.offset().y + 11.0));
                frame();
                assert!((row_y(&list, 1000) + 11.0).abs() < 1.0);
                window.close();
                window.set_content(None);
                owner.dispose();
                frame();

                let (window, list, owner) = mount(state.clone());
                assert!(
                    (row_y(&list, 1000) + 11.0).abs() < 1.0,
                    "remount restores the measured item and within-row anchor"
                );
                assert_eq!(state.current_group().as_deref(), Some("b"));
                state.set_groups(Vec::new());
                state.reset();
                list.set_items_source(Some(ItemsSource::from_items(std::iter::empty())));
                frame();
                assert_eq!(
                    state.current_group(),
                    None,
                    "empty search results cannot retain a stale letter"
                );
                assert_eq!(state.translation().y(), 0.0);
                assert_eq!(scroll(&list).offset().y, 0.0);
                assert!(list.get_realized_containers().is_empty());
                window.close();
                window.set_content(None);
                owner.dispose();
                state_owner.dispose();
            },
            CancellationToken::none(),
        )
        .wait_timeout(std::time::Duration::from_secs(30))
        .expect("native grouped list regression completes");
    session.dispose();
}
