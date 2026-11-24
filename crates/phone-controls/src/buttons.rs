use crate::{palette, text};
use ferroui_base::layout::{HorizontalAlignment as H, VerticalAlignment as V};
use ferroui_base::media::{FontWeight, TextWrapping};
use ferroui_view::prelude::*;

pub fn flat(label: &str, on_click: impl Fn() + 'static) -> Element {
    rsx! { Button {class:"phone-flat",min_height:44.0,automation_properties_name:label,
        with:|button|crate::attach_press_tilt(button,||true),
        horizontal_content_alignment:H::Left,vertical_content_alignment:V::Center,
        on_click:move |event| { event.set_handled(true); on_click(); },
        TextBlock {text:label,font_size:22.0*crate::text_scale(),text_wrapping:TextWrapping::Wrap}
    } }
}

pub fn command(label: &str, on_click: impl Fn() + 'static) -> Element {
    command_button(label, on_click).into()
}

/// A text command remains readable without application-bar expansion. Return
/// the native builder so dialogs can name and place it without hiding its label.
pub fn command_button(label: &str, on_click: impl Fn() + 'static) -> El<Button> {
    button()
        .class("phone-button")
        .min_width(0.0)
        .min_height(44.0)
        .horizontal_alignment(H::Stretch)
        .horizontal_content_alignment(H::Stretch)
        .automation_properties_name(label)
        .on_click(move |event| {
            event.set_handled(true);
            on_click();
        })
        .child(rsx! {TextBlock {
            text:label,font_size:22.0*crate::text_scale(),
            text_alignment:ferroui_base::media::TextAlignment::Center,
            text_wrapping:TextWrapping::Wrap,vertical_alignment:V::Center
        }})
}

pub fn pivot(label: &str, selected: bool, on_click: impl Fn() + 'static) -> Element {
    let p = palette();
    rsx! { Button {class:"phone-pivot",min_height:56.0,padding:(0.0,0.0,22.0,0.0),
        with:|button|crate::attach_press_tilt(button,||true),
        automation_properties_name:label,
        on_click:move |event| {event.set_handled(true);on_click();},
        TextBlock {text:label,font_size:crate::FONT_EXTRA_LARGE*crate::text_scale(),font_weight:FontWeight::Light,
            foreground:if selected{p.foreground}else{p.secondary},vertical_alignment:V::Center}
    } }
}

/// Native scrolling keeps large pivot labels reachable on narrow phone surfaces.
/// Recreate only the strip on selection so its newly selected label is revealed;
/// the app's content and its own scroll position retain their separate identity.
pub fn pivot_header(content: Element, selected_label: &str) -> Element {
    let selected = selected_label.to_owned();
    rsx! {ScrollViewer{
        key:format!("pivot-header-{selected_label}"),name:"PivotHeaderScroll",
        horizontal_scroll_bar_visibility:ferroui_controls::primitives::ScrollBarVisibility::Hidden,
        vertical_scroll_bar_visibility:ferroui_controls::primitives::ScrollBarVisibility::Disabled,
        on_loaded:move |event|{
            if let Some(root)=event.source().and_then(|source|source.cast::<Control>()) {
                if let Some(target)=root.find_descendant_of_type_where::<Button>(false,|button|
                    ferroui_controls::automation::AutomationProperties::get_name(button).as_deref()==Some(selected.as_str())) {
                    target.bring_into_view();
                }
            }
        },
        {content}
    }}
}

pub fn input(label: &str) -> El<TextBox> {
    text_box()
        .class("phone-input")
        .min_width(0.0)
        .min_height(44.0)
        .automation_properties_name(label)
        .placeholder_text(label)
}

/// Native draggable switch with Metro geometry, keyboard and automation state.
pub fn toggle(label: &str, value: bool, on_change: impl Fn(bool) + 'static) -> Element {
    let accessible = format!("{label}, {}", if value { "on" } else { "off" });
    rsx! { ToggleSwitch {
        class:if label.is_empty(){"phone-switch phone-switch-compact"}else{"phone-switch"},
        content:label,is_checked:crate::checked_state(Some(value),move |next|on_change(next.unwrap_or(false))),
        off_content:"off",on_content:"on",min_height:if label.is_empty(){44.0}else{66.0},automation_properties_name:accessible,
        on_click:|event|event.set_handled(true)
    } }
}

pub fn list_row(title: &str, detail: &str, on_click: impl Fn() + 'static) -> Element {
    let p = palette();
    rsx! {Button {class:"phone-row",min_height:68.0,padding:(0.0,7.0),automation_properties_name:title,
        with:|button|crate::attach_press_tilt(button,||true),
        on_click:move |event| {event.set_handled(true);on_click();},
        StackPanel {spacing:3.0,
            {text(title,24.0,p.foreground)}
            if !detail.is_empty() {TextBlock{text:detail,font_size:15.0,foreground:p.secondary,text_wrapping:TextWrapping::Wrap}}
        }
    }}
}
