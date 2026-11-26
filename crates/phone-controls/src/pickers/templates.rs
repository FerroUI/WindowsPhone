//! Phone presentation over the native picker presenters and virtualized panels.
//!
//! Geometry follows the WP8 Toolkit picker pages (148-unit cells with a 6-unit
//! inset at the 480-unit reference width). The native presenters continue to
//! own date coercion, culture order, clock mode and transactional confirmation.
use crate::{templates as shared, Icon, FONT_FAMILY};
use ferroui_base::data::TemplateBinding;
use ferroui_base::input::{InputElement, Key};
use ferroui_base::interactivity::RoutingStrategies;
use ferroui_base::layout::{HorizontalAlignment as H, VerticalAlignment as V};
use ferroui_base::media::{
    Brushes, FontFamily, FontWeight, Geometry, Stretch, StretchDirection, TextWrapping,
};
use ferroui_base::reactive::Subscription;
use ferroui_base::threading::{Dispatcher, DispatcherPriority};
use ferroui_base::utilities::DateTime;
use ferroui_base::{AnyValue, Ref, Thickness};
use ferroui_controls::automation::AutomationProperties;
use ferroui_controls::primitives::{
    DateTimePickerPanel, DateTimePickerPanelType, ScrollBarVisibility, TemplatedControl,
    UniformGrid,
};
use ferroui_controls::templates::{
    FuncControlTemplate, FuncTemplateNameScopeExtensions, IControlTemplate,
};
use ferroui_controls::{
    Border, Button, ColumnDefinitions, ContentControl, Control, DatePickerPresenter, Grid,
    GridLength, GridUnitType, ListBoxItem, Panel, PathIcon, RowDefinitions, ScrollViewer,
    StackPanel, TextBlock, Viewbox,
};
use ferroui_view::style;
use std::{
    cell::{OnceCell, RefCell},
    rc::Rc,
};

ferroui_base::ferro_thread_local! {
    static DATE: OnceCell<Rc<dyn IControlTemplate>> = const { OnceCell::new() };
    static TIME: OnceCell<Rc<dyn IControlTemplate>> = const { OnceCell::new() };
    static ITEM: OnceCell<Rc<dyn IControlTemplate>> = const { OnceCell::new() };
}

pub(super) fn presenter_template(date: bool) -> Rc<dyn IControlTemplate> {
    let create = || {
        FuncControlTemplate::new(move |_, scope| {
            let page = Grid::new();
            page.set_name(Some("PhonePickerSurface".into()));
            page.set_row_definitions(RowDefinitions::parse("Auto,*,60").expect("picker rows"));
            page.set_clip_to_bounds(true);
            page.bind_binding(
                Panel::background_property().as_property(),
                &*TemplateBinding::new(TemplatedControl::background_property().as_property()),
            );
            let title = TextBlock::new();
            title.set_name(Some("PhonePickerTitle".into()));
            title.set_text(Some(if date { "CHOOSE DATE" } else { "CHOOSE TIME" }));
            title.set_font_family(FontFamily::new(FONT_FAMILY));
            title.set_font_weight(FontWeight::SemiBold);
            title.set_font_size(22.667 * 5.0 / 6.0);
            title.set_margin(Thickness::new(20.0, 13.333, 20.0, 20.0));
            page.children().add(title.register_in_name_scope(&**scope));

            let wheels = Grid::new();
            wheels.set_name(Some("PART_PickerContainer".into()));
            wheels.set_horizontal_alignment(H::Center);
            wheels.set_width(370.0);
            wheels.set_clip_to_bounds(true);
            Grid::set_row(&wheels, 1);
            let pointer_input = super::input::WheelInput::new(&wheels);

            let entries: &[(&str, DateTimePickerPanelType, bool)] = if date {
                &[
                    ("Month", DateTimePickerPanelType::Month, true),
                    ("Day", DateTimePickerPanelType::Day, true),
                    ("Year", DateTimePickerPanelType::Year, false),
                ]
            } else {
                &[
                    ("Hour", DateTimePickerPanelType::Hour, true),
                    ("Minute", DateTimePickerPanelType::Minute, true),
                    ("Second", DateTimePickerPanelType::Second, true),
                    ("Period", DateTimePickerPanelType::TimePeriod, false),
                ]
            };
            for (index, &(name, kind, looping)) in entries.iter().enumerate() {
                let host = Grid::new();
                host.set_name(Some(format!("PART_{name}Host")));
                // The time presenter only relocates optional seconds/period
                // columns. Hour/minute positions are part of its template
                // contract; leaving both at Grid's default zero overlaps the
                // wheels and makes the hour column unreachable by pointer.
                // The date presenter replaces these defaults for culture order.
                Grid::set_column(&host, (index * 2) as i32);
                host.set_background(Some(Brushes::transparent()));
                let selector = DateTimePickerPanel::new();
                selector.set_name(Some(format!("PART_{name}Selector")));
                selector.classes().add("phone-picker-wheel");
                selector.set_panel_type(kind);
                selector.set_item_height(native_pitch(148.0 * 5.0 / 6.0));
                selector.set_should_loop(looping);
                selector.set_background(Some(Brushes::transparent()));
                selector.styles().add(
                    style("ListBoxItem")
                        .set(TemplatedControl::template_property(), Some(item_template()))
                        .build(),
                );
                let weak_selector = selector.downgrade();
                selector.size_changed(move |_, _| {
                    if let Some(selector) = weak_selector.upgrade() {
                        // The native panel grows its recycled ring when the
                        // popup becomes a full phone page, but refreshes the
                        // ring's labels only on its initial measure. Populate
                        // the new slots and recenter the selected container
                        // after every real viewport resize. Selection, offset
                        // and scrolling remain owned by the native panel.
                        selector.refresh_items();
                    }
                });
                super::input::attach(&wheels, &selector, &pointer_input);
                let weak_wheels = wheels.downgrade();
                let weak_selector = selector.downgrade();
                selector.add_handler_with(
                    InputElement::pointer_wheel_changed_event(),
                    move |_, _| {
                        if let (Some(wheels), Some(selector)) =
                            (weak_wheels.upgrade(), weak_selector.upgrade())
                        {
                            expand(&wheels, &selector);
                        }
                    },
                    RoutingStrategies::TUNNEL,
                    true,
                );
                if matches!(
                    kind,
                    DateTimePickerPanelType::Month | DateTimePickerPanelType::Year
                ) {
                    let weak_wheels = wheels.downgrade();
                    selector
                        .selection_changed(move || {
                            let weak = weak_wheels.clone();
                            // Native staged synchronization follows this event and can
                            // clamp the day. Refresh weekday ink after that transaction.
                            Dispatcher::ui_thread().post_local(
                                move || {
                                    if let Some(wheels) = weak.upgrade() {
                                        refresh_day_labels(&wheels);
                                    }
                                },
                                DispatcherPriority::LOADED,
                            );
                        })
                        .detach();
                }
                let weak_wheels = wheels.downgrade();
                let weak_selector = selector.downgrade();
                selector.add_handler_with(
                    InputElement::key_down_event(),
                    move |_, e| {
                        if matches!(
                            e.key,
                            Key::Up
                                | Key::Down
                                | Key::PageUp
                                | Key::PageDown
                                | Key::Home
                                | Key::End
                        ) {
                            if let (Some(wheels), Some(selector)) =
                                (weak_wheels.upgrade(), weak_selector.upgrade())
                            {
                                expand(&wheels, &selector);
                            }
                        }
                    },
                    RoutingStrategies::TUNNEL,
                    true,
                );
                let scroll = ScrollViewer::new();
                scroll.set_horizontal_scroll_bar_visibility(ScrollBarVisibility::Disabled);
                scroll.set_vertical_scroll_bar_visibility(ScrollBarVisibility::Hidden);
                scroll.set_content(Some(Control::boxed(
                    selector.register_in_name_scope(&**scope),
                )));
                host.children().add(scroll);
                wheels.children().add(host.register_in_name_scope(&**scope));
            }
            for (index, name) in ["PART_FirstSpacer", "PART_SecondSpacer", "PART_ThirdSpacer"]
                .into_iter()
                .enumerate()
            {
                let spacer = Border::new();
                spacer.set_name(Some(name.into()));
                Grid::set_column(&spacer, (index * 2 + 1) as i32);
                spacer.set_width(0.0);
                spacer.set_is_hit_test_visible(false);
                wheels
                    .children()
                    .add(spacer.register_in_name_scope(&**scope));
            }
            page.children().add(wheels.register_in_name_scope(&**scope));

            let bar = Border::new();
            bar.set_name(Some("PhonePickerApplicationBar".into()));
            bar.classes().add("phone-app-bar");
            bar.classes().add("collapsed");
            bar.set_vertical_alignment(V::Bottom);
            bar.set_height(60.0);
            Grid::set_row(&bar, 2);
            let row = Grid::new();
            row.set_column_definitions(
                ColumnDefinitions::parse("44,*,44").expect("picker commands"),
            );
            let commands = UniformGrid::new();
            commands.set_columns(2);
            commands.set_width(152.0);
            commands.set_horizontal_alignment(H::Center);
            Grid::set_column(&commands, 1);
            for (name, label, glyph) in [
                ("PART_AcceptButton", "done", Icon::Check),
                ("PART_DismissButton", "cancel", Icon::Close),
            ] {
                let command = command(name, label, glyph);
                commands
                    .children()
                    .add(command.register_in_name_scope(&**scope));
            }
            row.children().add(commands);
            let more = Button::new();
            more.set_name(Some("PhonePickerMore".into()));
            more.classes().add("phone-more");
            more.set_min_width(44.0);
            more.set_min_height(60.0);
            more.set_vertical_alignment(V::Top);
            AutomationProperties::set_name(&more, Some("more"));
            more.set_content(Some(Control::boxed(glyph(Icon::More, 24.0))));
            let weak = bar.downgrade();
            more.click(move |_, e| {
                e.set_handled(true);
                if let Some(bar) = weak.upgrade() {
                    set_bar_expanded(&bar, !bar.classes().contains("expanded"));
                }
            });
            Grid::set_column(&more, 2);
            row.children().add(more.register_in_name_scope(&**scope));
            bar.set_child(row);
            page.children().add(bar.register_in_name_scope(&**scope));
            page.register_in_name_scope(&**scope).upcast()
        }) as Rc<dyn IControlTemplate>
    };
    if date {
        DATE.with(|slot| slot.get_or_init(create).clone())
    } else {
        TIME.with(|slot| slot.get_or_init(create).clone())
    }
}

pub(super) fn set_bar_expanded(bar: &Border, expanded: bool) {
    bar.classes().set("expanded", expanded);
    bar.classes().set("collapsed", !expanded);
    bar.set_height(if expanded { 79.0 } else { 60.0 });
}

fn command(name: &str, label: &str, kind: Icon) -> Ref<Button> {
    let button = Button::new();
    button.set_name(Some(name.into()));
    button.classes().add("phone-action");
    button.set_theme(Some(shared::button_theme()));
    button.set_min_height(60.0);
    button.set_padding(Thickness::new(2.0, 10.0, 2.0, 10.0));
    button.set_vertical_alignment(V::Top);
    AutomationProperties::set_name(&button, Some(label));
    let content = StackPanel::new();
    content.set_spacing(5.0);
    content.set_horizontal_alignment(H::Center);
    let circle = Border::new();
    circle.classes().add("phone-action-circle");
    circle.set_width(40.0);
    circle.set_height(40.0);
    circle.set_corner_radius(ferroui_base::CornerRadius::uniform(40.0));
    circle.set_border_thickness(Thickness::uniform(2.5));
    circle.set_child(glyph(kind, 26.0 * 5.0 / 6.0));
    content.children().add(circle);
    let caption = TextBlock::new();
    caption.classes().add("phone-action-label");
    caption.set_text(Some(label));
    caption.set_font_size(11.0);
    caption.set_horizontal_alignment(H::Center);
    content.children().add(caption);
    button.set_content(Some(Control::boxed(content)));
    button
}

fn glyph(kind: Icon, size: f64) -> Ref<PathIcon> {
    let glyph = PathIcon::new();
    glyph.classes().add("phone-action-glyph");
    glyph.set_data(Some(
        Geometry::parse(crate::paths::path(kind)).expect("picker command glyph"),
    ));
    glyph.set_width(size);
    glyph.set_height(size);
    glyph.set_horizontal_alignment(H::Center);
    glyph.set_vertical_alignment(V::Center);
    glyph
}

pub(super) fn expand(wheels: &Grid, selected: &DateTimePickerPanel) {
    for visual in wheels.get_visual_descendants() {
        if let Some(panel) = visual.cast::<DateTimePickerPanel>() {
            panel
                .classes()
                .set("expanded", panel.ptr_eq(&selected.to_ref()));
        }
    }
}

fn item_template() -> Rc<dyn IControlTemplate> {
    ITEM.with(|slot| {
        slot.get_or_init(|| {
            let template: Rc<dyn IControlTemplate> = FuncControlTemplate::new(|owner, scope| {
                let item = owner.cast::<ListBoxItem>().expect("native picker item");
                super::input::prepare_item(&item);
                let cell = Border::new();
                cell.set_name(Some("PhonePickerCell".into()));
                cell.set_margin(Thickness::uniform(5.0));
                cell.set_border_thickness(Thickness::uniform(2.5));
                let content = StackPanel::new();
                content.set_name(Some("PhonePickerCellContent".into()));
                content.set_horizontal_alignment(H::Stretch);
                content.set_vertical_alignment(V::Bottom);
                content.set_margin(Thickness::uniform(5.0));
                let primary = TextBlock::new();
                primary.set_name(Some("PhonePickerPrimary".into()));
                primary.set_font_family(FontFamily::new(FONT_FAMILY));
                primary.set_font_weight(FontWeight::SemiBold);
                primary.set_font_size(45.0);
                primary.set_margin(Thickness::new(0.0, -6.667, 0.0, -6.667));
                let fit = Viewbox::new();
                fit.set_stretch(Stretch::Uniform);
                fit.set_stretch_direction(StretchDirection::DownOnly);
                fit.set_horizontal_alignment(H::Left);
                fit.set_child(primary.clone().register_in_name_scope(&**scope));
                content.children().add(fit);
                let secondary = TextBlock::new();
                secondary.set_name(Some("PhonePickerSecondary".into()));
                secondary.set_font_family(FontFamily::new(FONT_FAMILY));
                secondary.set_font_size(20.0 * 5.0 / 6.0);
                secondary.set_margin(Thickness::new(0.0, -1.667, 0.0, -1.667));
                secondary.set_text_wrapping(TextWrapping::Wrap);
                content
                    .children()
                    .add(secondary.clone().register_in_name_scope(&**scope));
                cell.set_child(content.clone().register_in_name_scope(&**scope));
                update_item(&item, &primary, &secondary);
                let scale = item
                    .parent()
                    .and_then(|parent| parent.cast::<DateTimePickerPanel>())
                    .map_or(5.0 / 6.0, |panel| panel.item_height() / 148.0);
                let text_scale = crate::palette().text_scale;
                cell.set_margin(Thickness::uniform(6.0 * scale));
                cell.set_border_thickness(Thickness::uniform(3.0 * scale));
                content.set_margin(Thickness::uniform(6.0 * scale));
                primary.set_font_size(54.0 * scale * text_scale);
                primary.set_margin(Thickness::new(0.0, -8.0 * scale, 0.0, -8.0 * scale));
                secondary.set_font_size(20.0 * scale * text_scale);
                secondary.set_margin(Thickness::new(0.0, -2.0 * scale, 0.0, -2.0 * scale));
                // The native panel recycles its items. Keep one observer for
                // this template's lifetime, remove it on detach, and rebind
                // when a retained popup opens again. An abandoned template
                // must not leave weak callbacks accumulating on its item.
                let subscription = Rc::new(RefCell::new(Some(bind_item_updates(
                    &item, &primary, &secondary,
                ))));
                let loaded_subscription = subscription.clone();
                let weak_item = item.downgrade();
                let weak_primary = primary.downgrade();
                let weak_secondary = secondary.downgrade();
                cell.loaded(move |_, _| {
                    if loaded_subscription.borrow().is_none() {
                        if let (Some(item), Some(primary), Some(secondary)) = (
                            weak_item.upgrade(),
                            weak_primary.upgrade(),
                            weak_secondary.upgrade(),
                        ) {
                            update_item(&item, &primary, &secondary);
                            *loaded_subscription.borrow_mut() =
                                Some(bind_item_updates(&item, &primary, &secondary));
                        }
                    }
                });
                cell.unloaded(move |_, _| {
                    subscription.borrow_mut().take();
                });
                cell.register_in_name_scope(&**scope).upcast()
            });
            template
        })
        .clone()
    })
}

fn bind_item_updates(
    item: &Ref<ListBoxItem>,
    primary: &Ref<TextBlock>,
    secondary: &Ref<TextBlock>,
) -> Subscription {
    let weak_item = item.downgrade();
    let weak_primary = primary.downgrade();
    let weak_secondary = secondary.downgrade();
    Subscription::new(item.property_changed(move |change| {
        if change.property() == Control::tag_property().as_property()
            || change.property() == ContentControl::content_property().as_property()
        {
            if let (Some(item), Some(primary), Some(secondary)) = (
                weak_item.upgrade(),
                weak_primary.upgrade(),
                weak_secondary.upgrade(),
            ) {
                update_item(&item, &primary, &secondary);
            }
        }
    }))
}

fn update_item(item: &ListBoxItem, primary: &TextBlock, secondary: &TextBlock) {
    let tag = item
        .tag()
        .and_then(|tag| (&*tag as &dyn AnyValue).downcast_ref::<i32>().copied());
    let text = item
        .content()
        .and_then(|value| (&*value as &dyn AnyValue).downcast_ref::<String>().cloned())
        .unwrap_or_default();
    let panel = item
        .parent()
        .and_then(|parent| parent.cast::<DateTimePickerPanel>());
    let kind = panel.as_ref().map(|panel| panel.panel_type());
    let (main, detail) = match (kind, tag) {
        (Some(DateTimePickerPanelType::Month), Some(value)) => (value.to_string(), text),
        (Some(DateTimePickerPanelType::Day), Some(value)) => {
            let detail = date_parts(item)
                .filter(|&(year, month)| {
                    (1..=DateTime::days_in_month(year, month)).contains(&value)
                })
                .map(|(year, month)| DateTime::new(year, month, value).to_string_with("dddd"))
                .unwrap_or_else(|| " ".into());
            (value.to_string(), detail)
        }
        (Some(DateTimePickerPanelType::Year), Some(value)) => (value.to_string(), " ".into()),
        _ => (text, " ".into()),
    };
    primary.set_text(Some(&main));
    secondary.set_text(Some(&detail));
    if let Some(panel) = panel {
        set_cell_metrics(item, panel.item_height() / 148.0);
    }
}

pub(super) fn reflow(presenter: &TemplatedControl, width: f64, height: f64) {
    let parts = presenter.get_template_descendants();
    let Some(wheels) = parts
        .iter()
        .find(|part| part.has_name("PART_PickerContainer"))
        .and_then(|part| part.cast::<Grid>())
    else {
        return;
    };
    let columns = wheels.column_definitions().snapshot();
    let visible = columns
        .iter()
        .filter(|column| column.width().grid_unit_type() == GridUnitType::Star)
        .count()
        .max(1);
    // Three 148-unit columns fit the 480-unit source page. Four columns (the
    // native optional-seconds extension) remain bounded by the same viewport.
    let scale = (width.min(480.0) / 480.0)
        .min((width - 24.0).max(1.0) / (visible as f64 * 148.0))
        .min((height - 60.0).max(1.0) / (148.0 + 72.0 * crate::palette().text_scale))
        .max(0.1);
    let pitch = native_pitch(148.0 * scale);
    let scale = pitch / 148.0;
    for column in columns.iter() {
        if column.width().grid_unit_type() == GridUnitType::Star {
            column.set_width(GridLength::STAR);
        }
    }
    wheels.set_width(visible as f64 * pitch);
    for part in parts {
        if let Some(panel) = part.cast::<DateTimePickerPanel>() {
            panel.set_item_height(pitch);
            for child in panel.children().snapshot().iter() {
                if let Some(item) = child.cast::<ListBoxItem>() {
                    set_cell_metrics(&item, scale);
                }
            }
        }
        if part.has_name("PhonePickerTitle") {
            if let Some(title) = part.cast::<TextBlock>() {
                title.set_font_size(22.667 * scale * crate::palette().text_scale);
                title.set_margin(Thickness::new(
                    24.0 * scale,
                    16.0 * scale,
                    24.0 * scale,
                    24.0 * scale,
                ));
            }
        }
    }
}

/// The native panel derives an integer item index by truncating offset/pitch.
/// Repeated keyboard steps with a repeating fraction (for example 98⅔ at a
/// 320-unit viewport) can fall just below an integer and lose a selection.
/// A binary-exact 1/64-unit pitch keeps native offset arithmetic exact without
/// replacing its selection/scroll contract; the maximum geometric adjustment
/// is 1/128 logical unit. Column width uses this same pitch to keep cells square.
fn native_pitch(value: f64) -> f64 {
    (value * 64.0).round() / 64.0
}

fn refresh_day_labels(wheels: &Grid) {
    for visual in wheels.get_visual_descendants() {
        let Some(item) = visual.cast::<ListBoxItem>() else {
            continue;
        };
        if !item.classes().contains("DayItem") {
            continue;
        }
        let parts = item.get_template_descendants();
        let text = |name: &str| {
            parts
                .iter()
                .find(|part| part.has_name(name))
                .and_then(|part| part.cast::<TextBlock>())
        };
        if let (Some(primary), Some(secondary)) =
            (text("PhonePickerPrimary"), text("PhonePickerSecondary"))
        {
            update_item(&item, &primary, &secondary);
        }
    }
}

fn set_cell_metrics(item: &ListBoxItem, scale: f64) {
    let text_scale = crate::palette().text_scale;
    for part in item.get_template_descendants() {
        match part.name().as_deref() {
            Some("PhonePickerCell") => {
                if let Some(cell) = part.cast::<Border>() {
                    cell.set_margin(Thickness::uniform(6.0 * scale));
                    cell.set_border_thickness(Thickness::uniform(3.0 * scale));
                }
            }
            Some("PhonePickerCellContent") => {
                if let Some(content) = part.cast::<StackPanel>() {
                    content.set_margin(Thickness::uniform(6.0 * scale));
                }
            }
            Some("PhonePickerPrimary") => {
                if let Some(text) = part.cast::<TextBlock>() {
                    text.set_font_size(54.0 * scale * text_scale);
                    text.set_margin(Thickness::new(0.0, -8.0 * scale, 0.0, -8.0 * scale));
                }
            }
            Some("PhonePickerSecondary") => {
                if let Some(text) = part.cast::<TextBlock>() {
                    text.set_font_size(20.0 * scale * text_scale);
                    text.set_margin(Thickness::new(0.0, -2.0 * scale, 0.0, -2.0 * scale));
                }
            }
            _ => {}
        }
    }
}

fn date_parts(item: &ListBoxItem) -> Option<(i32, i32)> {
    let presenter = item
        .get_visual_ancestors()
        .find_map(|part| part.cast::<DatePickerPresenter>())?;
    let parts = presenter.get_template_descendants();
    let value = |name: &str| {
        parts
            .iter()
            .find(|part| part.has_name(name))
            .and_then(|part| part.cast::<DateTimePickerPanel>())
            .map(|panel| panel.selected_value())
    };
    let year = value("PART_YearSelector")?;
    let month = value("PART_MonthSelector")?;
    (year > 0 && (1..=12).contains(&month)).then_some((year, month))
}
