//! Mounted native picker contracts: input, staged values, geometry and teardown.
use super::*;
use ferroui_base::animation::SignedDuration;
use ferroui_base::input::{Key, PhysicalKey, RawInputModifiers};
use ferroui_base::threading::{CancellationToken, Dispatcher};
use ferroui_base::utilities::DateTimeOffset;
use ferroui_base::{AnyValue, ObjectType, Ref};
use ferroui_controls::primitives::{DateTimePickerPanel, Popup};
use ferroui_controls::{
    AppBuilder, Application, Button, Control, DatePicker, Grid, ListBoxItem, TimePicker, Window,
};
use ferroui_headless::{
    FerroHeadlessPlatform, FerroHeadlessPlatformExtensions, FerroHeadlessPlatformOptions,
    HeadlessUnitTestSession, HeadlessWindowExtensions,
};
use ferroui_view::prelude::*;
use std::time::Duration;

fn frame() {
    Dispatcher::ui_thread().run_jobs(None);
    FerroHeadlessPlatform::force_render_timer_tick(1);
    Dispatcher::ui_thread().run_jobs(None);
}

fn part<T: ObjectType>(owner: &TemplatedControl, name: &str) -> Ref<T> {
    owner
        .get_template_descendants()
        .into_iter()
        .find(|part| part.has_name(name))
        .and_then(|part| part.cast::<T>())
        .unwrap_or_else(|| panic!("missing native picker part {name}"))
}

fn key(window: &Window, key: Key, physical: PhysicalKey) {
    window.key_press(key, RawInputModifiers::NONE, physical, None);
    window.key_release(key, RawInputModifiers::NONE, physical, None);
    frame();
}

fn open(window: &Window, picker: &TemplatedControl) -> Ref<TemplatedControl> {
    // Popup moves its child into the native overlay host when opened. Keep
    // the actual declared child before that visual-tree reparenting; walking
    // the owning field's visual template afterward cannot discover it.
    let presenter = part::<Popup>(picker, "PART_Popup")
        .child()
        .and_then(|child| child.cast::<TemplatedControl>())
        .expect("native popup presenter child");
    assert!(presenter.has_name("PART_PickerPresenter"));
    assert!(part::<Button>(picker, "PART_FlyoutButton").focus());
    key(window, Key::Space, PhysicalKey::Space);
    assert!(has_open_phone_picker());
    presenter
}

fn date(year: i32, month: i32, day: i32) -> DateTimeOffset {
    DateTimeOffset::new(year, month, day, 0, 0, 0, SignedDuration::ZERO)
}

fn assert_centered_wheel(panel: &DateTimePickerPanel) {
    let children = panel.children().snapshot();
    let selected = children
        .iter()
        .filter_map(|child| child.cast::<ListBoxItem>())
        .find(|item| item.is_selected())
        .expect("a native wheel has one selected container");
    let bounds = selected.bounds();
    assert!(
        (bounds.y + bounds.height / 2.0 - panel.bounds().height / 2.0).abs() < 1.0,
        "selected value stays centered after the native ring grows: item {bounds:?}, panel {:?}",
        panel.bounds()
    );
    for child in children.iter() {
        let item = child.cast::<ListBoxItem>().unwrap();
        assert!(
            item.tag()
                .is_some_and(|tag| { (&*tag as &dyn AnyValue).downcast_ref::<i32>().is_some() }),
            "every recycled native slot receives its value after a viewport resize"
        );
        assert!(
            item.content().is_some_and(|content| {
                (&*content as &dyn AnyValue)
                    .downcast_ref::<String>()
                    .is_some_and(|text| !text.is_empty())
            }),
            "expanded native slots contain formatted values instead of blank cells"
        );
    }
}

#[test]
fn native_phone_date_page_preserves_staging_range_and_palette_identity() {
    let session = HeadlessUnitTestSession::start_new(|| {
        AppBuilder::configure::<Application>().use_headless(FerroHeadlessPlatformOptions::default())
    });
    session
        .dispatch(
            || {
                let app = Application::current().unwrap();
                crate::set_motion_enabled(false);
                crate::install_theme(&app);
                let original = date(2024, 1, 31);
                let (root, owner) = rsx! {Grid {row_definitions:"26,*,56",
            Grid {name:"PhonePage",Grid.row:1,
                DatePicker {name:"DateUnderTest",class:"phone-picker",margin:20.0,
                    vertical_alignment:ferroui_base::layout::VerticalAlignment::Top,
                    selected_date:Some(original),min_year:date(2020,1,1),max_year:date(2030,12,31),
                    with:|picker|attach_picker_focus(picker)}
            }
        }}.mount_root();
                let window = Window::new();
                window.set_width(320.0);
                window.set_height(600.0);
                window.set_content(Some(Control::boxed(root.clone())));
                window.show();
                frame();
                let picker = root.find_descendant_of_type::<DatePicker>(false).unwrap();
                let page = root
                    .find_descendant_of_type_where::<Grid>(false, |part| part.has_name("PhonePage"))
                    .unwrap();
                let presenter = open(&window, &picker);
                assert_eq!(
                    presenter.bounds().size(),
                    page.bounds().size(),
                    "picker fills the page but preserves shell strips"
                );
                let surface = part::<Grid>(&presenter, "PhonePickerSurface");
                assert!(
                    surface.render_transform().is_none(),
                    "reduced motion is settled immediately"
                );
                let month = part::<DateTimePickerPanel>(&presenter, "PART_MonthSelector");
                let day = part::<DateTimePickerPanel>(&presenter, "PART_DaySelector");
                let year = part::<DateTimePickerPanel>(&presenter, "PART_YearSelector");
                assert_eq!((year.minimum_value(), year.maximum_value()), (2020, 2030));
                assert!((month.item_height() - 148.0 * 320.0 / 480.0).abs() < 0.01);
                for panel in [&month, &day, &year] {
                    assert_centered_wheel(panel);
                }
                // Growing only the viewport height does not change item
                // pitch. It must still populate the native panel's larger
                // recycled ring, preserving its staged date and focus.
                for height in [820.0, 600.0] {
                    window.set_height(height);
                    frame();
                    for panel in [&month, &day, &year] {
                        assert_centered_wheel(panel);
                    }
                    assert_eq!(month.selected_value(), 1);
                    assert_eq!(day.selected_value(), 31);
                    assert_eq!(year.selected_value(), 2024);
                    assert_eq!(picker.selected_date(), Some(original));
                }
                assert!(month.focus());
                key(&window, Key::Down, PhysicalKey::ArrowDown);
                assert_eq!(
                    (month.selected_value(), day.selected_value()),
                    (2, 29),
                    "native February clamps a staged leap day"
                );
                assert_eq!(
                    picker.selected_date(),
                    Some(original),
                    "wheel input remains uncommitted"
                );
                assert!(month.classes().contains("expanded"));
                assert!(!day.classes().contains("expanded"));
                for expected in 3..=9 {
                    key(&window, Key::Down, PhysicalKey::ArrowDown);
                    assert_eq!(
                        month.selected_value(),
                        expected,
                        "every native Down key advances one month at a 320-unit viewport"
                    );
                }
                assert_eq!(month.selected_value(), 9);
                let selected = month
                    .children()
                    .snapshot()
                    .iter()
                    .filter_map(|child| child.cast::<ListBoxItem>())
                    .find(|item| item.is_selected())
                    .unwrap();
                let label = part::<TextBlock>(&selected, "PhonePickerSecondary");
                assert_eq!(label.text().as_deref(), Some("September"));
                assert!(
                    label.bounds().width > 60.0 && label.bounds().width < selected.bounds().width,
                    "long names have their own bounded line"
                );
                crate::set_palette(&app, "#A200FF", true);
                frame();
                assert!(
                    Ref::ptr_eq(
                        &month,
                        &part::<DateTimePickerPanel>(&presenter, "PART_MonthSelector")
                    ),
                    "palette changes preserve wheel and focus ownership"
                );
                assert!(
                    Ref::ptr_eq(
                        &label,
                        &part::<TextBlock>(&selected, "PhonePickerSecondary")
                    ),
                    "palette changes preserve the selected cell's native presentation"
                );
                assert_eq!(month.selected_value(), 9);
                key(&window, Key::Enter, PhysicalKey::Enter);
                assert!(!has_open_phone_picker());
                assert_eq!(picker.selected_date(), Some(date(2024, 9, 29)));
                assert!(picker.is_keyboard_focus_within());
                let presenter = open(&window, &picker);
                assert!(part::<DateTimePickerPanel>(&presenter, "PART_MonthSelector").focus());
                key(&window, Key::Down, PhysicalKey::ArrowDown);
                key(&window, Key::Escape, PhysicalKey::Escape);
                assert_eq!(
                    picker.selected_date(),
                    Some(date(2024, 9, 29)),
                    "Escape cancels native staged changes"
                );
                let presenter = open(&window, &picker);
                assert!(part::<Button>(&presenter, "PhonePickerMore").focus());
                key(&window, Key::Space, PhysicalKey::Space);
                assert!(dismiss_phone_picker());
                assert!(
                    has_open_phone_picker(),
                    "first Back collapses picker command captions"
                );
                assert!(dismiss_phone_picker());
                assert!(
                    !has_open_phone_picker(),
                    "second Back dismisses the picker only"
                );
                assert!(picker.is_attached_to_visual_tree());
                window.close();
                owner.dispose();
            },
            CancellationToken::none(),
        )
        .wait_timeout(Duration::from_secs(30))
        .expect("native date page regression completes");
    session.dispose();
}

#[test]
fn native_phone_time_page_keeps_clock_increment_seconds_and_teardown_contracts() {
    let session = HeadlessUnitTestSession::start_new(|| {
        AppBuilder::configure::<Application>().use_headless(FerroHeadlessPlatformOptions::default())
    });
    session.dispatch(|| {
        crate::set_motion_enabled(false);
        crate::install_theme(&Application::current().unwrap());
        let original=SignedDuration::from_hms(13,5,30);
        let (root,owner)=rsx! {Grid {name:"PhonePage",
            TimePicker {class:"phone-picker",margin:20.0,vertical_alignment:ferroui_base::layout::VerticalAlignment::Top,
                selected_time:Some(original),clock_identifier:"24HourClock",minute_increment:5,
                with:|picker|attach_picker_focus(picker)}
        }}.mount_root();
        let window=Window::new(); window.set_width(320.0); window.set_height(460.0);
        window.set_content(Some(Control::boxed(root.clone()))); window.show(); frame();
        let picker=root.find_descendant_of_type::<TimePicker>(false).unwrap();
        let presenter=open(&window,&picker);
        assert!(!part::<Grid>(&presenter,"PART_PeriodHost").is_visible());
        let hour=part::<DateTimePickerPanel>(&presenter,"PART_HourSelector");
        let minute=part::<DateTimePickerPanel>(&presenter,"PART_MinuteSelector");
        assert!(hour.is_focused(),"native first-selector focus is attached before input");
        key(&window,Key::Down,PhysicalKey::ArrowDown);
        assert!(minute.focus()); key(&window,Key::Down,PhysicalKey::ArrowDown);
        assert!(!hour.classes().contains("expanded")); assert!(minute.classes().contains("expanded"));
        assert_eq!(picker.selected_time(),Some(original));
        key(&window,Key::Enter,PhysicalKey::Enter);
        assert_eq!(picker.selected_time(),Some(SignedDuration::from_hms(14,10,0)));
        picker.set_clock_identifier("12HourClock"); picker.set_use_seconds(true); picker.set_second_increment(15);
        let presenter=open(&window,&picker);
        assert!(part::<Grid>(&presenter,"PART_PeriodHost").is_visible());
        assert!(part::<Grid>(&presenter,"PART_SecondHost").is_visible());
        let wheels=part::<Grid>(&presenter,"PART_PickerContainer");
        assert!(wheels.bounds().width<=320.0,"the native optional seconds extension fits four columns");
        let seconds=part::<DateTimePickerPanel>(&presenter,"PART_SecondSelector");
        assert_eq!(seconds.increment(),15);
        assert!(seconds.focus()); key(&window,Key::Down,PhysicalKey::ArrowDown);
        close_phone_pickers(); frame();
        assert!(!has_open_phone_picker());
        assert!(!part::<Popup>(&picker,"PART_Popup").is_open());
        assert_eq!(picker.selected_time(),Some(SignedDuration::from_hms(14,10,0)),"privacy teardown never commits a staged time");
        open(&window,&picker); window.close(); frame();
        assert!(!has_open_phone_picker(),"owner unload tears down the popup registry");
        owner.dispose();
    },CancellationToken::none()).wait_timeout(Duration::from_secs(30)).expect("native time page regression completes");
    session.dispose();
}
