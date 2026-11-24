use super::*;
use ferroui_base::input::{KeyModifiers, PointerPointProperties};
use ferroui_base::Point;
use phone_domain::PhoneData;

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
