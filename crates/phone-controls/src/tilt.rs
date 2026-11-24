//! Location-sensitive press feedback on the content of a native Button.
//! The Button retains its input rectangle, capture and command semantics.
use ferroui_base::animation::easings::LinearEasing;
use ferroui_base::animation::{DoubleTransition, ITransition, SignedDuration, Transitions};
use ferroui_base::input::{HoldingState, InputElement};
use ferroui_base::interactivity::RoutingStrategies;
use ferroui_base::media::{ITransform, Rotate3DTransform, ScaleTransform, TransformGroup};
use ferroui_base::{Point, Ref, RelativePoint, WeakRef};
use ferroui_controls::{Button, Control};
use ferroui_view::on_cleanup;
use std::{cell::Cell, rc::Rc};

// Microsoft.Phone.Controls.Toolkit/Effects/TiltEffect.cs, commit
// 4e4fd5d36c769c004f7ccc7daf78a5ffb8ef18df: the angle budget is shared by
// both axes; depression is strongest at the center, zero at a corner.
const MAX_ANGLE_RADIANS: f64 = 0.3;
const MAX_DEPRESSION: f64 = crate::reference_pixels(25.0);
// Silverlight PlaneProjection uses a camera 1000 reference pixels from the
// plane (Microsoft MSDN, "Projection Transforms Sans Math", April 2010).
const CAMERA_DISTANCE: f64 = crate::reference_pixels(1000.0);
const RETURN_DELAY: f64 = 0.2;
const RETURN_DURATION: f64 = 0.1;

struct Tilt {
    content: WeakRef<Control>,
    transform: Ref<Rotate3DTransform>,
    scale: Ref<ScaleTransform>,
    projected: Rc<dyn ITransform>,
    pointer: Cell<Option<i32>>,
    releasing: Cell<bool>,
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
    group.children().add(transform.clone().upcast());
    group.children().add(scale.clone().upcast());
    transform.set_depth(CAMERA_DISTANCE);
    transform.enable_transitions();
    // A rotation at focal length D + depression followed by scale D/(D +
    // depression) is the same perspective projection as translating the
    // plane away from a fixed camera by depression. Derive scale from the
    // animated depth, so the 100 ms return also interpolates Z correctly.
    let weak_scale = scale.downgrade();
    let projection_changed = transform.property_changed(move |change| {
        if change.property() == Rotate3DTransform::depth_property().as_property() {
            if let Some(scale) = weak_scale.upgrade() {
                let depth = change.get_new_value::<f64>();
                let factor = if depth.is_finite() && depth >= CAMERA_DISTANCE {
                    CAMERA_DISTANCE / depth
                } else {
                    1.0
                };
                scale.set_scale_x(factor);
                scale.set_scale_y(factor);
            }
        }
    });
    let state = Rc::new(Tilt {
        content: content.downgrade(),
        projected: group.into(),
        transform,
        scale,
        pointer: Cell::new(None),
        releasing: Cell::new(false),
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
                    // An excursion outside pauses the effect; a captured
                    // pointer can reenter without requiring a second press.
                    moved_state.restore(false);
                }
            }
        },
        RoutingStrategies::BUBBLE,
        true,
    );
    let released_state = state.clone();
    let released = button.add_handler_with(
        InputElement::pointer_released_event(),
        move |_, event| {
            if released_state.pointer.get() == Some(event.pointer().id()) {
                released_state.releasing.set(true);
            }
        },
        RoutingStrategies::TUNNEL,
        true,
    );
    let changed_state = state.clone();
    let changed = button.property_changed(move |change| {
        if change.property() == Button::is_pressed_property().as_property()
            && !change.get_new_value::<bool>()
        {
            changed_state.reset(changed_state.releasing.replace(false));
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
    let capture_state = state.clone();
    let capture_lost =
        button.add_handler(InputElement::pointer_capture_lost_event(), move |_, _| {
            if capture_state.pointer.get().is_some() {
                capture_state.reset(false);
            }
        });
    let unloaded_state = state.clone();
    let unloaded = button.add_handler(Control::unloaded_event(), move |_, _| {
        unloaded_state.reset(false)
    });
    let weak = button.downgrade();
    on_cleanup(move || {
        changed.dispose();
        projection_changed.dispose();
        state.reset(false);
        if let Some(button) = weak.upgrade() {
            button.remove_handler(InputElement::pointer_pressed_event(), pressed);
            button.remove_handler(InputElement::pointer_moved_event(), moved);
            button.remove_handler(InputElement::pointer_released_event(), released);
            button.remove_handler(InputElement::holding_event(), held);
            button.remove_handler(InputElement::pointer_capture_lost_event(), capture_lost);
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
        let projection = projection(point, width, height);
        self.transform
            .set_depth(CAMERA_DISTANCE + projection.depression);
        self.transform.set_angle_x(projection.x);
        self.transform.set_angle_y(projection.y);
        content.set_render_transform_origin(RelativePoint::CENTER);
        content.set_render_transform(Some(self.projected.clone()));
    }

    fn reset(&self, animate: bool) {
        self.pointer.set(None);
        self.releasing.set(false);
        self.restore(animate);
    }

    fn restore(&self, animate: bool) {
        if self.transform.angle_x() == 0.0
            && self.transform.angle_y() == 0.0
            && self.scale.scale_x() == 1.0
        {
            return;
        }
        let transitions = |properties: [&'static ferroui_base::FerroProperty; 3]| {
            (animate && crate::motion_enabled()).then(|| {
                let transition = |property| {
                    let transition = DoubleTransition::with_property(
                        property,
                        SignedDuration::from_seconds(RETURN_DURATION),
                    );
                    transition.set_delay(SignedDuration::from_seconds(RETURN_DELAY));
                    transition.set_easing(LinearEasing::new());
                    Rc::<dyn ITransition>::from(transition)
                };
                Transitions::from_items(properties.map(transition))
            })
        };
        self.transform.set_transitions(transitions([
            Rotate3DTransform::angle_x_property().as_property(),
            Rotate3DTransform::angle_y_property().as_property(),
            Rotate3DTransform::depth_property().as_property(),
        ]));
        self.transform.set_angle_x(0.0);
        self.transform.set_angle_y(0.0);
        self.transform.set_depth(CAMERA_DISTANCE);
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

#[derive(Clone, Copy, Debug, Default, PartialEq)]
struct Projection {
    x: f64,
    y: f64,
    depression: f64,
}

fn projection(point: Point, width: f64, height: f64) -> Projection {
    if !point.x.is_finite()
        || !point.y.is_finite()
        || !width.is_finite()
        || !height.is_finite()
        || width <= 0.0
        || height <= 0.0
    {
        return Projection::default();
    }
    let u = (point.x / width).clamp(0.0, 1.0) - 0.5;
    let v = (point.y / height).clamp(0.0, 1.0) - 0.5;
    Projection {
        x: v * MAX_ANGLE_RADIANS.to_degrees(),
        y: -u * MAX_ANGLE_RADIANS.to_degrees(),
        depression: (1.0 - u.abs() - v.abs()) * MAX_DEPRESSION,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn center_press_depresses_without_rotation_and_corner_press_only_rotates() {
        for (w, h) in [(86.0, 86.0), (180.0, 180.0), (368.0, 180.0)] {
            let center = projection(Point::new(w / 2.0, h / 2.0), w, h);
            assert_eq!(
                center,
                Projection {
                    x: 0.0,
                    y: 0.0,
                    depression: MAX_DEPRESSION
                }
            );
            let top_left = projection(Point::new(0.0, 0.0), w, h);
            let bottom_right = projection(Point::new(w, h), w, h);
            assert_eq!(top_left.depression, 0.0);
            assert_eq!(bottom_right.depression, 0.0);
            assert_eq!(top_left.x, -bottom_right.x);
            assert_eq!(top_left.y, -bottom_right.y);
            assert!((top_left.x + 8.594366926962348).abs() < 1e-10);
            assert!((top_left.y - 8.594366926962348).abs() < 1e-10);
            let edge = projection(Point::new(0.0, h / 2.0), w, h);
            assert_eq!(edge.x, 0.0);
            assert_eq!(edge.depression, MAX_DEPRESSION / 2.0);
        }
    }

    #[test]
    fn invalid_or_outside_samples_never_produce_nonfinite_or_unbounded_projection() {
        assert_eq!(
            projection(Point::new(f64::NAN, 10.0), 0.0, f64::INFINITY),
            Projection::default()
        );
        assert_eq!(
            projection(Point::new(10.0, 10.0), -1.0, 80.0),
            Projection::default()
        );
        assert_eq!(
            projection(Point::new(-1000.0, 1000.0), 80.0, 80.0),
            projection(Point::new(0.0, 80.0), 80.0, 80.0)
        );
        for x in 0..=20 {
            for y in 0..=20 {
                let p = projection(Point::new(x as f64 * 4.0, y as f64 * 4.0), 80.0, 80.0);
                assert!(p.x.abs() + p.y.abs() <= MAX_ANGLE_RADIANS.to_degrees() + 1e-10);
                assert!((0.0..=MAX_DEPRESSION).contains(&p.depression));
            }
        }
    }
}
