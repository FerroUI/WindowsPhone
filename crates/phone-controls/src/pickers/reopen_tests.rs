//! A new popup transaction must discard every unconfirmed native selector,
//! including the DatePickerPresenter's private staged date.
use super::*;

fn selected_text(panel: &DateTimePickerPanel) -> Option<String> {
    panel
        .children()
        .snapshot()
        .iter()
        .filter_map(|child| child.cast::<ListBoxItem>())
        .find(|item| item.is_selected())
        .and_then(|item| item.content())
        .and_then(|content| {
            (&*content as &dyn AnyValue)
                .downcast_ref::<String>()
                .cloned()
        })
}

fn cancel(window: &Window, mode: &str) {
    match mode {
        "Back" => assert!(dismiss_phone_picker()),
        "Escape" => key(window, Key::Escape, PhysicalKey::Escape),
        "privacy" => close_phone_pickers(),
        _ => unreachable!(),
    }
    frame();
    assert!(!has_open_phone_picker());
}

fn step(window: &Window, panel: &DateTimePickerPanel, down: bool) {
    assert!(panel.focus());
    if down {
        key(window, Key::Down, PhysicalKey::ArrowDown);
    } else {
        key(window, Key::Up, PhysicalKey::ArrowUp);
    }
}

#[test]
fn native_date_reopen_restores_ranges_formats_hidden_fields_and_full_committed_date() {
    let session = HeadlessUnitTestSession::start_new(|| {
        AppBuilder::configure::<Application>().use_headless(FerroHeadlessPlatformOptions::default())
    });
    session.dispatch(|| {
        crate::set_motion_enabled(false);
        crate::install_theme(&Application::current().unwrap());
        for hidden in ["none", "Day", "Month", "Year"] {
            let (month_value, day_value) = if hidden == "none" || hidden == "Day" { (1, 31) } else { (2, 29) };
            let original = DateTimeOffset::new(2024, month_value, day_value, 13, 14, 15, SignedDuration::from_minutes(330.0));
            let (root, owner) = rsx! {Grid {name:"PhonePage",
                DatePicker {class:"phone-picker",margin:20.0,
                    vertical_alignment:ferroui_base::layout::VerticalAlignment::Top,
                    selected_date:Some(original),min_year:date(2020,1,1),max_year:date(2030,12,31),
                    day_format:"dddd",month_format:"MMMM",year_format:"yyyy",
                    day_visible:hidden!="Day",month_visible:hidden!="Month",year_visible:hidden!="Year",
                    with:|picker|attach_picker_focus(picker)}
            }}.mount_root();
            let window = Window::new();
            window.set_width(400.0);
            window.set_height(700.0);
            window.set_content(Some(Control::boxed(root.clone())));
            window.show();
            frame();
            let picker = root.find_descendant_of_type::<DatePicker>(false).unwrap();
            let presenter = open(&window, &picker);
            let year = part::<DateTimePickerPanel>(&presenter, "PART_YearSelector");
            let month = part::<DateTimePickerPanel>(&presenter, "PART_MonthSelector");
            let day = part::<DateTimePickerPanel>(&presenter, "PART_DaySelector");
            let panels = [&year, &month, &day];
            let baseline = panels.map(|panel| (panel.selected_value(), panel.minimum_value(), panel.maximum_value(), selected_text(panel)));
            for (index, name) in ["Year", "Month", "Day"].iter().enumerate() {
                if *name != hidden {
                    assert!(baseline[index].3.as_ref().is_some_and(|text| !text.is_empty()), "visible {name} has native formatted content");
                }
            }

            for mode in ["Back", "Escape", "privacy"] {
                if hidden != "Month" {
                    step(&window, &month, true);
                }
                if hidden != "Year" {
                    step(&window, &year, false);
                }
                if hidden == "none" {
                    assert_eq!((year.selected_value(), month.selected_value(), day.selected_value()), (2023, 2, 28));
                    assert_eq!(day.maximum_value(), 28, "native staging clamps Jan31 through leap and nonleap February");
                }
                assert_eq!(picker.selected_date(), Some(original));
                cancel(&window, mode);

                let reopened = open(&window, &picker);
                assert!(Ref::ptr_eq(&presenter, &reopened), "{mode} reuses the native presenter, hidden={hidden}");
                for (index, name) in ["Year", "Month", "Day"].iter().enumerate() {
                    let current = part::<DateTimePickerPanel>(&reopened, &format!("PART_{name}Selector"));
                    assert!(Ref::ptr_eq(panels[index], &current));
                    assert_eq!((current.selected_value(), current.minimum_value(), current.maximum_value(), selected_text(&current)), baseline[index],
                        "{mode} restores native range and formatted date context for {name}, hidden={hidden}");
                }
                assert_eq!((year.minimum_value(), year.maximum_value()), (2020, 2030));
                // This checks private sync_date too: merely repainting the
                // visible wheels would still commit the canceled date here.
                key(&window, Key::Enter, PhysicalKey::Enter);
                assert_eq!(picker.selected_date(), Some(original), "accept without editing preserves the full original time and offset after {mode}, hidden={hidden}");
                let restored = picker.selected_date().unwrap();
                assert_eq!(restored.date_time(), original.date_time(), "local date/time survives {mode}, hidden={hidden}");
                assert_eq!(restored.offset(), original.offset(), "the committed offset survives {mode}, hidden={hidden}");
                assert!(!has_open_phone_picker());
                assert!(Ref::ptr_eq(&presenter, &open(&window, &picker)), "accept/reopen keeps the same presenter");
            }

            if hidden == "none" {
                step(&window, &month, true);
                assert_eq!(day.selected_value(), 29);
                let accepted = DateTimeOffset::new(2024, 2, 29, 0, 0, 0, original.offset());
                let accepted_text = selected_text(&day);
                key(&window, Key::Enter, PhysicalKey::Enter);
                assert_eq!(picker.selected_date(), Some(accepted));
                open(&window, &picker);
                assert_eq!((year.selected_value(), month.selected_value(), day.selected_value()), (2024, 2, 29));
                assert_eq!(selected_text(&day), accepted_text);
                step(&window, &month, true);
                cancel(&window, "Back");
                open(&window, &picker);
                key(&window, Key::Enter, PhysicalKey::Enter);
                assert_eq!(picker.selected_date(), Some(accepted), "a later cancellation restores the newly accepted date");
            }
            close_phone_pickers();
            window.close();
            owner.dispose();
        }
    }, CancellationToken::none()).wait_timeout(Duration::from_secs(30)).expect("native date reopen regressions complete");
    session.dispose();
}

#[test]
fn native_time_reopen_preserves_clock_increments_seconds_and_retained_parts() {
    let session = HeadlessUnitTestSession::start_new(|| {
        AppBuilder::configure::<Application>().use_headless(FerroHeadlessPlatformOptions::default())
    });
    session
        .dispatch(
            || {
                crate::set_motion_enabled(false);
                crate::install_theme(&Application::current().unwrap());
                for (clock, use_seconds, original, expected, accepted) in [
                    (
                        "24HourClock",
                        false,
                        SignedDuration::from_hms(7, 0, 0),
                        [7, 0, 0, 0],
                        SignedDuration::from_hms(8, 0, 0),
                    ),
                    (
                        "24HourClock",
                        true,
                        SignedDuration::from_hms(23, 55, 45),
                        [23, 55, 45, 1],
                        SignedDuration::from_hms(0, 55, 45),
                    ),
                    (
                        "12HourClock",
                        true,
                        SignedDuration::from_hms(0, 5, 30),
                        [12, 5, 30, 0],
                        SignedDuration::from_hms(1, 5, 30),
                    ),
                    (
                        "12HourClock",
                        true,
                        SignedDuration::from_hms(12, 5, 30),
                        [12, 5, 30, 1],
                        SignedDuration::from_hms(13, 5, 30),
                    ),
                    (
                        "12HourClock",
                        false,
                        SignedDuration::from_hms(13, 5, 0),
                        [1, 5, 0, 1],
                        SignedDuration::from_hms(14, 5, 0),
                    ),
                ] {
                    let (root, owner) = rsx! {Grid {name:"PhonePage",
                TimePicker {class:"phone-picker",margin:20.0,
                    vertical_alignment:ferroui_base::layout::VerticalAlignment::Top,
                    selected_time:Some(original),clock_identifier:clock,use_seconds:use_seconds,
                    minute_increment:5,second_increment:15,
                    with:|picker|attach_picker_focus(picker)}
            }}.mount_root();
                    let window = Window::new();
                    window.set_width(400.0);
                    window.set_height(700.0);
                    window.set_content(Some(Control::boxed(root.clone())));
                    window.show();
                    frame();
                    let picker = root.find_descendant_of_type::<TimePicker>(false).unwrap();
                    let presenter = open(&window, &picker);
                    let hour = part::<DateTimePickerPanel>(&presenter, "PART_HourSelector");
                    let minute = part::<DateTimePickerPanel>(&presenter, "PART_MinuteSelector");
                    let second = part::<DateTimePickerPanel>(&presenter, "PART_SecondSelector");
                    let period = part::<DateTimePickerPanel>(&presenter, "PART_PeriodSelector");
                    let panels = [&hour, &minute, &second, &period];
                    assert_eq!(panels.map(|panel| panel.selected_value()), expected);

                    for mode in ["Back", "Escape", "privacy"] {
                        step(&window, &hour, true);
                        step(&window, &minute, true);
                        if use_seconds {
                            step(&window, &second, true);
                        }
                        if clock == "12HourClock" {
                            step(&window, &period, expected[3] == 0);
                        }
                        assert_ne!(panels.map(|panel| panel.selected_value()), expected);
                        assert_eq!(picker.selected_time(), Some(original));
                        cancel(&window, mode);
                        let reopened = open(&window, &picker);
                        assert!(Ref::ptr_eq(&presenter, &reopened));
                        for (index, name) in
                            ["Hour", "Minute", "Second", "Period"].iter().enumerate()
                        {
                            assert!(Ref::ptr_eq(
                                panels[index],
                                &part::<DateTimePickerPanel>(
                                    &reopened,
                                    &format!("PART_{name}Selector")
                                )
                            ));
                        }
                        assert_eq!(
                            panels.map(|panel| panel.selected_value()),
                            expected,
                            "{mode} restores {clock} with seconds={use_seconds}"
                        );
                        assert_eq!((minute.increment(), second.increment()), (5, 15));
                        key(&window, Key::Enter, PhysicalKey::Enter);
                        assert_eq!(
                            picker.selected_time(),
                            Some(original),
                            "accept after {mode} does not commit canceled wheel values"
                        );
                        assert!(!has_open_phone_picker());
                        assert!(Ref::ptr_eq(&presenter, &open(&window, &picker)));
                    }
                    step(&window, &hour, true);
                    key(&window, Key::Enter, PhysicalKey::Enter);
                    assert_eq!(picker.selected_time(), Some(accepted));
                    let accepted_parts = panels.map(|panel| panel.selected_value());
                    open(&window, &picker);
                    assert_eq!(panels.map(|panel| panel.selected_value()), accepted_parts);
                    step(&window, &minute, true);
                    cancel(&window, "Back");
                    open(&window, &picker);
                    key(&window, Key::Enter, PhysicalKey::Enter);
                    assert_eq!(
                        picker.selected_time(),
                        Some(accepted),
                        "a later cancellation restores the newly accepted time"
                    );
                    window.close();
                    owner.dispose();
                }
            },
            CancellationToken::none(),
        )
        .wait_timeout(Duration::from_secs(30))
        .expect("native time reopen regressions complete");
    session.dispose();
}
