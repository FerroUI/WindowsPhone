//! Presentation-only templates: Button and ListBoxItem still own interaction.
use ferroui_base::data::TemplateBinding;
use ferroui_base::styling::{ControlTheme, Setter};
use ferroui_base::Ref;
use ferroui_controls::presenters::ContentPresenter;
use ferroui_controls::primitives::{TemplatedControl, ToggleButton};
use ferroui_controls::templates::{
    FuncControlTemplate, FuncTemplateNameScopeExtensions, IControlTemplate,
};
use ferroui_controls::{Button, ContentControl, ListBoxItem};
use std::{cell::OnceCell, rc::Rc};

ferroui_base::ferro_thread_local! {
    static BUTTON: OnceCell<Ref<ControlTheme>> = const { OnceCell::new() };
    static TOGGLE: OnceCell<Ref<ControlTheme>> = const { OnceCell::new() };
    static ITEM: OnceCell<Ref<ControlTheme>> = const { OnceCell::new() };
}

fn content_theme<T: ferroui_base::ObjectType>(
    slot: &OnceCell<Ref<ControlTheme>>,
) -> Ref<ControlTheme> {
    slot.get_or_init(|| {
        let theme = ControlTheme::for_type::<T>();
        let template: Rc<dyn IControlTemplate> = FuncControlTemplate::new(|_, scope| {
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
            content.register_in_name_scope(&**scope).upcast()
        });
        theme.add_setter(Setter::new(
            TemplatedControl::template_property(),
            Some(template),
        ));
        theme
    })
    .clone()
}

pub(crate) fn button_theme() -> Ref<ControlTheme> {
    BUTTON.with(content_theme::<Button>)
}
pub(crate) fn toggle_theme() -> Ref<ControlTheme> {
    TOGGLE.with(content_theme::<ToggleButton>)
}
pub(crate) fn item_theme() -> Ref<ControlTheme> {
    ITEM.with(content_theme::<ListBoxItem>)
}
