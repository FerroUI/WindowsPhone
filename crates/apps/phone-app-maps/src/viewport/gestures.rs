use super::{model::Camera, State};
use ferroui_base::{
    input::{
        IPointer, InputElement, Pointer, PointerEventArgs, PointerPressedEventArgs,
        PointerReleasedEventArgs,
    },
    Point,
};
use ferroui_controls::Canvas;
use ferroui_view::prelude::*;
use phone_sdk::AppContext;
use std::{collections::BTreeMap, rc::Rc};

struct Contact {
    point: Point,
    pointer: Rc<dyn IPointer>,
}
#[derive(Default)]
pub(super) struct Gesture {
    contacts: BTreeMap<i32, Contact>,
    base: Camera,
    origin: Point,
    distance: Option<f64>,
    travel: f64,
    multi: bool,
}
impl Gesture {
    fn rebase(&mut self, camera: Camera) {
        self.base = camera;
        let mut points = self.contacts.values().map(|contact| contact.point);
        let Some(first) = points.next() else {
            return;
        };
        if let Some(second) = points.next() {
            self.origin = Point::new((first.x + second.x) / 2.0, (first.y + second.y) / 2.0);
            self.distance = Some((first.x - second.x).hypot(first.y - second.y).max(1.0));
            self.multi = true;
        } else {
            self.origin = first;
            self.distance = None;
        }
    }
    fn moved(&self, size: (f64, f64)) -> Camera {
        let mut points = self.contacts.values().map(|contact| contact.point);
        let Some(first) = points.next() else {
            return self.base;
        };
        if let Some(second) = points.next() {
            let midpoint = Point::new((first.x + second.x) / 2.0, (first.y + second.y) / 2.0);
            let scale = (first.x - second.x).hypot(first.y - second.y).max(1.0)
                / self.distance.unwrap_or(1.0);
            self.base
                .zoom_at(
                    self.base.zoom + scale.log2(),
                    (self.origin.x, self.origin.y),
                    size,
                )
                .pan(midpoint.x - self.origin.x, midpoint.y - self.origin.y, size)
        } else {
            self.base
                .pan(first.x - self.origin.x, first.y - self.origin.y, size)
        }
    }
    pub fn clear(&mut self) -> Vec<Rc<dyn IPointer>> {
        self.travel = 0.0;
        self.multi = false;
        std::mem::take(&mut self.contacts)
            .into_values()
            .map(|contact| contact.pointer)
            .collect()
    }
}
pub(super) fn cancel(state: &State) {
    let pointers = state.gesture.borrow_mut().clear();
    for pointer in pointers {
        pointer.capture(None);
    }
    state.dragging.set(false);
}
pub(super) fn pressed(state: &Rc<State>, node: NodeRef<Canvas>, event: &PointerPressedEventArgs) {
    if !event.properties().is_left_button_pressed {
        return;
    }
    let Some(canvas) = node.get_untracked() else {
        return;
    };
    let pointer = event.pointer();
    let point = event.get_position(Some(&canvas));
    {
        let mut gesture = state.gesture.borrow_mut();
        if gesture.contacts.len() >= 2 {
            event.set_handled(true);
            return;
        }
        if gesture.contacts.is_empty() {
            gesture.travel = 0.0;
            gesture.multi = false;
        }
        gesture.contacts.insert(
            pointer.id(),
            Contact {
                point,
                pointer: pointer.clone(),
            },
        );
        gesture.rebase(state.camera.get_untracked());
    }
    state.dragging.set(true);
    state.deferred.set(false);
    state.settle_at.set(None);
    canvas.focus();
    pointer.capture(Some(&canvas.upcast::<InputElement>()));
    if let Some(pointer) = pointer.as_any().downcast_ref::<Pointer>() {
        pointer.set_is_gesture_recognition_skipped(true);
    }
    event.set_handled(true);
}
pub(super) fn moved(state: &Rc<State>, node: NodeRef<Canvas>, event: &PointerEventArgs) {
    let Some(canvas) = node.get_untracked() else {
        return;
    };
    let point = event.get_position(Some(&canvas));
    let camera = {
        let mut gesture = state.gesture.borrow_mut();
        let Some(contact) = gesture.contacts.get_mut(&event.pointer().id()) else {
            return;
        };
        let distance = (point.x - contact.point.x).hypot(point.y - contact.point.y);
        contact.point = point;
        gesture.travel += distance;
        gesture.moved(state.size.get_untracked())
    };
    state.camera.set_if_changed(camera);
    event.set_handled(true);
}
pub(super) fn released(
    state: &Rc<State>,
    node: NodeRef<Canvas>,
    cx: &AppContext,
    event: &PointerReleasedEventArgs,
) {
    let Some(canvas) = node.get_untracked() else {
        return;
    };
    let point = event.get_position(Some(&canvas));
    let (finished, tap) = {
        let mut gesture = state.gesture.borrow_mut();
        if gesture.contacts.remove(&event.pointer().id()).is_none() {
            return;
        }
        let finished = gesture.contacts.is_empty();
        let tap = finished && !gesture.multi && gesture.travel < 5.0;
        if !finished {
            gesture.rebase(state.camera.get_untracked());
        }
        (finished, tap)
    };
    event.pointer().capture(None);
    event.set_handled(true);
    if finished {
        state.dragging.set(false);
        if tap {
            let coordinates = state
                .camera
                .get_untracked()
                .coordinate((point.x, point.y), state.size.get_untracked());
            crate::commands::select(
                cx,
                crate::model::Place {
                    id: 0,
                    name: coordinates.label(),
                    address: String::new(),
                    coordinates: Some(coordinates),
                },
            );
        }
    }
}
pub(super) fn lost(state: &Rc<State>, id: i32) {
    let empty = {
        let mut gesture = state.gesture.borrow_mut();
        gesture.contacts.remove(&id);
        if !gesture.contacts.is_empty() {
            gesture.rebase(state.camera.get_untracked());
        }
        gesture.contacts.is_empty()
    };
    if empty {
        state.dragging.set(false);
    }
}
