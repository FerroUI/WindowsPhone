//! Native checkbox/radio behavior with phone presentation.
//!
//! The checkbox geometry and immediate pressed/disabled resource roles are
//! adapted from Microsoft's MS-PL Windows Phone Toolkit WP8
//! `ListPickerPage.xaml`, `AnyFontSizeCheckBoxStyle`, revision
//! 4e4fd5d36c769c004f7ccc7daf78a5ffb8ef18df. This is a Toolkit ListPicker
//! reference, not a recovered universal Windows Phone 8.1 OS template.
//! The radio circle, desktop focus overlay and 22-pixel label typography are
//! application adaptations. Native state, keyboard, grouping and automation
//! remain the responsibility of CheckBox and RadioButton.
use crate::{
    CHOICE_INDICATOR_SIZE, CONTROL_BORDER, FONT_FAMILY, MIN_TOUCH_TARGET, Palette,
    reference_pixels as px,
};
use ferroui_base::data::TemplateBinding;
use ferroui_base::layout::{HorizontalAlignment as H, Layoutable, VerticalAlignment as V};
use ferroui_base::media::{Brushes, FontFamily, FontWeight, Geometry, Stretch, TextWrapping};
use ferroui_base::styling::Styles;
use ferroui_base::{CornerRadius, Ref, Thickness, Visual};
use ferroui_controls::presenters::ContentPresenter;
use ferroui_controls::primitives::TemplatedControl;
use ferroui_controls::shapes::{Path, Shape};
use ferroui_controls::templates::{
    FuncControlTemplate, FuncTemplateNameScopeExtensions, IControlTemplate,
};
use ferroui_controls::{
    Border, CheckBox, ColumnDefinitions, ContentControl, Control, Grid, RadioButton, TextBlock,
};
use ferroui_view::{prelude::*, style};
use std::{cell::OnceCell, rc::Rc};

// Copyright (c) Microsoft Corporation. Adapted under the Microsoft Public
// License from the Toolkit CheckMark path; see NOTICE and the bundled license.
const TOOLKIT_CHECK_PATH: &str = "M0,123 L39,93 L124,164 L256,18 L295,49 L124,240 z";

ferroui_base::ferro_thread_local! {
    static CHECK: OnceCell<Rc<dyn IControlTemplate>> = const { OnceCell::new() };
    static RADIO: OnceCell<Rc<dyn IControlTemplate>> = const { OnceCell::new() };
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

/// Keep the actual template outside the replaceable palette style group. A
/// light/dark or accent change must not recreate a pressed/focused control.
pub(crate) fn install_templates(styles: &Styles) {
    for (selector, radio) in [
        ("CheckBox.phone-check", false),
        ("RadioButton.phone-radio", true),
    ] {
        styles.add(
            style(selector)
                .set(TemplatedControl::template_property(), Some(template(radio)))
                .build(),
        );
    }
}

pub(crate) fn install(styles: &Styles, p: &Palette) {
    // The original source names resource roles, but the SDK's per-theme ARGB
    // table was not recovered. Our mapping is explicit: normal ink uses the
    // palette foreground; pressed fill/mark use foreground/background; the
    // disabled role uses secondary ink. These colors are an adaptation.
    for selector in ["CheckBox.phone-check", "RadioButton.phone-radio"] {
        styles.add(
            style(selector)
                .set(
                    TemplatedControl::font_family_property(),
                    FontFamily::new(FONT_FAMILY),
                )
                .set(TemplatedControl::font_size_property(), 22.0 * p.text_scale)
                .set(TemplatedControl::font_weight_property(), FontWeight::Normal)
                .set(TemplatedControl::foreground_property(), p.foreground)
                .set(TemplatedControl::background_property(), "Transparent")
                .set(TemplatedControl::padding_property(), 0.0)
                .set(TemplatedControl::border_thickness_property(), 0.0)
                .set(Layoutable::horizontal_alignment_property(), H::Stretch)
                .set(Layoutable::min_width_property(), 0.0)
                .set(Layoutable::min_height_property(), MIN_TOUCH_TARGET)
                .build(),
        );
        for (state, fill, ink, edge, caption) in [
            ("", "Transparent", p.foreground, p.foreground, p.foreground),
            (
                ":pressed",
                p.foreground,
                p.background,
                p.foreground,
                p.foreground,
            ),
            (
                ":disabled",
                "Transparent",
                p.secondary,
                p.secondary,
                p.secondary,
            ),
        ] {
            styles.add(
                style(&format!(
                    "{selector}{state} /template/ Border#PhoneChoiceOutline"
                ))
                .set(Border::border_brush_property(), edge)
                .set(Border::background_property(), fill)
                .build(),
            );
            for name in ["PhoneChoiceMark", "PhoneChoiceIndeterminate"] {
                styles.add(
                    style(&format!("{selector}{state} /template/ Border#{name}"))
                        .set(Border::background_property(), ink)
                        .build(),
                );
            }
            styles.add(
                style(&format!(
                    "{selector}{state} /template/ Path#PhoneChoiceCheck"
                ))
                .set(Shape::fill_property(), ink)
                .build(),
            );
            styles.add(
                style(&format!(
                    "{selector}{state} /template/ ContentPresenter#PART_ContentPresenter"
                ))
                .set(ContentPresenter::foreground_property(), caption)
                .build(),
            );
            // A default string template creates a TextBlock. Its explicit
            // general TextBlock style outranks inherited presenter ink, so
            // only generated label text receives this more specific rule.
            styles.add(
                style(&format!("{selector}{state} TextBlock.phone-choice-content"))
                    .set(TextBlock::foreground_property(), caption)
                    .build(),
            );
        }
        for (part, state) in [
            ("Border#PhoneChoiceMark", ":checked"),
            ("Path#PhoneChoiceCheck", ":checked"),
            ("Border#PhoneChoiceIndeterminate", ":indeterminate"),
        ] {
            styles.add(
                style(&format!("{selector} /template/ {part}"))
                    .set(Visual::is_visible_property(), false)
                    .build(),
            );
            styles.add(
                style(&format!("{selector}{state} /template/ {part}"))
                    .set(Visual::is_visible_property(), true)
                    .build(),
            );
        }
        for (state, edge) in [
            ("", "Transparent"),
            (":focus-visible", p.foreground),
            (":disabled", "Transparent"),
        ] {
            styles.add(
                style(&format!(
                    "{selector}{state} /template/ Border#PhoneChoiceFocus"
                ))
                .set(Border::border_brush_property(), edge)
                .build(),
            );
        }
        styles.add(
            style(&format!("{selector}:disabled"))
                .set(TemplatedControl::foreground_property(), p.secondary)
                .build(),
        );
    }
}

fn template(radio: bool) -> Rc<dyn IControlTemplate> {
    let create = |slot: &OnceCell<Rc<dyn IControlTemplate>>| {
        slot.get_or_init(|| {
            FuncControlTemplate::new(move |_, scope| {
                let root = Grid::new();
                root.set_background(Some(Brushes::transparent()));
                let row = Grid::new();
                row.set_column_definitions(
                    ColumnDefinitions::parse("Auto,*").expect("choice columns"),
                );
                row.set_column_spacing(px(12.0));
                row.set_margin(Thickness::symmetric(px(12.0), px(20.0)));

                let indicator = Grid::new();
                indicator.set_width(CHOICE_INDICATOR_SIZE);
                indicator.set_height(CHOICE_INDICATOR_SIZE);
                indicator.set_vertical_alignment(V::Top);
                indicator.set_is_hit_test_visible(false);
                let outline = Border::new();
                outline.set_name(Some("PhoneChoiceOutline".into()));
                outline.set_corner_radius(CornerRadius::uniform(if radio {
                    CHOICE_INDICATOR_SIZE
                } else {
                    0.0
                }));
                outline.set_border_thickness(Thickness::uniform(CONTROL_BORDER));
                indicator
                    .children()
                    .add(outline.register_in_name_scope(&**scope));
                if radio {
                    let dot = Border::new();
                    dot.set_name(Some("PhoneChoiceMark".into()));
                    dot.set_width(px(14.0));
                    dot.set_height(px(14.0));
                    dot.set_corner_radius(CornerRadius::uniform(CHOICE_INDICATOR_SIZE));
                    dot.set_horizontal_alignment(H::Center);
                    dot.set_vertical_alignment(V::Center);
                    indicator
                        .children()
                        .add(dot.register_in_name_scope(&**scope));
                } else {
                    let check = Path::new();
                    check.set_name(Some("PhoneChoiceCheck".into()));
                    check.set_data(Some(
                        Geometry::parse(TOOLKIT_CHECK_PATH).expect("Toolkit check geometry"),
                    ));
                    check.set_width(px(23.0));
                    check.set_height(px(21.0));
                    check.set_stretch(Stretch::Fill);
                    check.set_horizontal_alignment(H::Center);
                    check.set_vertical_alignment(V::Center);
                    indicator
                        .children()
                        .add(check.register_in_name_scope(&**scope));
                    let mixed = Border::new();
                    mixed.set_name(Some("PhoneChoiceIndeterminate".into()));
                    mixed.set_width(px(16.0));
                    mixed.set_height(px(16.0));
                    mixed.set_horizontal_alignment(H::Center);
                    mixed.set_vertical_alignment(V::Center);
                    indicator
                        .children()
                        .add(mixed.register_in_name_scope(&**scope));
                }
                row.children().add(indicator);
                let content = ContentPresenter::new();
                content.set_name(Some("PART_ContentPresenter".into()));
                content.set_vertical_alignment(V::Top);
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
                        ContentPresenter::font_family_property().as_property(),
                        TemplatedControl::font_family_property().as_property(),
                    ),
                    (
                        ContentPresenter::font_size_property().as_property(),
                        TemplatedControl::font_size_property().as_property(),
                    ),
                    (
                        ContentPresenter::font_weight_property().as_property(),
                        TemplatedControl::font_weight_property().as_property(),
                    ),
                    (
                        ContentPresenter::padding_property().as_property(),
                        TemplatedControl::padding_property().as_property(),
                    ),
                    (
                        ContentPresenter::horizontal_content_alignment_property().as_property(),
                        ContentControl::horizontal_content_alignment_property().as_property(),
                    ),
                ] {
                    content.bind_binding(target, &*TemplateBinding::new(source));
                }
                content.add_property_changed_handler(Rc::new(|change| {
                    if change.property() != ContentPresenter::child_property().as_property() {
                        return;
                    }
                    let Some(presenter) = change.sender().downcast_ref::<ContentPresenter>() else {
                        return;
                    };
                    if presenter.content_template().is_some()
                        || presenter
                            .content()
                            .as_ref()
                            .and_then(Control::from_boxed)
                            .is_some()
                    {
                        return;
                    }
                    if let Some(text) = change
                        .get_new_value::<Option<Ref<Control>>>()
                        .and_then(|child| child.cast::<TextBlock>())
                    {
                        text.classes().add("phone-choice-content");
                    }
                }));
                row.children().add(content.register_in_name_scope(&**scope));
                root.children().add(row);

                // This desktop keyboard affordance is an overlay: activating it
                // never changes the indicator, text bounds or native hit target.
                let focus = Border::new();
                focus.set_name(Some("PhoneChoiceFocus".into()));
                focus.set_border_thickness(Thickness::uniform(1.0));
                focus.set_is_hit_test_visible(false);
                root.children().add(focus.register_in_name_scope(&**scope));
                root.upcast()
            })
        })
        .clone()
    };
    if radio {
        RADIO.with(create)
    } else {
        CHECK.with(create)
    }
}

#[cfg(test)]
mod tests;
