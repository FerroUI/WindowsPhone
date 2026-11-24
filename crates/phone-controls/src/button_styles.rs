//! Command states are independent from layout and native activation. Original
//! app-bar glyph/label geometry is documented; unrecovered OS state ARGB values
//! are mapped to the phone palette with explicit foreground/background contrast.
use crate::{CONTROL_BORDER, FONT_FAMILY, Palette, templates::BUTTON_SELECTORS};
use ferroui_base::Visual;
use ferroui_base::layout::{HorizontalAlignment as H, Layoutable, VerticalAlignment as V};
use ferroui_base::media::{FontFamily, FontWeight};
use ferroui_base::styling::Styles;
use ferroui_controls::primitives::TemplatedControl;
use ferroui_controls::{Border, ContentControl, TextBlock};
use ferroui_view::style;

pub(crate) fn install(styles: &Styles, p: &Palette) {
    for selector in BUTTON_SELECTORS {
        styles.add(
            style(selector)
                .set(
                    TemplatedControl::font_family_property(),
                    FontFamily::new(FONT_FAMILY),
                )
                .set(TemplatedControl::font_size_property(), 22.0)
                .set(TemplatedControl::font_weight_property(), FontWeight::Normal)
                .set(TemplatedControl::foreground_property(), p.foreground)
                .set(TemplatedControl::background_property(), "Transparent")
                .set(TemplatedControl::corner_radius_property(), 0.0)
                .set(TemplatedControl::border_thickness_property(), 0.0)
                .set(TemplatedControl::padding_property(), 0.0)
                .set(Layoutable::min_width_property(), 0.0)
                .set(Layoutable::min_height_property(), 0.0)
                .set(
                    ContentControl::horizontal_content_alignment_property(),
                    H::Stretch,
                )
                .set(
                    ContentControl::vertical_content_alignment_property(),
                    V::Stretch,
                )
                .build(),
        );
        // Preserve custom app content colors on rows/tiles; those controls own
        // position-sensitive tilt rather than scaling their input rectangle.
        if matches!(
            selector,
            "Button.phone-flat" | "Button.phone-tile" | "Button.phone-row" | "Button.phone-pivot"
        ) {
            styles.add(
                style(&format!("{selector}:disabled"))
                    .set(Visual::opacity_property(), 0.35)
                    .build(),
            );
        }
        styles.add(
            style(&format!("{selector} /template/ Border#PhoneControlFocus"))
                .set(Border::border_brush_property(), p.foreground)
                .set(Visual::opacity_property(), 0.0)
                .build(),
        );
        styles.add(
            style(&format!(
                "{selector}:focus-visible /template/ Border#PhoneControlFocus"
            ))
            .set(Visual::opacity_property(), 1.0)
            .build(),
        );
    }
    styles.add(
        style("Button.phone-tile:pressed")
            .set(Visual::opacity_property(), 0.94)
            .build(),
    );
    styles.add(
        style("Button.phone-tile.folder-drop")
            .set(
                Visual::render_transform_property(),
                crate::motion::scale(if p.motion_enabled { 0.88 } else { 1.0 }),
            )
            .set(Visual::opacity_property(), 0.72)
            .build(),
    );

    styles.add(
        style("Button.phone-button")
            .set(TemplatedControl::border_brush_property(), p.foreground)
            .set(
                TemplatedControl::border_thickness_property(),
                CONTROL_BORDER,
            )
            .set(TemplatedControl::padding_property(), (16.0, 7.0))
            .set(Layoutable::min_height_property(), 44.0)
            .set(
                ContentControl::horizontal_content_alignment_property(),
                H::Center,
            )
            .set(
                ContentControl::vertical_content_alignment_property(),
                V::Center,
            )
            .build(),
    );
    for selector in ["Button.phone-button", "Button.phone-more"] {
        styles.add(
            style(&format!("{selector}:pressed"))
                .set(TemplatedControl::background_property(), p.foreground)
                .set(TemplatedControl::foreground_property(), p.background)
                .build(),
        );
        styles.add(
            style(&format!("{selector}:disabled"))
                .set(TemplatedControl::background_property(), "Transparent")
                .set(TemplatedControl::foreground_property(), p.secondary)
                .set(TemplatedControl::border_brush_property(), p.secondary)
                .build(),
        );
    }
    // The global TextBlock palette style has higher priority than inherited
    // Foreground. Style command captions directly so pressed ink stays legible.
    for (state, color) in [
        ("", p.foreground),
        (":pressed", p.background),
        (":disabled", p.secondary),
    ] {
        styles.add(
            style(&format!("Button.phone-button{state} TextBlock"))
                .set(TextBlock::foreground_property(), color)
                .build(),
        );
    }
    for (state, color) in [
        ("", p.foreground),
        (":pressed", p.background),
        (":disabled", p.secondary),
    ] {
        styles.add(
            style(&format!(
                "Button.phone-more{state} PathIcon.phone-action-glyph"
            ))
            .set(TemplatedControl::foreground_property(), color)
            .build(),
        );
    }
    for (state, fill, ink, border) in [
        ("", "Transparent", p.foreground, p.foreground),
        (":pressed", p.foreground, p.background, p.foreground),
        (":disabled", "Transparent", p.secondary, p.secondary),
    ] {
        styles.add(
            style(&format!(
                "Button.phone-action{state} Border.phone-action-circle"
            ))
            .set(Border::background_property(), fill)
            .set(Border::border_brush_property(), border)
            .build(),
        );
        styles.add(
            style(&format!(
                "Button.phone-action{state} PathIcon.phone-action-glyph"
            ))
            .set(TemplatedControl::foreground_property(), ink)
            .build(),
        );
    }
    styles.add(
        style("Button.phone-action:disabled TextBlock.phone-action-label")
            .set(TextBlock::foreground_property(), p.secondary)
            .build(),
    );
}

#[cfg(test)]
mod tests;
