//! Independent rotation and opacity tracks preserve the Toolkit key times.
use super::{Entrance, viewport_width};
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
    if let Entrance::Slide(distance) = entrance {
        return slide(target, distance, 0.0, cancellation, clock);
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
        Entrance::None | Entrance::Slide(_) => return AnimationTask::completed(),
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

/// Both sides of a shell page settle share the same clock and easing. Their
/// edges remain one viewport apart, including when release happens partway
/// through a pan. This is direct-manipulation continuation, not a second
/// turnstile or a pair of independently timed page entrances.
pub(super) fn slide_pair(
    outgoing: &Ref<Control>,
    incoming: &Ref<Control>,
    remaining: f64,
    width: f64,
    cancellation: CancellationToken,
    clock: Option<Rc<dyn IClock>>,
) -> AnimationTask {
    let travel = remaining.signum() * viewport_width(width);
    AnimationTask::when_all([
        slide(
            outgoing,
            remaining - travel,
            -travel,
            cancellation.clone(),
            clock.clone(),
        ),
        slide(incoming, remaining, 0.0, cancellation, clock),
    ])
}

fn slide(
    target: &Ref<Control>,
    from: f64,
    to: f64,
    cancellation: CancellationToken,
    clock: Option<Rc<dyn IClock>>,
) -> AnimationTask {
    translate(
        target,
        from,
        to,
        0.35,
        Easing::new(PhoneExponentialOut),
        cancellation,
        clock,
    )
}

/// A rejected shell pan returns both edges on the existing 180 ms cubic
/// settle policy. No route changes and no destination command can run.
pub(super) fn settle_pair(
    current: &Ref<Control>,
    neighbor: &Ref<Control>,
    delta: f64,
    edge: f64,
    cancellation: CancellationToken,
) -> AnimationTask {
    AnimationTask::when_all([
        translate(
            current,
            delta,
            0.0,
            0.18,
            CubicEaseOut::new().into(),
            cancellation.clone(),
            None,
        ),
        translate(
            neighbor,
            edge + delta,
            edge,
            0.18,
            CubicEaseOut::new().into(),
            cancellation,
            None,
        ),
    ])
}

fn translate(
    target: &Ref<Control>,
    from: f64,
    to: f64,
    seconds: f64,
    easing: Easing,
    cancellation: CancellationToken,
    clock: Option<Rc<dyn IClock>>,
) -> AnimationTask {
    if !from.is_finite()
        || !to.is_finite()
        || (from - to).abs() <= f64::EPSILON
        || target.render_transform().is_some()
    {
        return AnimationTask::completed();
    }
    let transform = TranslateTransform::new();
    // Establish the carried pointer position before the first clock tick so
    // the old page cannot flash back to its unmanipulated position.
    transform.set_x(from);
    let animation = Animation::new();
    animation.set_duration(SignedDuration::from_seconds(seconds));
    animation.set_easing(easing);
    for (cue, value) in [(0.0, from), (1.0, to)] {
        animation.children().add(KeyFrame::with_cue(
            Cue::new(cue),
            [Setter::new(TranslateTransform::x_property(), value) as _],
        ));
    }
    run_transform(
        target,
        transform.into(),
        target.render_transform_origin(),
        &animation,
        cancellation,
        clock,
    )
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
