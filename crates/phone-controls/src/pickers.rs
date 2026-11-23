//! Phone styling for native date/time wheels. The WinUI templates still own
//! their selectors, repeat buttons, validation, popup and accept/cancel logic.
mod focus;

pub use focus::attach_picker_focus;

use crate::{FONT_FAMILY, Palette};
use ferroui_base::layout::{HorizontalAlignment, Layoutable};
use ferroui_base::media::FontFamily;
use ferroui_base::styling::Styles;
use ferroui_controls::{Border, presenters::ContentPresenter, primitives::TemplatedControl};
use ferroui_view::style;

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
    for presenter in ["DatePickerPresenter", "TimePickerPresenter"] {
        styles.add(
            style(presenter)
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
                .set(Layoutable::width_property(), 256.0)
                .set(Layoutable::min_width_property(), 256.0)
                .build(),
        );
        styles.add(
            style(&format!("{presenter} /template/ Border#HighlightRect"))
                .set(Border::background_property(), p.accent.as_str())
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
    styles.add(
        style("DateTimePickerPanel > ListBoxItem")
            .set(
                TemplatedControl::font_family_property(),
                FontFamily::new(FONT_FAMILY),
            )
            .set(TemplatedControl::font_size_property(), 24.0 * p.text_scale)
            .set(TemplatedControl::corner_radius_property(), 0.0)
            .build(),
    );
    for (state, foreground) in [("", p.foreground), (":selected", "White")] {
        styles.add(style(&format!("DateTimePickerPanel > ListBoxItem{state} /template/ ContentPresenter#PART_ContentPresenter"))
            .set(ContentPresenter::foreground_property(), foreground)
            .build());
    }
}
