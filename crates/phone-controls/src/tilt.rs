//! Location-sensitive press feedback on the content of a native Button.
//! The Button retains its input rectangle, capture and command semantics.
use ferroui_base::animation::easings::CubicEaseOut;
use ferroui_base::animation::{DoubleTransition, ITransition, SignedDuration, Transitions};
use ferroui_base::input::{HoldingState, InputElement};
use ferroui_base::interactivity::RoutingStrategies;
use ferroui_base::media::{ITransform, Rotate3DTransform, ScaleTransform, TransformGroup};
use ferroui_base::{Point, Ref, RelativePoint, WeakRef};
use ferroui_controls::{Button, Control};
use ferroui_view::on_cleanup;
use std::{cell::Cell, rc::Rc};

struct Tilt {
    content: WeakRef<Control>,
    transform: Ref<Rotate3DTransform>,
    scale: Ref<ScaleTransform>,
    projected: Rc<dyn ITransform>,
    pointer: Cell<Option<i32>>,
    enabled: Box<dyn Fn() -> bool>,
}

/// Attach once from a declarative `with` callback. Only the content projection
/// moves; a press never substitutes application input for the native Button.
pub fn attach_press_tilt(button: &Ref<Button>, enabled: impl Fn() -> bool + 'static) {
    let Some(content) = button.content().as_ref().and_then(Control::from_boxed) else {
        return;
    };
    let transform = Rotate3DTransform::new();
    let scale = ScaleTransform::new();
    let group = TransformGroup::new();
    group.children().add(scale.clone().upcast());
    group.children().add(transform.clone().upcast());
    transform.enable_transitions();
    scale.enable_transitions();
    let state = Rc::new(Tilt {
        content: content.downgrade(),
        projected: group.into(),
        transform,
        scale,
        pointer: Cell::new(None),
        enabled: Box::new(enabled),
    });
    let weak = button.downgrade();
    let pressed_state = state.clone();
    let pressed = button.add_handler_with(
        InputElement::pointer_pressed_event(),
        move |_, event| {
            let Some(button) = weak.upgrade() else { return };
            if !crate::motion_enabled()
                || !(pressed_state.enabled)()
                || pressed_state.pointer.get().is_some()
            {
                return;
            }
            if button.is_pressed()
                && event
                    .get_current_point(Some(&*button))
                    .properties
                    .is_left_button_pressed
            {
                pressed_state.pointer.set(Some(event.pointer().id()));
                pressed_state.press(
                    event.get_position(Some(&*button)),
                    button.bounds().width,
                    button.bounds().height,
                );
            }
        },
        RoutingStrategies::BUBBLE,
        true,
    );
    let weak = button.downgrade();
    let moved_state = state.clone();
    let moved = button.add_handler_with(
        InputElement::pointer_moved_event(),
        move |_, event| {
            let Some(button) = weak.upgrade() else { return };
            if moved_state.pointer.get() == Some(event.pointer().id()) {
                let point = event.get_position(Some(&*button));
                let size = button.bounds().size();
                if button.is_pressed()
                    && point.x >= 0.0
                    && point.y >= 0.0
                    && point.x <= size.width
                    && point.y <= size.height
                {
                    moved_state.press(point, size.width, size.height);
                } else {
                    moved_state.reset(false);
                }
            }
        },
        RoutingStrategies::BUBBLE,
        true,
    );
    let changed_state = state.clone();
    let changed = button.property_changed(move |change| {
        if change.property() == Button::is_pressed_property().as_property()
            && !change.get_new_value::<bool>()
        {
            changed_state.reset(true);
        }
    });
    let holding_state = state.clone();
    let held = button.add_handler_with(
        InputElement::holding_event(),
        move |_, event| {
            if event.holding_state() == HoldingState::Started {
                holding_state.reset(false);
            }
        },
        RoutingStrategies::BUBBLE,
        true,
    );
    let unloaded_state = state.clone();
    let unloaded = button.add_handler(Control::unloaded_event(), move |_, _| {
        unloaded_state.reset(false)
    });
    let weak = button.downgrade();
    on_cleanup(move || {
        changed.dispose();
        state.reset(false);
        if let Some(button) = weak.upgrade() {
            button.remove_handler(InputElement::pointer_pressed_event(), pressed);
            button.remove_handler(InputElement::pointer_moved_event(), moved);
            button.remove_handler(InputElement::holding_event(), held);
            button.remove_handler(Control::unloaded_event(), unloaded);
        }
    });
}

impl Tilt {
    fn press(&self, point: Point, width: f64, height: f64) {
        if !crate::motion_enabled() || !(self.enabled)() {
            self.reset(false);
            return;
        }
        let Some(content) = self.content.upgrade() else {
            return;
        };
        if content
            .render_transform()
            .is_some_and(|current| !Rc::ptr_eq(&current, &self.projected))
        {
            return;
        }
        self.transform.set_transitions(None);
        self.scale.set_transitions(None);
        self.transform.set_depth(width.max(height).max(1.0) * 6.0);
        let (x, y) = angles(point, width, height);
        self.transform.set_angle_x(x);
        self.transform.set_angle_y(y);
        self.scale.set_scale_x(0.975);
        self.scale.set_scale_y(0.975);
        content.set_render_transform_origin(RelativePoint::CENTER);
        content.set_render_transform(Some(self.projected.clone()));
    }

    fn reset(&self, animate: bool) {
        if self.pointer.take().is_none()
            && self.transform.angle_x() == 0.0
            && self.transform.angle_y() == 0.0
            && self.scale.scale_x() == 1.0
        {
            return;
        }
        let transitions = |properties: [&'static ferroui_base::FerroProperty; 2]| {
            (animate && crate::motion_enabled()).then(|| {
                let transition = |property| {
                    let transition = DoubleTransition::with_property(
                        property,
                        SignedDuration::from_seconds(0.12),
                    );
                    transition.set_easing(CubicEaseOut::new());
                    Rc::<dyn ITransition>::from(transition)
                };
                Transitions::from_items(properties.map(transition))
            })
        };
        self.transform.set_transitions(transitions([
            Rotate3DTransform::angle_x_property().as_property(),
            Rotate3DTransform::angle_y_property().as_property(),
        ]));
        self.scale.set_transitions(transitions([
            ScaleTransform::scale_x_property().as_property(),
            ScaleTransform::scale_y_property().as_property(),
        ]));
        self.transform.set_angle_x(0.0);
        self.transform.set_angle_y(0.0);
        self.scale.set_scale_x(1.0);
        self.scale.set_scale_y(1.0);
        if !animate {
            if let Some(content) = self.content.upgrade() {
                if content
                    .render_transform()
                    .is_some_and(|current| Rc::ptr_eq(&current, &self.projected))
                {
                    content.set_render_transform(None);
                }
            }
        }
    }
}

fn angles(point: Point, width: f64, height: f64) -> (f64, f64) {
    let component = |value: f64, length: f64| {
        if !value.is_finite() || !length.is_finite() || length <= 0.0 {
            0.0
        } else {
            (value / length * 2.0 - 1.0).clamp(-1.0, 1.0) * 6.0
        }
    };
    (-component(point.y, height), component(point.x, width))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn press_projection_is_symmetric_and_bounded_for_all_tile_sizes() {
        for (w, h) in [(86.0, 86.0), (180.0, 180.0), (368.0, 180.0)] {
            assert_eq!(angles(Point::new(w / 2.0, h / 2.0), w, h), (0.0, 0.0));
            assert_eq!(angles(Point::new(0.0, 0.0), w, h), (6.0, -6.0));
            assert_eq!(angles(Point::new(w, h), w, h), (-6.0, 6.0));
            assert_eq!(angles(Point::new(w * 2.0, -h), w, h), (6.0, 6.0));
        }
        assert_eq!(
            angles(Point::new(f64::NAN, 10.0), 0.0, f64::INFINITY),
            (0.0, 0.0)
        );
    }
}
