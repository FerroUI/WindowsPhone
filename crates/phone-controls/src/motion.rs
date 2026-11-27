//! Bounded native motion. Every explicit animation cancels on unload and
//! releases its handler at completion; there is no application animation loop.
use ferroui_base::animation::easings::CubicEaseOut;
use ferroui_base::animation::{Animation, Cue, KeyFrame, SignedDuration};
use ferroui_base::media::transformation::TransformOperations;
use ferroui_base::media::{ITransform, TranslateTransform};
use ferroui_base::styling::Setter;
use ferroui_base::threading::CancellationToken;
use ferroui_base::{Ref, Visual};
use ferroui_controls::Control;
use std::{cell::Cell, rc::Rc};

ferroui_base::ferro_thread_local! {
    static ENABLED: Cell<bool> = const { Cell::new(true) };
}

pub fn set_motion_enabled(enabled: bool) {
    let changed = ENABLED.with(|value| value.replace(enabled) != enabled);
    if changed {
        crate::progress::refresh_motion();
    }
}

pub fn motion_enabled() -> bool {
    ENABLED.with(Cell::get)
}

pub(crate) fn scale(value: f64) -> Option<Rc<dyn ITransform>> {
    let mut transform = TransformOperations::create_builder(1);
    transform.append_scale(value, value);
    Some(transform.build())
}

/// A page's finite entrance. Positive direction enters from the right;
/// negative direction enters from the left; zero fades without translation.
pub fn page_enter(target: &Ref<Control>, direction: f64) {
    enter(target, direction.clamp(-1.0, 1.0) * 44.0, 0.0, 0.23, 0.12);
}

/// Folder/app-bar reveal without animating layout on each frame.
pub fn reveal(target: &Ref<Control>) {
    enter(target, 0.0, 22.0, 0.22, 0.12);
}

/// A live tile content update. Only its clipped inner panel moves; the tile's
/// hit rectangle and Start packing remain unchanged throughout the animation.
pub fn live_tile_enter(target: &Ref<Control>, distance: f64) {
    enter(target, 0.0, distance.clamp(0.0, 120.0), 0.42, 0.12);
}

/// Restore a canceled direct-manipulation gesture without fading the page.
pub fn settle(target: &Ref<Control>, x: f64, y: f64) {
    enter(target, x, y, 0.18, 1.0);
}

fn enter(target: &Ref<Control>, x: f64, y: f64, seconds: f64, start_opacity: f64) {
    if !motion_enabled() || !target.is_attached_to_visual_tree() {
        return;
    }
    // The dedicated page/live-content wrapper owns this transform. Do not
    // replace a gesture's or a caller's transform if it already has one.
    if target.render_transform().is_some() {
        return;
    }
    let transform: Rc<dyn ITransform> = TranslateTransform::new().into();
    target.set_render_transform(Some(transform.clone()));
    let animation = Animation::new();
    animation.set_duration(SignedDuration::from_seconds(seconds));
    animation.set_easing(CubicEaseOut::new());
    for (cue, dx, dy, opacity) in [(0.0, x, y, start_opacity), (1.0, 0.0, 0.0, 1.0)] {
        let frame = KeyFrame::new();
        frame.set_cue(Cue::new(cue));
        frame
            .setters()
            .add(Setter::new(TranslateTransform::x_property(), dx));
        frame
            .setters()
            .add(Setter::new(TranslateTransform::y_property(), dy));
        frame
            .setters()
            .add(Setter::new(Visual::opacity_property(), opacity));
        animation.children().add(frame);
    }
    let cancellation = CancellationToken::new();
    let cancel = cancellation.clone();
    let unloaded = target.add_handler(Control::unloaded_event(), move |_, _| cancel.cancel());
    let weak = target.downgrade();
    let task = animation.run_async(target, cancellation);
    task.on_completed(move || {
        if let Some(control) = weak.upgrade() {
            control.remove_handler(Control::unloaded_event(), unloaded);
            if control
                .render_transform()
                .is_some_and(|current| Rc::ptr_eq(&current, &transform))
            {
                control.set_render_transform(None);
            }
        }
    });
}
