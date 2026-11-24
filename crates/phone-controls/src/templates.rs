//! Presentation-only templates: Button and ListBoxItem still own interaction.
use ferroui_base::data::TemplateBinding;
use ferroui_base::styling::{ControlTheme, Setter, Styles};
use ferroui_base::{Ref, Thickness};
use ferroui_controls::presenters::ContentPresenter;
use ferroui_controls::primitives::TemplatedControl;
use ferroui_controls::templates::{
    FuncControlTemplate, FuncTemplateNameScopeExtensions, IControlTemplate,
};
use ferroui_controls::{Border, Button, ContentControl, Grid};
use ferroui_view::style;
use std::{cell::OnceCell, rc::Rc};

ferroui_base::ferro_thread_local! {
    static BUTTON: OnceCell<Ref<ControlTheme>> = const { OnceCell::new() };
    static CONTENT: OnceCell<Rc<dyn IControlTemplate>> = const { OnceCell::new() };
}

pub(crate) const BUTTON_SELECTORS: [&str; 7] = [
    "Button.phone-flat",
    "Button.phone-tile",
    "Button.phone-action",
    "Button.phone-more",
    "Button.phone-button",
    "Button.phone-pivot",
    "Button.phone-row",
];

/// Structural templates live outside the replaceable palette. A color change
/// must not detach a command presenter, focus target or native list container.
/// Set Template directly: a style-owned Theme is temporarily removed when
/// palettes invalidate style frames, before the next theme-application pass.
pub(crate) fn install(styles: &Styles) {
    for selector in BUTTON_SELECTORS {
        styles.add(
            style(selector)
                .set(
                    TemplatedControl::template_property(),
                    Some(content_template()),
                )
                .build(),
        );
    }
    styles.add(
        style("ToggleButton.phone-toggle")
            .set(
                TemplatedControl::template_property(),
                Some(content_template()),
            )
            .build(),
    );
    // Keep the native ListBox's ItemContainerTheme stable. A style-owned
    // override would refresh its generated containers on palette invalidation;
    // the direct container template below supplies the phone presentation.
    styles.add(
        style("ListBox.phone-list ListBoxItem")
            .set(
                TemplatedControl::template_property(),
                Some(content_template()),
            )
            .build(),
    );
}

fn content_theme<T: ferroui_base::ObjectType>(
    slot: &OnceCell<Ref<ControlTheme>>,
) -> Ref<ControlTheme> {
    slot.get_or_init(|| {
        let theme = ControlTheme::for_type::<T>();
        theme.add_setter(Setter::new(
            TemplatedControl::template_property(),
            Some(content_template()),
        ));
        theme
    })
    .clone()
}

fn content_template() -> Rc<dyn IControlTemplate> {
    CONTENT.with(|slot| {
        slot.get_or_init(|| {
            FuncControlTemplate::new(|_, scope| {
                let content = ContentPresenter::new();
                content.set_name(Some("PART_ContentPresenter".to_owned()));
                for (target, source) in [
                    (
                        ContentPresenter::content_property().as_property(),
                        ContentControl::content_property().as_property(),
                    ),
                    (
                        ContentPresenter::content_template_property().as_property(),
                        ContentControl::content_template_property().as_property(),
                    ),
                    (
                        ContentPresenter::background_property().as_property(),
                        TemplatedControl::background_property().as_property(),
                    ),
                    (
                        ContentPresenter::foreground_property().as_property(),
                        TemplatedControl::foreground_property().as_property(),
                    ),
                    (
                        ContentPresenter::border_brush_property().as_property(),
                        TemplatedControl::border_brush_property().as_property(),
                    ),
                    (
                        ContentPresenter::border_thickness_property().as_property(),
                        TemplatedControl::border_thickness_property().as_property(),
                    ),
                    (
                        ContentPresenter::corner_radius_property().as_property(),
                        TemplatedControl::corner_radius_property().as_property(),
                    ),
                    (
                        ContentPresenter::padding_property().as_property(),
                        TemplatedControl::padding_property().as_property(),
                    ),
                    (
                        ContentPresenter::horizontal_content_alignment_property().as_property(),
                        ContentControl::horizontal_content_alignment_property().as_property(),
                    ),
                    (
                        ContentPresenter::vertical_content_alignment_property().as_property(),
                        ContentControl::vertical_content_alignment_property().as_property(),
                    ),
                    (
                        ContentPresenter::font_size_property().as_property(),
                        TemplatedControl::font_size_property().as_property(),
                    ),
                    (
                        ContentPresenter::font_family_property().as_property(),
                        TemplatedControl::font_family_property().as_property(),
                    ),
                    (
                        ContentPresenter::font_weight_property().as_property(),
                        TemplatedControl::font_weight_property().as_property(),
                    ),
                ] {
                    content.bind_binding(target, &*TemplateBinding::new(source));
                }
                let root = Grid::new();
                root.children()
                    .add(content.register_in_name_scope(&**scope));
                // Keyboard focus is a separate overlay, so revealing it cannot
                // remeasure the command content or shift a native pointer target.
                let focus = Border::new();
                focus.set_name(Some("PhoneControlFocus".into()));
                focus.set_border_thickness(Thickness::uniform(1.5));
                focus.set_margin(Thickness::uniform(-3.0));
                focus.set_is_hit_test_visible(false);
                root.children().add(focus.register_in_name_scope(&**scope));
                root.upcast()
            }) as Rc<dyn IControlTemplate>
        })
        .clone()
    })
}

pub(crate) fn button_theme() -> Ref<ControlTheme> {
    BUTTON.with(content_theme::<Button>)
}
