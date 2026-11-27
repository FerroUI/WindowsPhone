//! Slim native progress and the Windows Phone Toolkit's five-dot activity strip.
//!
//! Indeterminate geometry/timing is adapted from the WP7 Toolkit
//! `PerformanceProgressBar` template at revision
//! `4e4fd5d36c769c004f7ccc7daf78a5ffb8ef18df` (Microsoft, MS-PL).
//! WP8.1 system-control pixel parity has not been established. The native
//! ProgressBar still owns range coercion, value and automation semantics.
use crate::Palette;
use ferroui_base::data::TemplateBinding;
use ferroui_base::layout::{HorizontalAlignment as H, Layoutable, VerticalAlignment as V};
use ferroui_base::styling::Styles;
use ferroui_base::{Ref, Visual};
use ferroui_controls::primitives::TemplatedControl;
use ferroui_controls::templates::{
    FuncControlTemplate, FuncTemplateNameScopeExtensions, IControlTemplate,
};
use ferroui_controls::{Border, Grid, ProgressBar};
use ferroui_view::{prelude::*, style};
use std::{cell::OnceCell, rc::Rc};

mod activity;
mod animation;
pub(crate) use animation::refresh_motion;
use animation::ProgressMotion;

pub(crate) const DOT_SIZE: f64 = crate::reference_pixels(4.0);
pub(crate) const DOT_COUNT: usize = 5;

ferroui_base::ferro_thread_local! {
    static TEMPLATE: OnceCell<Rc<dyn IControlTemplate>> = const { OnceCell::new() };
}

/// A native phone progress bar. Set `value`, `minimum` and `maximum` for known
/// work, or `is_indeterminate(true)` for an operation without a known total.
/// Keep a descriptive label; indeterminate activity never invents a percentage.
/// The builder allows ordinary reactive properties and a stable control name.
pub fn progress(label: &str) -> El<ProgressBar> {
    progress_bar()
        .class("phone-progress")
        .automation_properties_name(label)
        .is_hit_test_visible(false)
        .focusable(false)
}

/// The Toolkit's activity-only progress wrapper. Bind `is_indeterminate` to
/// whether work is requested and keep this control mounted while work finishes.
/// A running strip fades in over 250 ms and out over 750 ms; its dots stop only
/// after fade-out. Reversing a request reverses from the current opacity.
///
/// Like the original opacity-only wrapper, the idle control reserves its layout
/// slot. Initial attachment reflects the current request without a transition.
/// Explicit visibility, enablement and motion preferences remain application
/// controlled; hiding/detaching cancels motion, and reduced motion settles at
/// static dots or an empty slot. Use [`progress`] for determinate work.
pub fn activity_progress(label: &str) -> El<ProgressBar> {
    progress(label).class("phone-activity-progress")
}

pub(crate) fn install_template(styles: &Styles) {
    styles.add(
        style("ProgressBar.phone-progress")
            .set(TemplatedControl::template_property(), Some(template()))
            .build(),
    );
}

pub(crate) fn install(styles: &Styles, palette: &Palette) {
    styles.add(
        style("ProgressBar.phone-progress")
            .set(
                TemplatedControl::foreground_property(),
                palette.accent.as_str(),
            )
            .set(TemplatedControl::background_property(), palette.subtle)
            .set(TemplatedControl::border_thickness_property(), 0.0)
            .set(TemplatedControl::padding_property(), 0.0)
            .set(TemplatedControl::corner_radius_property(), 0.0)
            .set(Layoutable::min_width_property(), 0.0)
            .set(Layoutable::min_height_property(), 0.0)
            .build(),
    );
    styles.add(
        style("ProgressBar.phone-progress:horizontal")
            .set(Layoutable::height_property(), DOT_SIZE)
            .build(),
    );
    styles.add(
        style("ProgressBar.phone-progress:vertical")
            .set(Layoutable::width_property(), DOT_SIZE)
            .build(),
    );
    styles.add(
        style("ProgressBar.phone-progress:disabled")
            .set(Visual::opacity_property(), 0.4)
            .build(),
    );
}

fn ink(part: &Ref<Border>) {
    part.bind_binding(
        Border::background_property().as_property(),
        &*TemplateBinding::new(TemplatedControl::foreground_property().as_property()),
    );
}

fn template() -> Rc<dyn IControlTemplate> {
    TEMPLATE.with(|slot| {
        slot.get_or_init(|| {
            FuncControlTemplate::for_type::<ProgressBar>(|owner, scope| {
                let root = Grid::new();
                root.set_name(Some("PhoneProgressRoot".into()));
                root.set_clip_to_bounds(true);
                root.set_is_hit_test_visible(false);

                let track = Border::new();
                track.set_name(Some("PhoneProgressTrack".into()));
                track.bind_binding(
                    Border::background_property().as_property(),
                    &*TemplateBinding::new(TemplatedControl::background_property().as_property()),
                );
                root.children()
                    .add(track.clone().register_in_name_scope(&**scope));

                // ProgressBar measures this exact part against its parent,
                // preserving the native range and orientation implementation.
                let indicator = Border::new();
                indicator.set_name(Some("PART_Indicator".into()));
                indicator.set_horizontal_alignment(H::Left);
                indicator.set_vertical_alignment(V::Stretch);
                ink(&indicator);
                root.children()
                    .add(indicator.clone().register_in_name_scope(&**scope));

                let dots = (0..DOT_COUNT)
                    .map(|index| {
                        let dot = Border::new();
                        dot.set_name(Some(format!("PhoneProgressDot{index}")));
                        dot.set_width(DOT_SIZE);
                        dot.set_height(DOT_SIZE);
                        dot.set_horizontal_alignment(H::Left);
                        dot.set_vertical_alignment(V::Center);
                        dot.set_opacity(0.0);
                        ink(&dot);
                        root.children()
                            .add(dot.clone().register_in_name_scope(&**scope));
                        dot
                    })
                    .collect::<Vec<_>>();
                let state = ProgressMotion::attach(owner, &root, &track, &indicator, &dots);
                // The template owns the controller; subscriptions contain weak
                // links, so replacing the template drops every animation lease.
                root.set_tag(Some(state));
                root.register_in_name_scope(&**scope).upcast()
            }) as Rc<dyn IControlTemplate>
        })
        .clone()
    })
}

#[cfg(test)]
mod tests;
