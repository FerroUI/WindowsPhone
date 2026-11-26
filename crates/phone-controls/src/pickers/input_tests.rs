//! Actual pointer routing through the native overlay and recycled wheel cells.
use super::*;
use ferroui_base::input::MouseButton;
use ferroui_base::Point;
use std::{cell::Cell, rc::Rc};

fn cell(panel: &DateTimePickerPanel, value: i32) -> Ref<ListBoxItem> {
    panel
        .children()
        .snapshot()
        .iter()
        .filter_map(|child| child.cast::<ListBoxItem>())
        .find(|item| {
            item.tag()
                .is_some_and(|tag| (&*tag as &dyn AnyValue).downcast_ref::<i32>() == Some(&value))
        })
        .unwrap_or_else(|| panic!("value {value} is visible in the native wheel"))
}

fn center(control: &Control, window: &Window) -> Point {
    control
        .translate_point(
            Point::new(control.bounds().width / 2.0, control.bounds().height / 2.0),
            window,
        )
        .expect("pointer target is attached to the window")
}

#[test]
fn picker_pointer_taps_stage_once_and_cancel_without_accepting_the_owner() {
    let session = HeadlessUnitTestSession::start_new(|| {
        AppBuilder::configure::<Application>().use_headless(FerroHeadlessPlatformOptions::default())
    });
    session.dispatch(|| {
        crate::set_motion_enabled(false);
        crate::install_theme(&Application::current().unwrap());
        let original = date(2024, 5, 15);
        let (root, owner) = rsx! {Grid {name:"PhonePage",
            DatePicker {class:"phone-picker",margin:20.0,
                vertical_alignment:ferroui_base::layout::VerticalAlignment::Top,
                selected_date:Some(original),with:|picker|attach_picker_focus(picker)}
        }}.mount_root();
        let window = Window::new();
        window.set_width(400.0); window.set_height(700.0);
        window.set_content(Some(Control::boxed(root.clone()))); window.show(); frame();
        let picker = root.find_descendant_of_type::<DatePicker>(false).unwrap();
        let presenter = open(&window, &picker);
        let day = part::<DateTimePickerPanel>(&presenter, "PART_DaySelector");
        let selected = center(&cell(&day, 15), &window);
        let touch = window.touch_begin(selected, RawInputModifiers::NONE);
        assert_eq!(day.selected_value(), 15);
        window.touch_end(&*touch, selected, RawInputModifiers::NONE); frame();
        assert!(day.classes().contains("expanded"));
        assert!(day.is_focused());
        assert_eq!(day.selected_value(), 15, "expansion keeps the selected value");

        let observed_clicks = Rc::new(Cell::new(0));
        let observed = observed_clicks.clone();
        day.add_handler_with(ferroui_base::input::InputElement::pointer_pressed_event(), move |_, event| {
            observed.set(observed.get().max(event.click_count()));
        }, ferroui_base::interactivity::RoutingStrategies::TUNNEL, true);
        let next = center(&cell(&day, 16), &window);
        for expected in 16..=18 {
            window.mouse_move(next, RawInputModifiers::NONE);
            window.mouse_down(next, MouseButton::Left, RawInputModifiers::NONE);
            assert_eq!(day.selected_value(), expected - 1, "a pointer press does not stage a value before release");
            window.mouse_up(next, MouseButton::Left, RawInputModifiers::NONE); frame();
            assert_eq!(day.selected_value(), expected, "every rapid tap stages exactly its pressed value, including double-tap input");
            assert_eq!(picker.selected_date(), Some(original), "pointer selection never confirms the page");
        }
        assert!(observed_clicks.get() >= 2, "the native input stream exercised a multi-click gesture");

        // Native pointer cancellation must not be interpreted as a release.
        let next = center(&cell(&day, 19), &window);
        let touch = window.touch_begin(next, RawInputModifiers::NONE);
        touch.dispose(); frame();
        assert_eq!(day.selected_value(), 18, "cancelled contacts do not stage a value");

        let month = part::<DateTimePickerPanel>(&presenter, "PART_MonthSelector");
        let month_point = center(&cell(&month, 5), &window);
        let first = window.touch_begin(next, RawInputModifiers::NONE);
        let second = window.touch_begin(month_point, RawInputModifiers::NONE);
        window.touch_end(&*first, next, RawInputModifiers::NONE); frame();
        window.touch_end(&*second, month_point, RawInputModifiers::NONE); frame();
        assert_eq!(day.selected_value(), 18, "a cross-wheel second contact cancels the first candidate");
        assert_eq!(month.selected_value(), 5);

        let first = window.touch_begin(next, RawInputModifiers::NONE);
        let second = window.touch_begin(next, RawInputModifiers::NONE);
        let third = window.touch_begin(next, RawInputModifiers::NONE);
        window.touch_end(&*second, next, RawInputModifiers::NONE);
        first.dispose();
        window.touch_end(&*third, next, RawInputModifiers::NONE); frame();
        assert_eq!(day.selected_value(), 18, "a third contact cannot replace a cancelled multi-touch transaction");
        let recovered = window.touch_begin(next, RawInputModifiers::NONE);
        window.touch_end(&*recovered, next, RawInputModifiers::NONE); frame();
        assert_eq!(day.selected_value(), 19, "selection resumes after every overlapping contact ends");

        // A drag owns native scrolling. Its terminal release must not use a
        // recycled container's tag as a second selection or accept the page.
        // Disable inertia in this routing regression so terminal native snap
        // is observable synchronously; browser coverage retains normal physics.
        day.get_visual_ancestors().find_map(|part| part.cast::<ferroui_controls::ScrollViewer>())
            .unwrap().set_is_scroll_inertia_enabled(false);
        let start = center(&cell(&day, 19), &window);
        let touch = window.touch_begin(start, RawInputModifiers::NONE);
        let end = Point::new(start.x, start.y - day.item_height() * 1.25);
        window.touch_move(&*touch, end, RawInputModifiers::NONE); frame();
        assert_ne!(day.selected_value(), 19, "native scrolling changes the staged selection");
        let overlapping = window.touch_begin(month_point, RawInputModifiers::NONE);
        assert!(day.classes().contains("expanded"));
        assert!(!month.classes().contains("expanded"), "a second wheel cannot take selection while the native scroll recognizer holds the first contact");
        window.touch_end(&*touch, end, RawInputModifiers::NONE); frame();
        let staged = day.selected_value();
        let next = center(&cell(&day, staged + 1), &window);
        let third = window.touch_begin(next, RawInputModifiers::NONE);
        window.touch_end(&*third, next, RawInputModifiers::NONE); frame();
        assert_eq!(day.selected_value(), staged, "the remaining second contact keeps a third tap suppressed after drag release");
        window.touch_end(&*overlapping, month_point, RawInputModifiers::NONE); frame();
        let recovered = window.touch_begin(next, RawInputModifiers::NONE);
        window.touch_end(&*recovered, next, RawInputModifiers::NONE); frame();
        assert_eq!(day.selected_value(), staged + 1, "normal taps recover after a captured drag and every overlapping finger end");
        assert_eq!(picker.selected_date(), Some(original));
        assert!(has_open_phone_picker());

        let dismiss = part::<Button>(&presenter, "PART_DismissButton");
        let target = center(&dismiss, &window);
        window.mouse_down(target, MouseButton::Left, RawInputModifiers::NONE);
        window.mouse_up(target, MouseButton::Left, RawInputModifiers::NONE); frame();
        assert!(!has_open_phone_picker());
        assert_eq!(picker.selected_date(), Some(original), "native cancel abandons taps and drags");
        window.close(); owner.dispose();
    }, CancellationToken::none()).wait_timeout(Duration::from_secs(30)).expect("pointer picker regression completes");
    session.dispose();
}
