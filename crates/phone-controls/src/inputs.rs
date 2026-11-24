//! PhoneTextBox resource roles over the existing native TextBox template.
//!
//! Source: Microsoft's Windows Phone Toolkit WP8 `Themes/Generic.xaml`,
//! PhoneTextBox style, revision 4e4fd5d36c769c004f7ccc7daf78a5ffb8ef18df.
//! That source specifies the normal/editing/disabled roles, 25.333 font,
//! 3-pixel border/padding and 1×2 inner margin. Exact SDK brush ARGB values
//! were not recovered; the neutral surfaces and palette mapping below are
//! application adaptations, not asserted original Windows Phone colors.
//!
//! Native text, composition, selection, undo, clipboard, password handling,
//! validation and ScrollViewer parts stay mounted. The source swaps in a
//! separate read-only TextBox when disabled; here transparent chrome and
//! disabled ink on the existing presenter preserve the same editing state.
use crate::{
    CONTROL_BORDER, FONT_FAMILY, FONT_MEDIUM_LARGE, MIN_TOUCH_TARGET, Palette,
    reference_pixels as px,
};
use ferroui_base::{
    Visual,
    layout::Layoutable,
    media::{FontFamily, FontWeight},
    styling::Styles,
};
use ferroui_controls::{Border, TextBox, primitives::TemplatedControl};
use ferroui_view::style;

pub(crate) fn install(styles: &Styles, palette: &Palette) {
    // PhoneTextBoxBrush -> a neutral resting surface and the same border.
    // PhoneTextBoxEditBackgroundBrush -> white editing surface.
    // PhoneTextBoxEditBorderBrush -> application accent.
    // PhoneTextBoxForegroundBrush -> black on those light surfaces.
    // PhoneDisabledBrush -> the palette's secondary ink on transparent chrome.
    // High contrast deliberately follows the complete palette instead.
    let surface = if palette.high_contrast {
        palette.background
    } else {
        "#D6D6D6"
    };
    let editing = if palette.high_contrast {
        palette.background
    } else {
        "#FFFFFF"
    };
    let ink = if palette.high_contrast {
        palette.foreground
    } else {
        "#000000"
    };
    let hint = if palette.high_contrast {
        palette.foreground
    } else {
        "#666666"
    };
    let edge = if palette.high_contrast {
        palette.foreground
    } else {
        surface
    };
    let accent = if palette.high_contrast {
        palette.foreground
    } else {
        palette.accent.as_str()
    };
    let selection_ink = if palette.high_contrast {
        palette.background
    } else {
        "#FFFFFF"
    };
    styles.add(
        style("TextBox.phone-input")
            .set(
                TemplatedControl::font_family_property(),
                FontFamily::new(FONT_FAMILY),
            )
            .set(
                TemplatedControl::font_size_property(),
                FONT_MEDIUM_LARGE * palette.text_scale,
            )
            .set(TemplatedControl::font_weight_property(), FontWeight::Normal)
            .set(TemplatedControl::foreground_property(), ink)
            .set(TemplatedControl::background_property(), surface)
            .set(TemplatedControl::border_brush_property(), edge)
            .set(
                TemplatedControl::border_thickness_property(),
                CONTROL_BORDER,
            )
            .set(TemplatedControl::corner_radius_property(), 0.0)
            // The native ScrollViewer folds the source's padding3 and inner
            // margin1×2 into one inset. Field placement remains caller-owned:
            // adding the original outer12 touch overhang would duplicate the
            // margins already supplied by app forms and search layouts.
            .set(TemplatedControl::padding_property(), (px(4.0), px(5.0)))
            .set(Layoutable::min_width_property(), 0.0)
            .set(Layoutable::min_height_property(), MIN_TOUCH_TARGET)
            .set(TextBox::caret_brush_property(), ink)
            .set(TextBox::selection_brush_property(), accent)
            .set(
                TextBox::selection_foreground_brush_property(),
                selection_ink,
            )
            .set(TextBox::placeholder_foreground_property(), hint)
            .build(),
    );

    // Toolkit changes focus colors immediately and has no mouse-over color
    // shift. Override native WinUI's state styles on the existing border and
    // ScrollViewer, so their child TextPresenter inherits the correct ink.
    // A general read-only field is an adaptation: selectable normal text,
    // transparent fill, subdued outline, and a keyboard-visible focus edge.
    // The disabled rule is last and replaces ink rather than fading content.
    for (state, background, outline, foreground, placeholder) in [
        ("", surface, edge, ink, hint),
        (":pointerover", surface, edge, ink, hint),
        (":focus", editing, accent, ink, hint),
        (
            "[IsReadOnly=True]",
            "Transparent",
            palette.secondary,
            palette.foreground,
            palette.secondary,
        ),
        (
            "[IsReadOnly=True]:pointerover",
            "Transparent",
            palette.secondary,
            palette.foreground,
            palette.secondary,
        ),
        (
            "[IsReadOnly=True]:focus",
            "Transparent",
            palette.secondary,
            palette.foreground,
            palette.secondary,
        ),
        (
            "[IsReadOnly=True]:focus-visible",
            "Transparent",
            palette.foreground,
            palette.foreground,
            palette.secondary,
        ),
        (
            ":disabled",
            "Transparent",
            "Transparent",
            palette.secondary,
            palette.secondary,
        ),
    ] {
        styles.add(
            style(&format!("TextBox.phone-input{state}"))
                .set(TemplatedControl::background_property(), background)
                .set(TemplatedControl::border_brush_property(), outline)
                .set(TemplatedControl::foreground_property(), foreground)
                .set(TextBox::placeholder_foreground_property(), placeholder)
                .set(TextBox::caret_brush_property(), foreground)
                .build(),
        );
        styles.add(
            style(&format!(
                "TextBox.phone-input{state} /template/ Border#BorderElement"
            ))
            .set(Border::border_brush_property(), outline)
            .set(Border::border_thickness_property(), CONTROL_BORDER)
            .set(Border::corner_radius_property(), 0.0)
            .set(Border::background_property(), background)
            .build(),
        );
        styles.add(
            style(&format!(
                "TextBox.phone-input{state} /template/ ScrollViewer#PART_ScrollViewer"
            ))
            .set(TemplatedControl::foreground_property(), foreground)
            .build(),
        );
    }
    // Read-only remains focusable for selection and copying. Its native
    // presenter owns the selection; suppress only the editing caret.
    styles.add(
        style("TextBox.phone-input[IsReadOnly=True]")
            .set(TextBox::caret_brush_property(), "Transparent")
            .build(),
    );
    styles.add(
        style("TextBox.phone-input:disabled")
            .set(TextBox::caret_brush_property(), "Transparent")
            .build(),
    );
    styles.add(
        style("TextBox.phone-input:focus /template/ Button#DeleteButton")
            .set(Visual::is_visible_property(), false)
            .build(),
    );
}

#[cfg(test)]
mod tests;
