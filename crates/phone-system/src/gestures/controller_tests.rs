use super::*;
use ferroui_base::animation::SignedDuration;
use ferroui_base::input::{
    Key, KeyModifiers, PhysicalKey, PointerPointProperties, RawInputModifiers,
};
use ferroui_base::interactivity::RoutingStrategies;
use ferroui_base::threading::{CancellationToken, Dispatcher};
use ferroui_base::{AnyValue, ObjectType, Point};
use ferroui_controls::primitives::{DateTimePickerPanel, Popup, TemplatedControl};
use ferroui_controls::{
    AppBuilder, Application, Button, ListBoxItem, ScrollViewer, TimePicker, Window,
};
use ferroui_headless::{
    FerroHeadlessPlatform, FerroHeadlessPlatformExtensions, FerroHeadlessPlatformOptions,
    HeadlessUnitTestSession, HeadlessWindowExtensions,
};
use ferroui_view::prelude::*;
use phone_domain::{AppId, PhoneData};
use phone_sdk::{AppRegistration, AppRegistry};

#[test]
fn native_surface_keeps_touch_release_and_capture() {
    let root = Grid::new();
    let embedded = Grid::new();
    embedded.classes().add("phone-native-input");
    let vm = PhoneViewModel::new(PhoneData::default(), Rc::new(|_| {}));
    let registry = Rc::new(phone_sdk::AppRegistry::new(Vec::new()).unwrap());
    let ui = Rc::new(Session::new(vm.now(), registry));
    let controller = Controller::new(&root, &vm, &ui);

    for native in [
        embedded.upcast::<Control>(),
        Slider::new().upcast(),
        ToggleSwitch::new().upcast(),
    ] {
        root.children().add(native.clone());
        // Primary and secondary native contacts must both keep ownership.
        for primary in [true, false] {
            let pointer: Rc<dyn IPointer> =
                Pointer::new(Pointer::get_next_free_id(), PointerType::Touch, primary);
            let captured = native.clone().upcast::<InputElement>();
            pointer.capture(Some(&captured));
            let pressed = PointerPressedEventArgs::new(
                native.clone(),
                pointer.clone(),
                &root,
                Point::new(40.0, 80.0),
                1,
                PointerPointProperties {
                    is_left_button_pressed: true,
                    ..Default::default()
                },
                KeyModifiers::NONE,
                1,
            );
            controller.pressed(&pressed);
            assert!(!pressed.handled());
            assert!(controller.contacts.borrow().is_empty());
            let released = PointerReleasedEventArgs::new(
                native.clone(),
                pointer.clone(),
                &root,
                Point::new(70.0, 80.0),
                80,
                PointerPointProperties::default(),
                KeyModifiers::NONE,
                MouseButton::Left,
            );
            controller.released(&released);
            assert!(
                !released.handled(),
                "the native surface must receive release"
            );
            assert_eq!(pointer.captured(), Some(captured));
            pointer.capture(None);
        }
    }
}

fn frame() {
    Dispatcher::ui_thread().run_jobs(None);
    FerroHeadlessPlatform::force_render_timer_tick(1);
    Dispatcher::ui_thread().run_jobs(None);
}

fn picker_part<T: ObjectType>(owner: &TemplatedControl, name: &str) -> Ref<T> {
    owner
        .get_template_descendants()
        .into_iter()
        .find(|part| part.has_name(name))
        .and_then(|part| part.cast::<T>())
        .unwrap_or_else(|| panic!("missing native picker part {name}"))
}

fn minute_cell(panel: &DateTimePickerPanel, value: i32) -> Ref<ListBoxItem> {
    panel
        .children()
        .snapshot()
        .iter()
        .filter_map(|child| child.cast::<ListBoxItem>())
        .find(|item| {
            item.tag()
                .is_some_and(|tag| (&*tag as &dyn AnyValue).downcast_ref::<i32>() == Some(&value))
        })
        .unwrap_or_else(|| panic!("minute {value} is visible in the native wheel"))
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
fn cancelled_picker_contact_preserves_the_next_native_drag_through_the_shell() {
    let session = HeadlessUnitTestSession::start_new(|| {
        AppBuilder::configure::<Application>().use_headless(FerroHeadlessPlatformOptions::default())
    });
    session
        .dispatch(
            || {
                phone_controls::install_theme(&Application::current().unwrap());
                let mut data = PhoneData::default();
                data.settings.lock_timeout = 0;
                data.settings.reduced_motion = true;
                data.settings.live_tiles = false;
                data.events.clear();
                let vm = PhoneViewModel::new(data, Rc::new(|_| {}));
                vm.set_viewport(320.0, 740.0);
                vm.set_clock(phone_domain::days_from_date(2026, 11, 24) * 86_400 + 12 * 3_600);
                vm.set_utc_offset(0);
                vm.navigate(Route::App(AppId::Alarms));
                let registry = Rc::new(
                    AppRegistry::new(vec![AppRegistration::builtin(AppId::Alarms, |_, _| {
                        rsx! {Grid {
                            TimePicker {class:"phone-picker",margin:20.0,
                                vertical_alignment:ferroui_base::layout::VerticalAlignment::Top,
                                selected_time:Some(SignedDuration::from_hms(7, 0, 0)),
                                clock_identifier:"24HourClock",
                                with:|picker|phone_controls::attach_picker_focus(picker)}
                        }}
                    })])
                    .unwrap(),
                );
                // Use the production shell, not just an isolated picker. A popup
                // press reaches PhoneScreen through its logical owner; direct
                // capture loss follows only the overlay's visual ancestors.
                let root = crate::create_view(vm.clone(), registry);
                let window = Window::new();
                window.set_width(320.0);
                window.set_height(740.0);
                window.set_content(Some(Control::boxed(root.clone())));
                window.show();
                frame();
                let controller = crate::gestures::ACTIVE
                    .with(|slot| slot.borrow().upgrade())
                    .expect("the production shell installed its gesture controller");
                let screen = controller.root.upgrade().unwrap();
                let touch_presses = Rc::new(Cell::new(0));
                let observed = touch_presses.clone();
                let routed_pointer = Rc::new(RefCell::new(None::<Rc<dyn IPointer>>));
                let observed_pointer = routed_pointer.clone();
                screen.add_handler_with(
                    InputElement::pointer_pressed_event(),
                    move |_, event| {
                        if event.pointer().type_() == PointerType::Touch {
                            observed.set(observed.get() + 1);
                            *observed_pointer.borrow_mut() = Some(event.pointer().clone());
                        }
                    },
                    RoutingStrategies::TUNNEL,
                    true,
                );
                let picker = root.find_descendant_of_type::<TimePicker>(false).unwrap();
                let popup = picker_part::<Popup>(&picker, "PART_Popup");
                let presenter = popup
                    .child()
                    .and_then(|child| child.cast::<TemplatedControl>())
                    .unwrap();
                assert!(picker_part::<Button>(&picker, "PART_FlyoutButton").focus());
                window.key_press(Key::Space, RawInputModifiers::NONE, PhysicalKey::Space, None);
                window.key_release(Key::Space, RawInputModifiers::NONE, PhysicalKey::Space, None);
                frame();
                assert!(popup.is_open() && phone_controls::has_open_phone_picker());
                let minute = picker_part::<DateTimePickerPanel>(&presenter, "PART_MinuteSelector");
                let scroll = minute
                    .get_visual_ancestors()
                    .find_map(|ancestor| ancestor.cast::<ScrollViewer>())
                    .unwrap();
                assert!(scroll.is_scroll_inertia_enabled(), "retain native scrolling physics");
                let original = picker.selected_time();
                let selected = center(&minute_cell(&minute, 0), &window);
                window.mouse_move(selected, RawInputModifiers::NONE);
                window.mouse_down(selected, MouseButton::Left, RawInputModifiers::NONE);
                window.mouse_up(selected, MouseButton::Left, RawInputModifiers::NONE);
                frame();
                assert!(minute.classes().contains("expanded"));
                assert_eq!(minute.selected_value(), 0);

                // Match the browser sequence: cancel a candidate in the next
                // cell, then drag the selected cell through eight small moves.
                let candidate = center(&minute_cell(&minute, 1), &window);
                let cancelled = window.touch_begin(candidate, RawInputModifiers::NONE);
                cancelled.dispose();
                frame();
                let stale_contacts = controller.contacts.borrow().len();
                assert_eq!(minute.selected_value(), 0, "cancellation cannot stage minute 01");
                let start = center(&minute_cell(&minute, 0), &window);
                let offset_before = scroll.offset().y;
                let touch = window.touch_begin(start, RawInputModifiers::NONE);
                let pointer = routed_pointer.borrow().clone().unwrap();
                let native = pointer.as_any().downcast_ref::<Pointer>().unwrap();
                let skipped_at_press = native.is_gesture_recognition_skipped();
                let mut samples = Vec::new();
                let mut end = start;
                for step in 1..=8 {
                    end = Point::new(start.x, start.y - minute.item_height() * 1.25 * step as f64 / 8.0);
                    window.touch_move(&*touch, end, RawInputModifiers::NONE);
                    frame();
                    samples.push((minute.selected_value(), scroll.offset().y));
                }
                let native_capture = native.captured_gesture_recognizer().is_some();
                let contacts_during_drag = controller.contacts.borrow().len();
                let moved = samples.iter().any(|(_, offset)| (*offset - offset_before).abs() > 1.0);
                let staged = minute.selected_value();
                window.touch_end(&*touch, end, RawInputModifiers::NONE);
                frame();
                assert_eq!(picker.selected_time(), original, "dragging only stages the native presenter");
                assert_eq!(vm.route(), Route::App(AppId::Alarms));
                crate::host_closing(&vm);
                window.close();
                window.set_content(None);
                frame();

                assert_eq!(touch_presses.get(), 2, "both popup contacts actually traverse the shell's routed handler");
                assert!(moved && staged != 0 && native_capture && !skipped_at_press,
                    "native drag must move before release: stale contacts={stale_contacts}, active shell contacts={contacts_during_drag}, native capture={native_capture}, skipped at press={skipped_at_press}, initial offset={offset_before}, samples={samples:?}");
                assert_eq!(stale_contacts, 0, "the shell must not retain a canceled picker contact");
                assert_eq!(contacts_during_drag, 0, "the picker owns its full pointer lifetime");
            },
            CancellationToken::none(),
        )
        .wait_timeout(Duration::from_secs(30))
        .expect("integrated shell/picker cancellation regression completes");
    session.dispose();
}

#[test]
fn committed_shell_pan_does_not_start_a_competing_snap_back_on_release() {
    let session = HeadlessUnitTestSession::start_new(|| {
        AppBuilder::configure::<Application>().use_headless(FerroHeadlessPlatformOptions::default())
    });
    session
        .dispatch(
            || {
                phone_controls::set_motion_enabled(true);
                for (from, displacement, expected, settles) in [
                    (Route::Start, -130.0, Route::Apps, false),
                    (Route::Apps, 130.0, Route::Start, false),
                    (Route::Start, -20.0, Route::Start, true),
                    (Route::Apps, -130.0, Route::Apps, true),
                ] {
                    let vm = PhoneViewModel::new(PhoneData::default(), Rc::new(|_| {}));
                    vm.set_viewport(320.0, 740.0);
                    if from != Route::Start {
                        vm.navigate(from.clone());
                    }
                    let ui = Rc::new(Session::new(
                        vm.now(), Rc::new(AppRegistry::new(Vec::new()).unwrap()),
                    ));
                    let root = Grid::new();
                    let page = ferroui_controls::Border::new();
                    page.set_name(Some("PhonePage".into()));
                    root.children().add(page.clone());
                    let window = Window::new();
                    window.set_width(320.0);
                    window.set_height(740.0);
                    window.set_content(Some(Control::boxed(root.clone())));
                    window.show();
                    frame();
                    let controller = Controller::new(&root, &vm, &ui);
                    let pointer: Rc<dyn IPointer> =
                        Pointer::new(Pointer::get_next_free_id(), PointerType::Touch, true);
                    let x = if displacement < 0.0 { 240.0 } else { 60.0 };
                    controller.pressed(&PointerPressedEventArgs::new(
                        page.clone(), pointer.clone(), &root, Point::new(x, 100.0), 1,
                        PointerPointProperties { is_left_button_pressed: true, ..Default::default() },
                        KeyModifiers::NONE, 1,
                    ));
                    let move_event = PointerEventArgs::new(
                        Some(InputElement::pointer_moved_event()), page.clone(), pointer.clone(),
                        Some(&root), Point::new(x + displacement, 100.0), 200,
                        PointerPointProperties { is_left_button_pressed: true, ..Default::default() },
                        KeyModifiers::NONE,
                    );
                    controller.moved(&move_event);
                    assert!(page.translation().x.abs() > 1.0, "the current page follows the finger");
                    let released = PointerReleasedEventArgs::new(
                        page.clone(), pointer.clone(), &root,
                        Point::new(x + displacement, 100.0), 400,
                        PointerPointProperties::default(), KeyModifiers::NONE, MouseButton::Left,
                    );
                    controller.released(&released);
                    assert!(released.handled(), "the swiped row cannot receive a click");
                    assert_eq!(vm.route(), expected);
                    assert_eq!(page.translation().x, 0.0, "raw pointer translation is restored before capture");
                    assert_eq!(page.render_transform().is_some(), settles,
                        "only a rejected or bounded-edge pan starts a snap-back ({from:?}, {displacement})");
                    assert!(controller.contacts.borrow().is_empty());
                    assert!(pointer.captured().is_none());
                    controller.cancel();
                    window.close();
                    window.set_content(None);
                    frame();
                }
            },
            CancellationToken::none(),
        )
        .wait_timeout(Duration::from_secs(30))
        .expect("shell paging release regression completes");
    session.dispose();
}
