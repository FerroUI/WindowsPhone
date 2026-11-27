//! The public WP7 Toolkit wrapper's Normal/Hidden visual states. Adapted from
//! Microsoft's MS-PL PerformanceProgressBar.cs and WP7 Themes/Generic.xaml at
//! 4e4fd5d36c769c004f7ccc7daf78a5ffb8ef18df (see the repository NOTICE).
//!
//! Requested activity and actual dot motion deliberately differ during exit.
//! The original stops ActualIsIndeterminate only when Hidden has completed.
use ferroui_base::animation::easings::LinearEasing;
use ferroui_base::animation::{Animation, Cue, KeyFrame, PlaybackBehavior, SignedDuration};
use ferroui_base::reactive::{IDisposable, Observable, Subscription};
use ferroui_base::rendering::composition::{
    CompositionBatchTypes, CompositionVisual, ElementComposition, ICompositionObjectAnimations,
};
use ferroui_base::styling::Setter;
use ferroui_base::{Ref, Visual, WeakRef};
use ferroui_controls::Grid;
use std::{
    cell::{Cell, RefCell},
    rc::{Rc, Weak},
    time::Duration,
};

pub(super) const SHOW_DURATION: Duration = Duration::from_millis(250);
pub(super) const HIDE_DURATION: Duration = Duration::from_millis(750);

#[derive(Clone, Copy, PartialEq)]
struct Request {
    active: bool,
    animate: bool,
}

enum Fade {
    Composition {
        visual: Weak<CompositionVisual>,
        completed: Subscription,
    },
    Native(Rc<dyn IDisposable>),
}

impl Fade {
    fn targets(&self, target: &Rc<CompositionVisual>) -> bool {
        matches!(self, Self::Composition { visual, .. }
            if visual.upgrade().is_some_and(|visual| Rc::ptr_eq(&visual, target)))
    }

    /// A replacement compositor animation must inherit the server's current
    /// value. Disposing its callback is sufficient; StartAnimation replaces the
    /// old animation atomically, without writing a stale UI-thread opacity.
    fn dispose(self, replacing: bool) {
        match self {
            Self::Composition { visual, completed } => {
                completed.dispose();
                if !replacing {
                    if let Some(visual) = visual.upgrade() {
                        visual.stop_animation("Opacity");
                    }
                }
            }
            Self::Native(animation) => animation.dispose(),
        }
    }
}

pub(super) struct ActivityMotion {
    root: WeakRef<Grid>,
    request: Cell<Option<Request>>,
    actual: Cell<bool>,
    generation: Cell<u64>,
    fade: RefCell<Option<Fade>>,
    changed: Rc<dyn Fn()>,
}

impl ActivityMotion {
    pub(super) fn new(root: &Ref<Grid>, changed: Rc<dyn Fn()>) -> Rc<Self> {
        Rc::new(Self {
            root: root.downgrade(),
            request: Cell::new(None),
            actual: Cell::new(false),
            generation: Cell::new(0),
            fade: RefCell::new(None),
            changed,
        })
    }

    pub(super) fn actual(&self) -> bool {
        self.actual.get()
    }

    pub(super) fn update(self: &Rc<Self>, active: bool, animate: bool) {
        let request = Request { active, animate };
        let previous = self.request.replace(Some(request));
        if previous == Some(request) {
            return;
        }
        let Some(root) = self.root.upgrade() else {
            self.cancel();
            self.actual.set(false);
            return;
        };

        // OnApplyTemplate in the original calls UpdateVisualStates(false).
        // Likewise, resume with the current application request after an
        // invisible/detached/disabled/reduced-motion interval, not a stale exit.
        if !animate || previous.is_none_or(|previous| !previous.animate) {
            self.settle(&root, active);
        } else if previous.is_some_and(|previous| previous.active != active) {
            if active {
                self.actual.set(true);
            }
            if self.actual.get() {
                self.start(&root, active);
            } else {
                self.settle(&root, false);
            }
        }
    }

    pub(super) fn detach(&self) {
        self.cancel();
        self.request.set(None);
        self.actual.set(false);
        if let Some(root) = self.root.upgrade() {
            set_opacity(&root, 0.0);
        }
    }

    fn cancel(&self) {
        self.generation.set(self.generation.get().wrapping_add(1));
        let fade = self.fade.borrow_mut().take();
        if let Some(fade) = fade {
            fade.dispose(false);
        }
    }

    fn settle(&self, root: &Grid, active: bool) {
        self.cancel();
        self.actual.set(active);
        set_opacity(root, if active { 1.0 } else { 0.0 });
    }

    fn start(self: &Rc<Self>, root: &Ref<Grid>, active: bool) {
        let generation = self.generation.get().wrapping_add(1);
        self.generation.set(generation);
        let target = if active { 1.0 } else { 0.0 };
        let duration = if active { SHOW_DURATION } else { HIDE_DURATION };
        let composition = ElementComposition::get_element_visual(root);
        let previous = self.fade.borrow_mut().take();
        let replacing = previous.as_ref().is_some_and(|previous| {
            composition
                .as_ref()
                .is_some_and(|visual| previous.targets(visual))
        });
        // Capture the effective native animation value before disposing its
        // lease. Compositor opacity is deliberately never read from Visual.
        let native_opacity = root.opacity();
        if let Some(previous) = previous {
            previous.dispose(replacing);
        }

        let weak = Rc::downgrade(self);
        if let Some(visual) = composition {
            if !replacing {
                set_opacity(root, native_opacity);
            }
            let compositor = visual.compositor();
            let batch = compositor.create_scoped_batch(CompositionBatchTypes::ANIMATION);
            let animation = compositor.create_scalar_key_frame_animation();
            animation.set_duration(duration);
            // Omitting a key at zero uses the compositor's current value,
            // including the exact frame of an interrupted entry or exit.
            animation.insert_key_frame_with_easing(
                1.0,
                target as f32,
                Rc::new(LinearEasing::new()),
            );
            visual.start_animation("Opacity", &*animation);
            batch.end();
            let completed = batch.completed(move |_| {
                if let Some(state) = weak.upgrade() {
                    state.complete(generation, active);
                }
            });
            *self.fade.borrow_mut() = Some(Fade::Composition {
                visual: Rc::downgrade(&visual),
                completed,
            });
        } else {
            root.set_opacity(native_opacity);
            let animation = opacity_animation(native_opacity, target, duration);
            let completed = Rc::new(move || {
                if let Some(state) = weak.upgrade() {
                    state.complete(generation, active);
                }
            });
            let lease =
                animation.apply(root, None, Observable::return_(true), Some(completed), true);
            *self.fade.borrow_mut() = Some(Fade::Native(lease));
        }
    }

    fn complete(&self, generation: u64, active: bool) {
        // A stopped/replaced compositor batch may already have posted its
        // completion. A later request always owns the final state.
        if self.generation.get() != generation
            || self
                .request
                .get()
                .is_none_or(|request| request.active != active)
        {
            return;
        }
        let Some(root) = self.root.upgrade() else {
            return;
        };
        self.settle(&root, active);
        (self.changed)();
    }

    #[cfg(test)]
    pub(super) fn is_fading(&self) -> bool {
        self.fade.borrow().is_some()
    }
}

impl Drop for ActivityMotion {
    fn drop(&mut self) {
        if let Some(fade) = self.fade.get_mut().take() {
            fade.dispose(false);
        }
    }
}

fn set_opacity(root: &Grid, value: f64) {
    root.set_opacity(value);
    // A canceled compositor animation can differ from the base opacity even
    // when the native property's value is unchanged. Assigning the owned
    // composition property explicitly both resets it and removes that animation.
    if let Some(visual) = ElementComposition::get_element_visual(root) {
        visual.set_opacity(value as f32);
    }
}

fn opacity_animation(from: f64, to: f64, duration: Duration) -> Ref<Animation> {
    let animation = Animation::new();
    animation.set_duration(SignedDuration::from_seconds(duration.as_secs_f64()));
    animation.set_easing(LinearEasing::new());
    animation.set_playback_behavior(PlaybackBehavior::Always);
    for (cue, value) in [(0.0, from), (1.0, to)] {
        let frame = KeyFrame::new();
        frame.set_cue(Cue::new(cue));
        frame
            .setters()
            .add(Setter::new(Visual::opacity_property(), value));
        animation.children().add(frame);
    }
    animation
}
