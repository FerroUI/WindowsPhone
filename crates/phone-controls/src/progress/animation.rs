//! One bounded pair of compositor animations per dot; no UI-thread tick loop.
use super::activity::ActivityMotion;
use super::{DOT_COUNT, DOT_SIZE};
use ferroui_base::animation::easings::{Easing, IEasing, SharedEasing};
use ferroui_base::animation::{
    Animation, Cue, IterationCount, KeyFrame, PlaybackBehavior, SignedDuration,
};
use ferroui_base::input::InputElement;
use ferroui_base::layout::{HorizontalAlignment as H, Orientation, VerticalAlignment as V};
use ferroui_base::media::TranslateTransform;
use ferroui_base::reactive::{Disposable, IDisposable, Observable, Subscription};
use ferroui_base::rendering::composition::animations::AnimationIterationBehavior;
use ferroui_base::rendering::composition::{ElementComposition, ICompositionObjectAnimations};
use ferroui_base::styling::Setter;
use ferroui_base::{Ref, Vector3D, Visual, WeakRef};
use ferroui_controls::{Border, Grid, ProgressBar};
use std::{
    cell::{Cell, RefCell},
    rc::{Rc, Weak},
    sync::Arc,
    time::Duration,
};

pub(super) const PERIOD: f64 = 4.4;
pub(super) const STAGGER: f64 = 0.2;
pub(super) const TRAVEL: f64 = 2.5;

ferroui_base::ferro_thread_local! {
    static ACTIVE: RefCell<Vec<Weak<ProgressMotion>>> = const { RefCell::new(Vec::new()) };
}

pub(crate) fn refresh_motion() {
    let states = ACTIVE.with(|active| {
        let mut active = active.borrow_mut();
        active.retain(|entry| entry.strong_count() > 0);
        active.iter().filter_map(Weak::upgrade).collect::<Vec<_>>()
    });
    for state in states {
        state.refresh();
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct Presentation {
    indeterminate: bool,
    animate: bool,
    vertical: bool,
    extent: f64,
    compositor: bool,
}

pub(super) struct ProgressMotion {
    owner: WeakRef<ProgressBar>,
    root: WeakRef<Grid>,
    track: WeakRef<Border>,
    indicator: WeakRef<Border>,
    dots: Vec<WeakRef<Border>>,
    activity: Option<Rc<ActivityMotion>>,
    presentation: Cell<Option<Presentation>>,
    subscriptions: RefCell<Vec<Subscription>>,
    properties: RefCell<Vec<Rc<dyn IDisposable>>>,
    animations: RefCell<Vec<Rc<dyn IDisposable>>>,
}

// The template Tag is a property value. Its controller compares by identity,
// never by live subscriptions, animation handles or current frame state.
impl PartialEq for ProgressMotion {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

impl ProgressMotion {
    pub(super) fn attach(
        owner: &Ref<ProgressBar>,
        root: &Ref<Grid>,
        track: &Ref<Border>,
        indicator: &Ref<Border>,
        dots: &[Ref<Border>],
    ) -> Rc<Self> {
        let state = Rc::new_cyclic(|weak: &Weak<Self>| Self {
            owner: owner.downgrade(),
            root: root.downgrade(),
            track: track.downgrade(),
            indicator: indicator.downgrade(),
            dots: dots.iter().map(|dot| dot.downgrade()).collect(),
            activity: owner
                .classes()
                .contains("phone-activity-progress")
                .then(|| {
                    let weak = weak.clone();
                    ActivityMotion::new(
                        root,
                        Rc::new(move || {
                            if let Some(state) = weak.upgrade() {
                                state.refresh();
                            }
                        }),
                    )
                }),
            presentation: Cell::new(None),
            subscriptions: RefCell::new(Vec::new()),
            properties: RefCell::new(Vec::new()),
            animations: RefCell::new(Vec::new()),
        });
        let weak = Rc::downgrade(&state);
        let attached = root.attached_to_visual_tree(move |_| {
            if let Some(state) = weak.upgrade() {
                state.refresh();
            }
        });
        let weak = Rc::downgrade(&state);
        let detached = root.detached_from_visual_tree(move |_| {
            if let Some(state) = weak.upgrade() {
                state.stop();
                if let Some(activity) = &state.activity {
                    activity.detach();
                }
                state.presentation.set(None);
            }
        });
        let weak = Rc::downgrade(&state);
        let visible = root.is_effectively_visible_changed(move || {
            if let Some(state) = weak.upgrade() {
                state.refresh();
            }
        });
        let weak = Rc::downgrade(&state);
        let layout = root.layout_updated(move || {
            if let Some(state) = weak.upgrade() {
                state.refresh();
            }
        });
        state
            .subscriptions
            .borrow_mut()
            .extend([attached, detached, visible, layout]);
        let weak = Rc::downgrade(&state);
        let bounds = root.property_changed(move |change| {
            if change.property() == Visual::bounds_property().as_property() {
                if let Some(state) = weak.upgrade() {
                    state.refresh();
                }
            }
        });
        let weak = Rc::downgrade(&state);
        let owner_properties = owner.property_changed(move |change| {
            if change.property() == ProgressBar::is_indeterminate_property().as_property()
                || change.property() == ProgressBar::orientation_property().as_property()
                || change.property()
                    == InputElement::is_effectively_enabled_property().as_property()
            {
                if let Some(state) = weak.upgrade() {
                    state.refresh();
                }
            }
        });
        state
            .properties
            .borrow_mut()
            .extend([bounds, owner_properties]);
        ACTIVE.with(|active| {
            let mut active = active.borrow_mut();
            active.retain(|entry| entry.strong_count() > 0);
            active.push(Rc::downgrade(&state));
        });
        state.refresh();
        state
    }

    fn refresh(self: &Rc<Self>) {
        let (Some(owner), Some(root)) = (self.owner.upgrade(), self.root.upgrade()) else {
            self.stop();
            return;
        };
        let vertical = owner.orientation() == Orientation::Vertical;
        let extent = if vertical {
            root.bounds().height
        } else {
            root.bounds().width
        };
        let extent = if extent.is_finite() {
            extent.max(0.0)
        } else {
            0.0
        };
        let can_animate = owner.is_effectively_enabled()
            && root.is_attached_to_visual_tree()
            && root.is_effectively_visible()
            && crate::motion_enabled()
            && extent > 0.0;
        let indeterminate = if let Some(activity) = &self.activity {
            activity.update(owner.is_indeterminate(), can_animate);
            activity.actual()
        } else {
            owner.is_indeterminate()
        };
        let next = Presentation {
            indeterminate,
            animate: indeterminate && can_animate,
            vertical,
            extent,
            compositor: self.dots.iter().all(|dot| {
                dot.upgrade()
                    .is_some_and(|dot| ElementComposition::get_element_visual(&dot).is_some())
            }),
        };
        if self.presentation.replace(Some(next)) == Some(next) {
            return;
        }
        self.stop();
        if let Some(track) = self.track.upgrade() {
            track.set_is_visible(!next.indeterminate && self.activity.is_none());
        }
        if let Some(indicator) = self.indicator.upgrade() {
            indicator.set_is_visible(!next.indeterminate && self.activity.is_none());
            indicator.set_horizontal_alignment(if vertical { H::Stretch } else { H::Left });
            indicator.set_vertical_alignment(if vertical { V::Bottom } else { V::Stretch });
        }
        for (index, weak) in self.dots.iter().enumerate() {
            let Some(dot) = weak.upgrade() else { continue };
            dot.set_horizontal_alignment(if vertical { H::Center } else { H::Left });
            dot.set_vertical_alignment(if vertical { V::Bottom } else { V::Center });
            dot.set_is_visible(next.indeterminate);
            let transform = TranslateTransform::new();
            dot.set_render_transform(Some(transform.clone().into()));
            if !next.animate {
                // A static cluster signals unknown work without suggesting a
                // percentage when motion is suppressed or the control disabled.
                let available = (extent - DOT_SIZE).max(0.0);
                let spacing = crate::reference_pixels(20.0).min(available / (DOT_COUNT - 1) as f64);
                let offset = available / 2.0 + (index as f64 - 2.0) * spacing;
                if vertical {
                    transform.set_y(-offset);
                } else {
                    transform.set_x(offset);
                }
                dot.set_opacity(if next.indeterminate { 1.0 } else { 0.0 });
                continue;
            }
            if next.compositor {
                dot.set_opacity(1.0);
                if let Some(visual) = ElementComposition::get_element_visual(&dot) {
                    let property = if vertical {
                        "Translation.Y"
                    } else {
                        "Translation.X"
                    };
                    let mut animations = self.animations.borrow_mut();
                    for (property, opacity, end) in [
                        (property, false, if vertical { -extent } else { extent }),
                        ("Opacity", true, 1.0),
                    ] {
                        let animation = visual.compositor().create_scalar_key_frame_animation();
                        animation.set_duration(Duration::from_millis(4400));
                        animation.set_iteration_behavior(AnimationIterationBehavior::Forever);
                        animation.insert_key_frame(0.0, 0.0);
                        animation.insert_key_frame_with_easing(
                            1.0,
                            end as f32,
                            Rc::new(DotEase { index, opacity }),
                        );
                        visual.start_animation(property, &*animation);
                        let weak = Rc::downgrade(&visual);
                        animations.push(Disposable::create(move || {
                            if let Some(visual) = weak.upgrade() {
                                visual.stop_animation(property);
                                if opacity {
                                    visual.set_opacity(1.0);
                                } else {
                                    visual.set_translation(Vector3D::new(0.0, 0.0, 0.0));
                                }
                            }
                        }));
                    }
                    continue;
                }
            }
            // Non-composition/headless hosts retain the same timeline through
            // the native clock. A later compositor attachment replaces these
            // leases at the next layout without changing the control's value.
            dot.set_opacity(0.0);
            let position = program(
                DotEase {
                    index,
                    opacity: false,
                },
                if vertical { -extent } else { extent },
                vertical,
                false,
            );
            let opacity = program(
                DotEase {
                    index,
                    opacity: true,
                },
                1.0,
                vertical,
                true,
            );
            let mut animations = self.animations.borrow_mut();
            for animation in [position, opacity] {
                animations.push(animation.apply(&dot, None, Observable::return_(true), None, true));
            }
        }
    }

    fn stop(&self) {
        let animations = self.animations.borrow_mut().drain(..).collect::<Vec<_>>();
        for animation in animations {
            animation.dispose();
        }
    }

    #[cfg(test)]
    pub(super) fn animation_count(&self) -> usize {
        self.animations.borrow().len()
    }

    #[cfg(test)]
    pub(super) fn activity(&self) -> Option<&Rc<ActivityMotion>> {
        self.activity.as_ref()
    }
}

impl Drop for ProgressMotion {
    fn drop(&mut self) {
        self.stop();
        for property in self.properties.get_mut().drain(..) {
            property.dispose();
        }
        for subscription in self.subscriptions.get_mut().drain(..) {
            subscription.dispose();
        }
    }
}

fn program(ease: DotEase, end: f64, vertical: bool, opacity: bool) -> Ref<Animation> {
    let animation = Animation::new();
    animation.set_duration(SignedDuration::from_seconds(PERIOD));
    animation.set_iteration_count(IterationCount::INFINITE);
    animation.set_playback_behavior(PlaybackBehavior::OnlyIfVisible);
    animation.set_easing(Easing::new(ease));
    for (cue, value) in [(0.0, 0.0), (1.0, end)] {
        let frame = KeyFrame::new();
        frame.set_cue(Cue::new(cue));
        if opacity {
            frame
                .setters()
                .add(Setter::new(Visual::opacity_property(), value));
        } else if vertical {
            frame
                .setters()
                .add(Setter::new(TranslateTransform::y_property(), value));
        } else {
            frame
                .setters()
                .add(Setter::new(TranslateTransform::x_property(), value));
        }
        animation.children().add(frame);
    }
    animation
}

/// The original relative animation marks 0%, 33%, 66%, 100% at 0, .5, 2,
/// 2.5 seconds; a 200 ms stagger applies to each square. Each animation shares
/// the whole 4.4 s cycle so later dots never drift relative to earlier ones.
#[derive(Clone, Copy)]
pub(super) struct DotEase {
    pub(super) index: usize,
    pub(super) opacity: bool,
}

impl IEasing for DotEase {
    fn ease(&self, progress: f64) -> f64 {
        let seconds = progress.clamp(0.0, 1.0) * PERIOD - self.index as f64 * STAGGER;
        if self.opacity {
            return if (0.0..TRAVEL).contains(&seconds) {
                1.0
            } else {
                0.0
            };
        }
        if seconds <= 0.0 {
            return 0.0;
        }
        if seconds < 0.5 {
            return 0.33 * (1.0 - ((1.0 - seconds / 0.5).exp() - 1.0) / (1.0_f64.exp() - 1.0));
        }
        if seconds <= 2.0 {
            return 0.33 + 0.33 * (seconds - 0.5) / 1.5;
        }
        if seconds < TRAVEL {
            return 0.66 + 0.34 * (((seconds - 2.0) / 0.5).exp() - 1.0) / (1.0_f64.exp() - 1.0);
        }
        1.0
    }

    fn to_shared(&self) -> Arc<SharedEasing> {
        Arc::new(*self)
    }
}
