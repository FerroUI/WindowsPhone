use crate::{
    DEFAULT_ACCENT, FONT_FAMILY, Palette,
    compiled_markup::THEME_PARTS_FERROUI_THEMES_WINUI_WINUITHEME as PARTS,
};
use ferroui_base::layout::{HorizontalAlignment as H, Layoutable, VerticalAlignment as V};
use ferroui_base::media::FontFamily;
use ferroui_base::styling::{Styles, ThemeVariant};
use ferroui_base::{Ref, Visual};
use ferroui_controls::primitives::{ScrollBarVisibility, TemplatedControl};
use ferroui_controls::{Application, ContentControl, ScrollViewer, TextBlock};
use ferroui_view::style;
use std::cell::RefCell;

ferroui_base::ferro_thread_local! {
    static PALETTE_STYLES: RefCell<Option<Ref<Styles>>> = const { RefCell::new(None) };
}

pub fn install_theme(app: &Application) {
    app.styles()
        .add(ferroui_themes_winui::WinUITheme::with_parts(None, PARTS).as_style());
    // A preference change updates ink and motion while native input controls
    // retain their template parts, focus and gesture ownership.
    let templates = Styles::new();
    crate::templates::install(&templates);
    crate::choices::install_templates(&templates);
    crate::sliders::install_templates(&templates);
    crate::switches::install_templates(&templates);
    crate::pivot_host::install_template(&templates);
    crate::panorama::install_template(&templates);
    crate::progress::install_template(&templates);
    app.styles().add(templates);
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
    app.set_requested_theme_variant(Some(if palette.light {
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
    crate::button_styles::install(styles, p);
    styles.add(
        style("ToggleButton.phone-toggle")
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
        // Standalone actions have no ellipsis. Only an actual collapsed
        // application bar may hide the caption beneath an action's icon.
        style("Border.phone-app-bar.collapsed TextBlock.phone-action-label")
            .set(Visual::is_visible_property(), false)
            .build(),
    );
    styles.add(
        style("Border.phone-app-bar.expanded TextBlock.phone-action-label")
            .set(Visual::is_visible_property(), true)
            .build(),
    );
    install_lists(styles);
    crate::pivot_host::install(styles, p);
    crate::inputs::install(styles, p);
    crate::sliders::install(styles, p);
    crate::progress::install(styles, p);
    crate::pickers::install(styles, p);
    crate::switches::install(styles, p);
    crate::choices::install(styles, p);
}

fn install_lists(styles: &Styles) {
    styles.add(
        style("ListBox.phone-list")
            .set(TemplatedControl::background_property(), "Transparent")
            .set(TemplatedControl::border_thickness_property(), 0.0)
            .set(TemplatedControl::padding_property(), 0.0)
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

#[cfg(test)]
mod tests {
    use super::*;
    use ferroui_base::FerroLocator;
    use ferroui_base::platform::IPlatformRenderInterface;
    use ferroui_controls::Border;
    use ferroui_view::prelude::*;

    #[test]
    fn action_captions_only_collapse_inside_an_actual_application_bar() {
        let _scope = FerroLocator::enter_scope();
        FerroLocator::current_mutable()
            .bind::<dyn IPlatformRenderInterface>()
            .to_constant(ferroui_headless::testing::headless_platform_render_interface());
        let content: Element = stack_panel()
            .child(crate::action(crate::Icon::Check, "confirm", || {}))
            .child(crate::app_bar(
                vec![crate::action(crate::Icon::Plus, "add", || {})],
                false,
                vec![],
            ))
            .into();
        let (root, owner) = content.mount_root();
        let styles = Styles::new();
        install_palette(&styles, &Palette::new(DEFAULT_ACCENT, false));
        root.styles().add(styles);
        root.apply_styling();
        for element in root.get_logical_descendants().collect::<Vec<_>>() {
            element.apply_styling();
        }
        let label = |name: &str| {
            root.get_logical_descendants()
                .filter_map(|element| element.cast::<TextBlock>())
                .find(|element| element.has_name(name))
                .expect("mounted action caption")
        };
        let standalone = label("AppBarLabel-confirm");
        let primary = label("AppBarLabel-add");
        let bar = root
            .get_logical_descendants()
            .filter_map(|element| element.cast::<Border>())
            .find(|element| element.has_name("PhoneApplicationBar"))
            .expect("mounted application bar");
        assert!(
            standalone.is_visible(),
            "standalone actions have no ellipsis"
        );
        assert!(
            !primary.is_visible(),
            "the real collapsed bar hides its caption"
        );

        bar.classes().remove("collapsed");
        bar.classes().add("expanded");
        assert!(
            primary.is_visible(),
            "expansion reveals the existing caption"
        );
        assert!(standalone.is_visible());

        bar.classes().remove("expanded");
        bar.classes().add("collapsed");
        assert!(
            !primary.is_visible(),
            "collapsing restores the icon-only bar"
        );
        assert!(
            standalone.is_visible(),
            "bar state cannot hide an unrelated action"
        );
        owner.dispose();
    }
}
