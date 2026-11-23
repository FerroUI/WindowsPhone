use crate::{
    DEFAULT_ACCENT, FONT_FAMILY, Palette,
    compiled_markup::THEME_PARTS_FERROUI_THEMES_WINUI_WINUITHEME as PARTS, templates,
};
use ferroui_base::animation::Animatable;
use ferroui_base::layout::{HorizontalAlignment as H, Layoutable, VerticalAlignment as V};
use ferroui_base::media::{FontFamily, FontWeight};
use ferroui_base::styling::{Styles, ThemeVariant};
use ferroui_base::{Ref, RelativePoint, StyledElement, Visual};
use ferroui_controls::primitives::{ScrollBarVisibility, TemplatedControl};
use ferroui_controls::{Application, ContentControl, ItemsControl, ScrollViewer, TextBlock};
use ferroui_view::style;
use std::cell::RefCell;

ferroui_base::ferro_thread_local! {
    static PALETTE_STYLES: RefCell<Option<Ref<Styles>>> = const { RefCell::new(None) };
}

pub fn install_theme(app: &Application) {
    app.styles()
        .add(ferroui_themes_winui::WinUITheme::with_parts(None, PARTS).as_style());
    let styles = Styles::new();
    app.styles().add(styles.clone());
    PALETTE_STYLES.with(|current| *current.borrow_mut() = Some(styles));
    set_palette(app, DEFAULT_ACCENT, false);
}

/// Replaces one bounded palette style group, preserving the WinUI template tree.
pub fn set_palette(app: &Application, accent: &str, light: bool) {
    let palette = Palette::new(accent, light);
    let populated = PALETTE_STYLES.with(|current| {
        current
            .borrow()
            .as_ref()
            .is_some_and(|styles| !styles.snapshot().is_empty())
    });
    if populated && crate::tokens::PALETTE.with(|current| *current.borrow() == palette) {
        return;
    }
    crate::tokens::PALETTE.with(|current| *current.borrow_mut() = palette.clone());
    app.set_requested_theme_variant(Some(if light {
        ThemeVariant::light()
    } else {
        ThemeVariant::dark()
    }));
    PALETTE_STYLES.with(|current| {
        let Some(styles) = current.borrow().clone() else {
            return;
        };
        styles.clear();
        install_palette(&styles, &palette);
    });
}

fn install_palette(styles: &Styles, p: &Palette) {
    styles.add(
        style("TextBlock")
            .set(
                TextBlock::font_family_property(),
                FontFamily::new(FONT_FAMILY),
            )
            .set(TextBlock::foreground_property(), p.foreground)
            .build(),
    );
    for selector in [
        "Button.phone-flat",
        "Button.phone-tile",
        "Button.phone-action",
        "Button.phone-button",
        "Button.phone-pivot",
        "Button.phone-row",
    ] {
        styles.add(
            style(selector)
                .set(
                    StyledElement::theme_property(),
                    Some(templates::button_theme()),
                )
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
                .set(
                    Visual::render_transform_origin_property(),
                    RelativePoint::CENTER,
                )
                .set(
                    Animatable::transitions_property(),
                    crate::motion::press_transitions(),
                )
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
        styles.add(
            style(&format!("{selector}:pressed"))
                .set(
                    Visual::opacity_property(),
                    if selector == "Button.phone-tile" {
                        0.94
                    } else {
                        0.76
                    },
                )
                .set(
                    Visual::render_transform_property(),
                    crate::motion::scale(if p.motion_enabled && selector != "Button.phone-tile" {
                        0.965
                    } else {
                        1.0
                    }),
                )
                .build(),
        );
        styles.add(
            style(&format!("{selector}:disabled"))
                .set(Visual::opacity_property(), 0.35)
                .build(),
        );
        styles.add(
            style(&format!("{selector}:focus-visible"))
                .set(TemplatedControl::border_brush_property(), p.foreground)
                .set(TemplatedControl::border_thickness_property(), 2.0)
                .build(),
        );
    }
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
            .set(TemplatedControl::border_thickness_property(), 2.0)
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
    styles.add(
        style("ToggleButton.phone-toggle")
            .set(
                StyledElement::theme_property(),
                Some(templates::toggle_theme()),
            )
            .set(TemplatedControl::background_property(), "Transparent")
            .set(TemplatedControl::border_thickness_property(), 0.0)
            .set(TemplatedControl::padding_property(), 0.0)
            .set(
                ContentControl::horizontal_content_alignment_property(),
                H::Stretch,
            )
            .build(),
    );
    styles.add(
        style("TextBlock.phone-action-label")
            .set(Visual::is_visible_property(), false)
            .build(),
    );
    styles.add(
        style("Border.phone-app-bar.expanded TextBlock.phone-action-label")
            .set(Visual::is_visible_property(), true)
            .build(),
    );
    install_lists(styles);
    crate::inputs::install(styles, p);
    crate::sliders::install(styles, p);
    crate::pickers::install(styles, p);
}

fn install_lists(styles: &Styles) {
    styles.add(
        style("ListBox.phone-list")
            .set(TemplatedControl::background_property(), "Transparent")
            .set(TemplatedControl::border_thickness_property(), 0.0)
            .set(TemplatedControl::padding_property(), 0.0)
            .set(
                ItemsControl::item_container_theme_property(),
                Some(templates::item_theme()),
            )
            .set(
                ScrollViewer::horizontal_scroll_bar_visibility_property(),
                ScrollBarVisibility::Disabled,
            )
            .set(
                ScrollViewer::vertical_scroll_bar_visibility_property(),
                ScrollBarVisibility::Hidden,
            )
            .build(),
    );
    styles.add(
        style("ListBox.phone-list ListBoxItem")
            .set(
                StyledElement::theme_property(),
                Some(templates::item_theme()),
            )
            .set(TemplatedControl::background_property(), "Transparent")
            .set(TemplatedControl::padding_property(), 0.0)
            .set(TemplatedControl::border_thickness_property(), 0.0)
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
}
