//! Independent rotation and opacity tracks preserve the Toolkit key times.
use super::{viewport_width, Entrance};
use ferroui_base::animation::easings::{CubicEaseOut, Easing, IEasing, SharedEasing};
use ferroui_base::animation::{Animation, AnimationTask, Cue, IClock, KeyFrame, SignedDuration};
use ferroui_base::media::{ITransform, Rotate3DTransform, TranslateTransform};
use ferroui_base::styling::Setter;
use ferroui_base::threading::CancellationToken;
use ferroui_base::{Ref, RelativePoint, RelativeUnit, Visual};
use ferroui_controls::Control;
use std::{rc::Rc, sync::Arc};

#[derive(Clone, Copy)]
pub(super) struct PhoneExponentialOut;

impl IEasing for PhoneExponentialOut {
    fn ease(&self, progress: f64) -> f64 {
        1.0 - PhoneExponentialIn.ease(1.0 - progress.clamp(0.0, 1.0))
    }
    fn to_shared(&self) -> Arc<SharedEasing> {
        Arc::new(*self)
    }
}

#[derive(Clone, Copy)]
struct PhoneExponentialIn;

impl IEasing for PhoneExponentialIn {
    fn ease(&self, progress: f64) -> f64 {
        ((progress.clamp(0.0, 1.0) * 6.0).exp() - 1.0) / (6.0_f64.exp() - 1.0)
    }
    fn to_shared(&self) -> Arc<SharedEasing> {
        Arc::new(*self)
    }
}

pub(super) fn enter(
    target: &Ref<Control>,
    entrance: Entrance,
    cancellation: CancellationToken,
    clock: Option<Rc<dyn IClock>>,
) -> AnimationTask {
    if entrance == Entrance::None || target.render_transform().is_some() {
        return AnimationTask::completed();
    }
    let animation = Animation::new();
    animation.set_duration(SignedDuration::from_seconds(
        if entrance == Entrance::Reveal {
            0.22
        } else {
            0.35
        },
    ));
    animation.set_easing(if entrance == Entrance::Reveal {
        CubicEaseOut::new().into()
    } else {
        Easing::new(PhoneExponentialOut)
    });
    let old_origin = target.render_transform_origin();
    let transform: Rc<dyn ITransform> = match entrance {
        Entrance::Slide(distance) => {
            let transform = TranslateTransform::new();
            for (cue, value) in [(0.0, distance), (1.0, 0.0)] {
                animation.children().add(KeyFrame::with_cue(
                    Cue::new(cue),
                    [Setter::new(TranslateTransform::x_property(), value) as _],
                ));
            }
            transform.into()
        }
        Entrance::Turnstile { backward } => {
            let transform = turnstile_transform(target.bounds().width);
            target.set_render_transform_origin(RelativePoint::new(
                0.5,
                0.5,
                RelativeUnit::Relative,
            ));
            for (cue, value) in [(0.0, if backward { 50.0 } else { -80.0 }), (1.0, 0.0)] {
                animation.children().add(KeyFrame::with_cue(
                    Cue::new(cue),
                    [Setter::new(Rotate3DTransform::angle_y_property(), value) as _],
                ));
            }
            transform.into()
        }
        Entrance::Reveal => {
            let transform = TranslateTransform::new();
            for (cue, value, opacity) in [(0.0, 22.0, 0.12), (1.0, 0.0, 1.0)] {
                animation.children().add(KeyFrame::with_cue(
                    Cue::new(cue),
                    [
                        Setter::new(TranslateTransform::y_property(), value) as _,
                        Setter::new(Visual::opacity_property(), opacity) as _,
                    ],
                ));
            }
            transform.into()
        }
        Entrance::None => return AnimationTask::completed(),
    };
    let rotation = run_transform(
        target,
        transform,
        old_origin,
        &animation,
        cancellation.clone(),
        clock.clone(),
    );
    if matches!(entrance, Entrance::Turnstile { .. }) {
        // The original incoming opacity reaches 1 at 10 ms, independently of
        // the exponential rotation. Applying that easing to opacity would
        // incorrectly move the key time.
        AnimationTask::when_all([
            rotation,
            opacity(
                target,
                0.35,
                &[(0.0, 0.0), (0.01 / 0.35, 1.0), (1.0, 1.0)],
                cancellation,
                clock,
            ),
        ])
    } else {
        rotation
    }
}

pub(super) fn exit(
    target: &Ref<Control>,
    backward: bool,
    width: f64,
    cancellation: CancellationToken,
    clock: Option<Rc<dyn IClock>>,
) -> AnimationTask {
    let old_origin = target.render_transform_origin();
    target.set_render_transform_origin(RelativePoint::CENTER);
    let transform: Rc<dyn ITransform> = turnstile_transform(width).into();
    let rotation = Animation::new();
    rotation.set_duration(SignedDuration::from_seconds(0.25));
    rotation.set_easing(Easing::new(PhoneExponentialIn));
    for (cue, angle) in [(0.0, 0.0), (1.0, if backward { -80.0 } else { 50.0 })] {
        rotation.children().add(KeyFrame::with_cue(
            Cue::new(cue),
            [Setter::new(Rotate3DTransform::angle_y_property(), angle) as _],
        ));
    }
    let rotation = run_transform(
        target,
        transform,
        old_origin,
        &rotation,
        cancellation.clone(),
        clock.clone(),
    );
    // The outgoing page remains opaque for 240 ms, then disappears at 250 ms.
    AnimationTask::when_all([
        rotation,
        opacity(
            target,
            0.25,
            &[(0.0, 1.0), (0.24 / 0.25, 1.0), (1.0, 0.0)],
            cancellation,
            clock,
        ),
    ])
}

fn opacity(
    target: &Ref<Control>,
    duration: f64,
    frames: &[(f64, f64)],
    cancellation: CancellationToken,
    clock: Option<Rc<dyn IClock>>,
) -> AnimationTask {
    let animation = Animation::new();
    animation.set_duration(SignedDuration::from_seconds(duration));
    for &(cue, value) in frames {
        animation.children().add(KeyFrame::with_cue(
            Cue::new(cue),
            [Setter::new(Visual::opacity_property(), value) as _],
        ));
    }
    animation.run_async_with_clock(target, clock, cancellation)
}

fn run_transform(
    target: &Ref<Control>,
    transform: Rc<dyn ITransform>,
    old_origin: RelativePoint,
    animation: &Ref<Animation>,
    cancellation: CancellationToken,
    clock: Option<Rc<dyn IClock>>,
) -> AnimationTask {
    target.set_render_transform(Some(transform.clone()));
    let weak = target.downgrade();
    let task = animation.run_async_with_clock(target, clock, cancellation);
    task.on_completed(move || {
        if let Some(control) = weak.upgrade() {
            if control
                .render_transform()
                .is_some_and(|current| Rc::ptr_eq(&current, &transform))
            {
                control.set_render_transform(None);
                control.set_render_transform_origin(old_origin);
            }
        }
    });
    task
}

pub(super) fn turnstile_transform(width: f64) -> Ref<Rotate3DTransform> {
    let width = viewport_width(width);
    let transform = Rotate3DTransform::new();
    // The hinge and the centered perspective camera are separate.
    transform.set_center_x(-width * 0.5);
    transform.set_depth(width * (1_000.0 / 480.0));
    transform
}
