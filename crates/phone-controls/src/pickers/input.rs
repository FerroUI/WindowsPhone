//! Release-based selection over the native selector's recycled containers.
//!
//! A selector must stage every tap, including rapid repeated taps. The native
//! panel listens only to Tapped, whereas the input recognizer replaces every
//! second tap with DoubleTapped on press. Owning the small tap transaction
//! avoids both a lost second selection and premature selection on a drag.
//! ScrollViewer still owns pointer capture, scrolling, inertia and snapping.
use super::templates::expand;
use ferroui_base::input::{
    IPointer, InputElement, KeyModifiers, MouseButton, NavigationMethod, Pointer,
};
use ferroui_base::interactivity::RoutingStrategies;
use ferroui_base::{AnyValue, Point, Ref, Size, Vector, Visual, WeakRef};
use ferroui_controls::{primitives::DateTimePickerPanel, Grid, ListBoxItem};
use std::{
    cell::RefCell,
    rc::{Rc, Weak},
};

#[derive(Default)]
struct Contacts {
    pointers: Vec<(i32, Weak<dyn IPointer>)>,
    generation: u64,
    suppressed: bool,
}

/// One contact owner for all wheels. Weak pointers keep a captured native drag
/// observable until its physical release, even though the scroll recognizer
/// consumes that release before normal routed pointer handlers see it.
#[derive(Clone)]
pub(super) struct WheelInput(Rc<RefCell<Contacts>>);

fn has_capture(pointer: &dyn IPointer) -> bool {
    pointer.captured().is_some()
        || pointer
            .as_any()
            .downcast_ref::<Pointer>()
            .is_some_and(|pointer| pointer.captured_gesture_recognizer().is_some())
}

impl WheelInput {
    pub(super) fn new(wheels: &Grid) -> Self {
        let input = Self(Rc::new(RefCell::new(Contacts::default())));
        let state = input.clone();
        wheels.add_handler_with(
            InputElement::pointer_capture_lost_event(),
            move |_, event| {
                if !has_capture(&**event.pointer()) {
                    state.finish(event.pointer().id(), None);
                }
            },
            RoutingStrategies::DIRECT,
            true,
        );
        let state = input.clone();
        wheels.unloaded(move |_, _| {
            *state.0.borrow_mut() = Contacts::default();
        });
        input
    }

    fn begin(&self, pointer: &Rc<dyn IPointer>) -> Option<u64> {
        let mut state = self.0.borrow_mut();
        // A mouse is reused across contacts. Any previous capture has ended
        // before its next press, even if a recognizer consumed that release.
        state.pointers.retain(|(id, pointer_ref)| {
            *id != pointer.id()
                && pointer_ref
                    .upgrade()
                    .is_some_and(|pointer| has_capture(&*pointer))
        });
        if state.pointers.is_empty() {
            state.suppressed = false;
        } else {
            state.suppressed = true;
            state.generation = state.generation.wrapping_add(1);
        }
        state.pointers.push((pointer.id(), Rc::downgrade(pointer)));
        (!state.suppressed).then_some(state.generation)
    }

    fn finish(&self, pointer: i32, generation: Option<u64>) -> bool {
        let mut state = self.0.borrow_mut();
        let accepted = !state.suppressed && generation == Some(state.generation);
        state.pointers.retain(|(id, _)| *id != pointer);
        if state.pointers.is_empty() {
            state.suppressed = false;
        }
        accepted
    }
}

struct Tap {
    pointer: i32,
    origin: Point,
    tolerance: Size,
    offset: Vector,
    value: i32,
    item: WeakRef<ListBoxItem>,
    generation: u64,
}

impl Tap {
    fn contains(&self, position: Point) -> bool {
        (position.x - self.origin.x).abs() <= self.tolerance.width
            && (position.y - self.origin.y).abs() <= self.tolerance.height
    }
}

fn value(item: &ListBoxItem) -> Option<i32> {
    item.tag()
        .and_then(|tag| (&*tag as &dyn AnyValue).downcast_ref::<i32>().copied())
}

pub(super) fn attach(wheels: &Ref<Grid>, selector: &Ref<DateTimePickerPanel>, input: &WheelInput) {
    let pending = Rc::new(RefCell::new(None::<Tap>));
    let state = pending.clone();
    let weak_wheels = wheels.downgrade();
    let weak_selector = selector.downgrade();
    let contacts = input.clone();
    selector.add_handler_with(
        InputElement::pointer_pressed_event(),
        move |_, event| {
            state.borrow_mut().take();
            let (Some(wheels), Some(selector)) = (weak_wheels.upgrade(), weak_selector.upgrade())
            else {
                return;
            };
            if !event
                .get_current_point(Some(&selector))
                .properties
                .is_left_button_pressed
            {
                return;
            }
            let Some(generation) = contacts.begin(event.pointer()) else {
                return;
            };
            expand(&wheels, &selector);
            selector.focus_with(NavigationMethod::Pointer, KeyModifiers::NONE);
            let Some(source) = event.source().and_then(|source| source.cast::<Visual>()) else {
                return;
            };
            let Some(item) = std::iter::once(source.clone())
                .chain(source.get_visual_ancestors())
                .find_map(|part| part.cast::<ListBoxItem>())
            else {
                return;
            };
            if !item
                .parent()
                .and_then(|parent| parent.cast::<DateTimePickerPanel>())
                .is_some_and(|parent| Ref::ptr_eq(&parent, &selector))
            {
                return;
            }
            let Some(value) = value(&item) else {
                return;
            };
            *state.borrow_mut() = Some(Tap {
                pointer: event.pointer().id(),
                origin: event.get_position(Some(&selector)),
                tolerance: selector
                    .get_platform_settings()
                    .map(|settings| settings.get_tap_size(event.pointer().type_()))
                    .unwrap_or(Size::new(4.0, 4.0)),
                offset: selector.offset(),
                value,
                item: item.downgrade(),
                generation,
            });
        },
        RoutingStrategies::TUNNEL,
        true,
    );

    let state = pending.clone();
    let weak_selector = selector.downgrade();
    let contacts = input.clone();
    selector.add_handler_with(
        InputElement::pointer_moved_event(),
        move |_, event| {
            let Some(selector) = weak_selector.upgrade() else {
                return;
            };
            let mut tap = state.borrow_mut();
            if tap.as_ref().is_some_and(|tap| {
                tap.pointer == event.pointer().id()
                    && !tap.contains(event.get_position(Some(&selector)))
            }) {
                tap.take();
            }
        },
        RoutingStrategies::TUNNEL,
        true,
    );

    let state = pending.clone();
    let weak_selector = selector.downgrade();
    selector.add_handler_with(
        InputElement::pointer_released_event(),
        move |_, event| {
            let tap = {
                let mut state = state.borrow_mut();
                if state
                    .as_ref()
                    .is_some_and(|tap| tap.pointer == event.pointer().id())
                {
                    state.take()
                } else {
                    None
                }
            };
            let accepted =
                contacts.finish(event.pointer().id(), tap.as_ref().map(|tap| tap.generation));
            let (Some(tap), Some(selector)) = (tap, weak_selector.upgrade()) else {
                return;
            };
            let Some(item) = tap.item.upgrade() else {
                return;
            };
            let position = event.get_position(Some(&item));
            if !accepted
                || event.initial_press_mouse_button() != MouseButton::Left
                || !tap.contains(event.get_position(Some(&selector)))
                || selector.offset() != tap.offset
                || !item.is_attached_to_visual_tree()
                || !selector.is_effectively_enabled()
                || !selector.classes().contains("expanded")
                || !item.is_effectively_visible()
                || !item.is_hit_test_visible()
                || position.x < 0.0
                || position.y < 0.0
                || position.x >= item.bounds().width
                || position.y >= item.bounds().height
                || value(&item) != Some(tap.value)
            {
                return;
            }
            if selector.selected_value() != tap.value {
                selector.set_selected_value(tap.value);
            }
        },
        RoutingStrategies::TUNNEL,
        true,
    );

    let state = pending.clone();
    selector.add_handler_with(
        InputElement::pointer_capture_lost_event(),
        move |_, event| {
            let mut tap = state.borrow_mut();
            if tap
                .as_ref()
                .is_some_and(|tap| tap.pointer == event.pointer().id())
            {
                tap.take();
            }
        },
        RoutingStrategies::DIRECT,
        true,
    );
    let state = pending.clone();
    selector.add_handler_with(
        InputElement::holding_event(),
        move |_, _| {
            state.borrow_mut().take();
        },
        RoutingStrategies::BUBBLE,
        true,
    );
    let state = pending.clone();
    selector.size_changed(move |_, _| {
        state.borrow_mut().take();
    });
    selector.unloaded(move |_, _| {
        pending.borrow_mut().take();
    });
}

/// Tapped only bubbles in FerroUI. Intercept at the item, before its native
/// panel handler, rather than registering a tunnel that will never execute.
/// The marker keeps one pair of handlers for each recycled item's lifetime,
/// even if its visual template is later replaced.
pub(super) fn prepare_item(item: &ListBoxItem) {
    const INSTALLED: &str = "phone-picker-pointer-adapter";
    if item.classes().contains(INSTALLED) {
        return;
    }
    item.classes().add(INSTALLED);
    // After release, recycling changes this item's tag. A second native
    // Tapped transaction would select that new value, not the pressed value.
    for event in [
        InputElement::tapped_event(),
        InputElement::double_tapped_event(),
    ] {
        item.add_handler_with(
            event,
            |_, event| event.set_handled(true),
            RoutingStrategies::BUBBLE,
            true,
        );
    }
}
