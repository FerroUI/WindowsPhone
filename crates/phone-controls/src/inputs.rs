use crate::{FONT_FAMILY, Palette};
use ferroui_base::{Visual, layout::Layoutable, media::FontFamily, styling::Styles};
use ferroui_controls::{Border, TextBox, primitives::TemplatedControl};
use ferroui_view::style;

pub(crate) fn install(styles: &Styles, palette: &Palette) {
    styles.add(
        style("TextBox.phone-input")
            .set(
                TemplatedControl::font_family_property(),
                FontFamily::new(FONT_FAMILY),
            )
            .set(
                TemplatedControl::font_size_property(),
                20.0 * palette.text_scale,
            )
            .set(TemplatedControl::foreground_property(), "#000000")
            .set(TemplatedControl::background_property(), "#FFFFFF")
            .set(TemplatedControl::border_brush_property(), "#D6D6D6")
            .set(TemplatedControl::border_thickness_property(), 2.0)
            .set(TemplatedControl::corner_radius_property(), 0.0)
            .set(TemplatedControl::padding_property(), (10.0, 7.0))
            .set(Layoutable::min_width_property(), 0.0)
            .set(Layoutable::min_height_property(), 44.0)
            .set(TextBox::caret_brush_property(), "#000000")
            .set(TextBox::selection_brush_property(), palette.accent.as_str())
            .set(TextBox::selection_foreground_brush_property(), "#FFFFFF")
            .set(TextBox::placeholder_foreground_property(), "#777777")
            .build(),
    );
    for (state, outline, background) in [
        ("", "#D6D6D6", "#FFFFFF"),
        (":pointerover", "#AAAAAA", "#FFFFFF"),
        (":focus", palette.accent.as_str(), "#FFFFFF"),
        (":disabled", "#777777", "#CCCCCC"),
    ] {
        styles.add(
            style(&format!(
                "TextBox.phone-input{state} /template/ Border#BorderElement"
            ))
            .set(Border::border_brush_property(), outline)
            .set(Border::border_thickness_property(), 2.0)
            .set(Border::corner_radius_property(), 0.0)
            .set(Border::background_property(), background)
            .build(),
        );
        // WinUI applies state foregrounds to the template's ScrollViewer.
        // Keep its inherited TextPresenter ink aligned with our light input
        // surface, including when the surrounding phone theme is dark.
        styles.add(
            style(&format!(
                "TextBox.phone-input{state} /template/ ScrollViewer#PART_ScrollViewer"
            ))
            .set(TemplatedControl::foreground_property(), "#000000")
            .build(),
        );
    }
    styles.add(
        style("TextBox.phone-input:focus")
            .set(TemplatedControl::foreground_property(), "#000000")
            .set(TextBox::caret_brush_property(), "#000000")
            .build(),
    );
    styles.add(
        style("TextBox.phone-input:focus /template/ Button#DeleteButton")
            .set(Visual::is_visible_property(), false)
            .build(),
    );
}
