//! Phone checkbox/radio presentation; native controls retain state, keyboard,
//! radio grouping and UI Automation semantics, including indeterminate checks.
use crate::{Palette, CHOICE_INDICATOR_SIZE, CONTROL_BORDER, FONT_FAMILY, MIN_TOUCH_TARGET};
use ferroui_base::data::TemplateBinding;
use ferroui_base::layout::{HorizontalAlignment as H, Layoutable, VerticalAlignment as V};
use ferroui_base::media::{Brushes, FontFamily, Geometry, TextWrapping};
use ferroui_base::styling::{ControlTheme, Setter, Styles};
use ferroui_base::{CornerRadius, Ref, StyledElement, Thickness, Visual};
use ferroui_controls::presenters::ContentPresenter;
use ferroui_controls::primitives::TemplatedControl;
use ferroui_controls::templates::{
    FuncControlTemplate, FuncTemplateNameScopeExtensions, IControlTemplate,
};
use ferroui_controls::{
    Border, CheckBox, ColumnDefinitions, ContentControl, Grid, PathIcon, RadioButton,
};
use ferroui_view::{prelude::*, style};
use std::{cell::OnceCell, rc::Rc};

ferroui_base::ferro_thread_local! {
    static CHECK: OnceCell<Ref<ControlTheme>> = const { OnceCell::new() };
    static RADIO: OnceCell<Ref<ControlTheme>> = const { OnceCell::new() };
}

/// A two-state checkbox. Its native IsCheckedChanged event also supports
/// Space and automation, without synthesizing a pointer click.
pub fn check(label: &str, value: bool, on_change: impl Fn(bool) + 'static) -> Element {
    rsx! {CheckBox {class:"phone-check",content:label,
        is_checked:crate::checked_state(Some(value),move |next|on_change(next.unwrap_or(false))),
        automation_properties_name:label,
        on_click:|event|event.set_handled(true)
    }}
}

/// A native exclusive option. A selected radio cannot unselect itself; the
/// framework owns group membership and updates other members atomically.
pub fn radio(label: &str, group: &str, selected: bool, on_select: impl Fn() + 'static) -> Element {
    rsx! {RadioButton {class:"phone-radio",content:label,group_name:group,
        is_checked:crate::checked_state(Some(selected),move |next|if next==Some(true){on_select();}),
        automation_properties_name:label,
        on_click:|event|event.set_handled(true)
    }}
}

pub(crate) fn install(styles: &Styles, p: &Palette) {
    for (selector, theme) in [
        (
            "CheckBox.phone-check",
            CHECK.with(|slot| theme::<CheckBox>(slot, false)),
        ),
        (
            "RadioButton.phone-radio",
            RADIO.with(|slot| theme::<RadioButton>(slot, true)),
        ),
    ] {
        styles.add(
            style(selector)
                .set(StyledElement::theme_property(), Some(theme))
                .set(
                    TemplatedControl::font_family_property(),
                    FontFamily::new(FONT_FAMILY),
                )
                .set(TemplatedControl::font_size_property(), 22.0 * p.text_scale)
                .set(TemplatedControl::foreground_property(), p.foreground)
                .set(TemplatedControl::background_property(), "Transparent")
                .set(TemplatedControl::padding_property(), 0.0)
                .set(TemplatedControl::border_thickness_property(), 0.0)
                .set(Layoutable::horizontal_alignment_property(), H::Stretch)
                .set(Layoutable::min_width_property(), 0.0)
                .set(Layoutable::min_height_property(), MIN_TOUCH_TARGET)
                .build(),
        );
        styles.add(
            style(&format!("{selector} /template/ Border#PhoneChoiceOutline"))
                .set(Border::border_brush_property(), p.foreground)
                .set(Border::background_property(), p.background)
                .build(),
        );
        styles.add(
            style(&format!(
                "{selector}:pressed /template/ Border#PhoneChoiceOutline"
            ))
            .set(Border::background_property(), p.accent.as_str())
            .build(),
        );
        styles.add(
            style(&format!("{selector} /template/ Border#PhoneChoiceMark"))
                .set(Border::background_property(), p.foreground)
                .build(),
        );
        styles.add(
            style(&format!("{selector} /template/ PathIcon#PhoneChoiceCheck"))
                .set(TemplatedControl::foreground_property(), p.foreground)
                .build(),
        );
        for name in ["Border#PhoneChoiceMark", "PathIcon#PhoneChoiceCheck"] {
            styles.add(
                style(&format!("{selector} /template/ {name}"))
                    .set(Visual::is_visible_property(), false)
                    .build(),
            );
            styles.add(
                style(&format!("{selector}:checked /template/ {name}"))
                    .set(Visual::is_visible_property(), true)
                    .build(),
            );
        }
        styles.add(
            style(&format!(
                "{selector}:indeterminate /template/ Border#PhoneChoiceIndeterminate"
            ))
            .set(Border::background_property(), p.foreground)
            .set(Visual::is_visible_property(), true)
            .build(),
        );
        styles.add(
            style(&format!(
                "{selector}:not(:indeterminate) /template/ Border#PhoneChoiceIndeterminate"
            ))
            .set(Visual::is_visible_property(), false)
            .build(),
        );
        styles.add(
            style(&format!(
                "{selector}:focus-visible /template/ Border#PhoneChoiceFocus"
            ))
            .set(Border::border_brush_property(), p.foreground)
            .build(),
        );
        styles.add(
            style(&format!("{selector}:disabled"))
                .set(Visual::opacity_property(), 0.3)
                .build(),
        );
    }
}

fn theme<T: ferroui_base::ObjectType>(
    slot: &OnceCell<Ref<ControlTheme>>,
    radio: bool,
) -> Ref<ControlTheme> {
    slot.get_or_init(|| {
        let theme = ControlTheme::for_type::<T>();
        let template: Rc<dyn IControlTemplate> = FuncControlTemplate::new(move |_, scope| {
            let focus = Border::new();
            focus.set_name(Some("PhoneChoiceFocus".into()));
            focus.set_background(Some(Brushes::transparent()));
            focus.set_border_thickness(Thickness::uniform(1.0));
            let root = Grid::new();
            root.set_column_definitions(
                ColumnDefinitions::parse("44,*").expect("fixed choice columns"),
            );
            root.set_column_spacing(8.0);

            let outline = Border::new();
            outline.set_name(Some("PhoneChoiceOutline".into()));
            outline.set_width(CHOICE_INDICATOR_SIZE);
            outline.set_height(CHOICE_INDICATOR_SIZE);
            outline.set_horizontal_alignment(H::Center);
            outline.set_vertical_alignment(V::Center);
            outline.set_corner_radius(CornerRadius::uniform(if radio {
                CHOICE_INDICATOR_SIZE
            } else {
                0.0
            }));
            outline.set_border_thickness(Thickness::uniform(CONTROL_BORDER));
            if radio {
                let dot = Border::new();
                dot.set_name(Some("PhoneChoiceMark".into()));
                dot.set_width(crate::reference_pixels(14.0));
                dot.set_height(crate::reference_pixels(14.0));
                dot.set_corner_radius(CornerRadius::uniform(CHOICE_INDICATOR_SIZE));
                dot.set_horizontal_alignment(H::Center);
                dot.set_vertical_alignment(V::Center);
                outline.set_child(dot.register_in_name_scope(&**scope));
            } else {
                let marks = Grid::new();
                let check = PathIcon::new();
                check.set_name(Some("PhoneChoiceCheck".into()));
                check.set_data(Some(
                    Geometry::parse(crate::paths::path(crate::Icon::Check))
                        .expect("phone check geometry"),
                ));
                check.set_width(crate::reference_pixels(24.0));
                check.set_height(crate::reference_pixels(24.0));
                check.set_horizontal_alignment(H::Center);
                check.set_vertical_alignment(V::Center);
                marks.children().add(check.register_in_name_scope(&**scope));
                let mixed = Border::new();
                mixed.set_name(Some("PhoneChoiceIndeterminate".into()));
                mixed.set_width(crate::reference_pixels(16.0));
                mixed.set_height(crate::reference_pixels(5.0));
                mixed.set_horizontal_alignment(H::Center);
                mixed.set_vertical_alignment(V::Center);
                marks.children().add(mixed.register_in_name_scope(&**scope));
                outline.set_child(marks);
            }
            root.children()
                .add(outline.register_in_name_scope(&**scope));
            let content = ContentPresenter::new();
            content.set_name(Some("PART_ContentPresenter".into()));
            content.set_vertical_alignment(V::Center);
            content.set_text_wrapping(TextWrapping::Wrap);
            Grid::set_column(&content, 1);
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
                    ContentPresenter::foreground_property().as_property(),
                    TemplatedControl::foreground_property().as_property(),
                ),
                (
                    ContentPresenter::font_family_property().as_property(),
                    TemplatedControl::font_family_property().as_property(),
                ),
                (
                    ContentPresenter::font_size_property().as_property(),
                    TemplatedControl::font_size_property().as_property(),
                ),
            ] {
                content.bind_binding(target, &*TemplateBinding::new(source));
            }
            root.children()
                .add(content.register_in_name_scope(&**scope));
            focus.set_child(root);
            focus.register_in_name_scope(&**scope).upcast()
        });
        theme.add_setter(Setter::new(
            TemplatedControl::template_property(),
            Some(template),
        ));
        theme
    })
    .clone()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::{Cell, RefCell};

    #[test]
    fn checkbox_publishes_native_changes_without_initial_or_duplicate_notifications() {
        let values = Rc::new(RefCell::new(Vec::new()));
        let recorded = values.clone();
        let (control, owner) = check("sync mail", true, move |value| {
            recorded.borrow_mut().push(value)
        })
        .mount_root();
        let checkbox = control.cast::<CheckBox>().expect("a native checkbox");
        assert_eq!(checkbox.is_checked(), Some(true));
        assert!(
            values.borrow().is_empty(),
            "mounting a saved value must not write it again"
        );
        checkbox.set_is_checked(Some(false));
        checkbox.set_is_checked(Some(false));
        checkbox.set_is_checked(Some(true));
        assert_eq!(*values.borrow(), [false, true]);
        owner.dispose();
    }

    #[test]
    fn radio_selection_keeps_native_exclusive_group_and_notifies_only_selected_option() {
        let first_count = Rc::new(Cell::new(0));
        let second_count = Rc::new(Cell::new(0));
        let first_recorded = first_count.clone();
        let second_recorded = second_count.clone();
        let (control, owner) = stack_panel()
            .child(radio("dark", "phone-background-test", true, move || {
                first_recorded.set(first_recorded.get() + 1)
            }))
            .child(radio("light", "phone-background-test", false, move || {
                second_recorded.set(second_recorded.get() + 1)
            }))
            .into_element()
            .mount_root();
        let panel = control.cast::<StackPanel>().unwrap();
        let first = panel.children().get(0).cast::<RadioButton>().unwrap();
        let second = panel.children().get(1).cast::<RadioButton>().unwrap();
        assert_eq!((first_count.get(), second_count.get()), (0, 0));
        second.set_is_checked(Some(true));
        assert_eq!(first.is_checked(), Some(false));
        assert_eq!(second.is_checked(), Some(true));
        assert_eq!((first_count.get(), second_count.get()), (0, 1));
        first.set_is_checked(Some(true));
        assert_eq!(second.is_checked(), Some(false));
        assert_eq!((first_count.get(), second_count.get()), (1, 1));
        owner.dispose();
    }
}
