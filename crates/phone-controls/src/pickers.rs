//! Phone pages over the native date/time picker value and selection contracts.
mod focus;
mod input;
mod presentation;
mod templates;

pub use focus::attach_picker_focus;
pub use presentation::{close_phone_pickers, dismiss_phone_picker, has_open_phone_picker};

use crate::{Palette, FONT_FAMILY};
use ferroui_base::animation::{
    Animatable, DoubleTransition, ITransition, SignedDuration, Transitions,
};
use ferroui_base::input::InputElement;
use ferroui_base::layout::{HorizontalAlignment, Layoutable};
use ferroui_base::media::FontFamily;
use ferroui_base::styling::Styles;
use ferroui_base::Visual;
use ferroui_controls::{
    presenters::ContentPresenter, primitives::TemplatedControl, Border, TextBlock,
};
use ferroui_view::style;
use std::rc::Rc;

pub(crate) fn install(styles: &Styles, p: &Palette) {
    for picker in ["DatePicker.phone-picker", "TimePicker.phone-picker"] {
        styles.add(
            style(picker)
                .set(
                    TemplatedControl::font_family_property(),
                    FontFamily::new(FONT_FAMILY),
                )
                .set(TemplatedControl::font_size_property(), 22.0 * p.text_scale)
                .set(TemplatedControl::foreground_property(), p.foreground)
                .set(TemplatedControl::background_property(), p.background)
                .set(TemplatedControl::border_brush_property(), p.foreground)
                .set(TemplatedControl::border_thickness_property(), 2.0)
                .set(TemplatedControl::corner_radius_property(), 0.0)
                .set(Layoutable::min_width_property(), 0.0)
                .set(Layoutable::max_width_property(), f64::INFINITY)
                .set(Layoutable::min_height_property(), 46.0)
                .set(
                    Layoutable::horizontal_alignment_property(),
                    HorizontalAlignment::Stretch,
                )
                .build(),
        );
        styles.add(
            style(&format!("{picker} /template/ Button#PART_FlyoutButton"))
                .set(Layoutable::min_height_property(), 46.0)
                .set(TemplatedControl::corner_radius_property(), 0.0)
                .build(),
        );
        for (state, outline, background) in [
            ("", p.foreground, p.background),
            (":pointerover", p.foreground, p.surface),
            (":pressed", p.accent.as_str(), p.surface),
            (":focus-visible", p.accent.as_str(), p.background),
            (":disabled", p.secondary, p.surface),
        ] {
            styles.add(style(&format!("{picker} /template/ Button#PART_FlyoutButton{state} /template/ ContentPresenter#PART_ContentPresenter"))
                .set(ContentPresenter::background_property(), background)
                .set(ContentPresenter::foreground_property(), p.foreground)
                .set(ContentPresenter::border_brush_property(), outline)
                .set(ContentPresenter::border_thickness_property(), 2.0)
                .set(ContentPresenter::corner_radius_property(), 0.0)
                .build());
        }
    }
    for presenter in [
        "DatePickerPresenter.phone-picker-presenter",
        "TimePickerPresenter.phone-picker-presenter",
    ] {
        styles.add(
            style(presenter)
                .set(
                    TemplatedControl::font_family_property(),
                    FontFamily::new(FONT_FAMILY),
                )
                .set(TemplatedControl::font_size_property(), 22.0 * p.text_scale)
                .set(TemplatedControl::foreground_property(), p.foreground)
                .set(TemplatedControl::background_property(), p.surface)
                .set(TemplatedControl::border_thickness_property(), 0.0)
                .set(TemplatedControl::corner_radius_property(), 0.0)
                .set(TemplatedControl::padding_property(), 0.0)
                .set(Layoutable::min_width_property(), 0.0)
                .build(),
        );
        styles.add(
            style(&format!(
                "{presenter} /template/ Border#PhonePickerApplicationBar"
            ))
            .set(Border::background_property(), p.surface)
            .set(Border::corner_radius_property(), 0.0)
            .build(),
        );
        styles.add(
            style(&format!("{presenter} /template/ Button"))
                .set(TemplatedControl::foreground_property(), p.foreground)
                .set(TemplatedControl::corner_radius_property(), 0.0)
                .set(Layoutable::min_height_property(), 44.0)
                .build(),
        );
    }
    let items = "DateTimePickerPanel.phone-picker-wheel > ListBoxItem";
    let transitions = p.motion_enabled.then(|| {
        let opacity: Rc<dyn ITransition> = DoubleTransition::with_property(
            Visual::opacity_property().as_property(),
            SignedDuration::from_seconds(0.33),
        )
        .into();
        Transitions::from_items([opacity])
    });
    styles.add(
        style(items)
            .set(
                TemplatedControl::font_family_property(),
                FontFamily::new(FONT_FAMILY),
            )
            .set(TemplatedControl::font_size_property(), 24.0 * p.text_scale)
            .set(TemplatedControl::corner_radius_property(), 0.0)
            .set(Animatable::transitions_property(), transitions)
            .set(Visual::opacity_property(), 0.0)
            .set(InputElement::is_hit_test_visible_property(), false)
            .build(),
    );
    for (selector, opacity) in [
        (
            "DateTimePickerPanel.phone-picker-wheel.expanded > ListBoxItem",
            0.8,
        ),
        (
            "DateTimePickerPanel.phone-picker-wheel > ListBoxItem:selected",
            1.0,
        ),
    ] {
        styles.add(
            style(selector)
                .set(Visual::opacity_property(), opacity)
                .set(InputElement::is_hit_test_visible_property(), true)
                .build(),
        );
    }
    styles.add(
        style(&format!("{items}:selected"))
            .set(Animatable::transitions_property(), None::<Transitions>)
            .build(),
    );
    styles.add(
        style(&format!("{items} /template/ Border#PhonePickerCell"))
            .set(Border::background_property(), "Transparent")
            .set(
                Border::border_brush_property(),
                if p.high_contrast {
                    p.foreground
                } else {
                    p.subtle
                },
            )
            .build(),
    );
    styles.add(
        style(&format!(
            "{items}:selected /template/ Border#PhonePickerCell"
        ))
        .set(Border::background_property(), p.accent.as_str())
        .set(Border::border_brush_property(), "Transparent")
        .build(),
    );
    styles.add(
        style(&format!("{items} /template/ TextBlock"))
            .set(TextBlock::foreground_property(), p.foreground)
            .build(),
    );
    styles.add(
        style(&format!("{items}:selected /template/ TextBlock"))
            .set(TextBlock::foreground_property(), "White")
            .build(),
    );
}

#[cfg(test)]
mod tests;
