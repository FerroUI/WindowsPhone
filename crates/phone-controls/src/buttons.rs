use crate::{Icon, icon, palette, text};
use ferroui_base::layout::{HorizontalAlignment as H, Orientation, VerticalAlignment as V};
use ferroui_base::media::{FontWeight, TextWrapping};
use ferroui_view::prelude::*;

/// Circular Windows Phone application-bar icon, with a label on expanded bars.
pub fn action(kind: Icon, label: &str, on_click: impl Fn() + 'static) -> Element {
    let p = palette();
    rsx! { Button {
        class:"phone-action", min_width:64.0, min_height:54.0, padding:(6.0,5.0),
        automation_properties_name:label, tool_tip_tip:label,
        on_click:move |event| { event.set_handled(true); on_click(); },
        StackPanel { spacing:5.0, horizontal_alignment:H::Center,
            Border { width:38.0,height:38.0,corner_radius:38.0,
                border_thickness:2.0,border_brush:p.foreground,
                {icon(kind,19.0,p.foreground)}
            }
            TextBlock {class:"phone-action-label",text:label,font_size:11.0,
                horizontal_alignment:H::Center,foreground:p.foreground}
        }
    } }
}

pub fn flat(label: &str, on_click: impl Fn() + 'static) -> Element {
    rsx! { Button {class:"phone-flat",min_height:44.0,automation_properties_name:label,
        horizontal_content_alignment:H::Left,vertical_content_alignment:V::Center,
        on_click:move |event| { event.set_handled(true); on_click(); },
        TextBlock {text:label,font_size:22.0*crate::text_scale(),text_wrapping:TextWrapping::Wrap}
    } }
}

pub fn command(label: &str, on_click: impl Fn() + 'static) -> Element {
    rsx! { Button {class:"phone-button",content:label,automation_properties_name:label,
        on_click:move |event| { event.set_handled(true); on_click(); }
    } }
}

pub fn pivot(label: &str, selected: bool, on_click: impl Fn() + 'static) -> Element {
    let p = palette();
    rsx! { Button {class:"phone-pivot",min_height:56.0,padding:(0.0,0.0,22.0,0.0),
        automation_properties_name:label,
        on_click:move |event| {event.set_handled(true);on_click();},
        TextBlock {text:label,font_size:36.0,font_weight:FontWeight::Light,
            foreground:if selected{p.foreground}else{p.secondary},vertical_alignment:V::Center}
    } }
}

pub fn input(label: &str) -> El<TextBox> {
    text_box()
        .class("phone-input")
        .min_width(0.0)
        .min_height(44.0)
        .automation_properties_name(label)
        .placeholder_text(label)
}

/// Touch-friendly switch matching the square Windows Phone switch treatment.
pub fn toggle(label: &str, value: bool, on_change: impl Fn(bool) + 'static) -> Element {
    let p = palette();
    let accessible = format!("{label}, {}", if value { "on" } else { "off" });
    rsx! { ToggleButton {class:"phone-toggle",is_checked:Some(value),min_height:66.0,automation_properties_name:accessible,
        on_click:move |event| {event.set_handled(true);on_change(!value);},
        Grid {column_definitions:"*,70",column_spacing:18.0,
            TextBlock {text:label,font_size:22.0*crate::text_scale(),text_wrapping:TextWrapping::Wrap,vertical_alignment:V::Center}
            Border {Grid.column:1,width:60.0,height:26.0,border_brush:p.foreground,border_thickness:2.0,padding:3.0,vertical_alignment:V::Center,
                Grid{background:if value{p.accent.as_str()}else{"Transparent"},
                    Border {width:15.0,background:p.foreground,horizontal_alignment:H::Left,
                        transitions:crate::motion::press_transitions(),
                        render_transform:crate::motion::switch_position(value)}
                }
            }
        }
    } }
}

/// Supply up to four icon actions, including the caller's ellipsis action.
pub fn app_bar(actions: Vec<Element>, expanded: bool, secondary: Vec<Element>) -> Element {
    let p = palette();
    let mut icons = stack_panel()
        .orientation(Orientation::Horizontal)
        .horizontal_alignment(H::Center);
    for action in actions {
        icons = icons.child(action);
    }
    let mut body = stack_panel().child(icons);
    if expanded {
        let mut menu = stack_panel()
            .key("app-bar-menu")
            .spacing(5.0)
            .margin((20.0, 8.0, 20.0, 18.0))
            .on_loaded(|event| {
                if let Some(control) = event.source().and_then(|source| source.cast::<Control>()) {
                    crate::reveal(&control);
                }
            });
        for action in secondary {
            menu = menu.child(action);
        }
        body = body.child(menu);
    }
    border()
        .class("phone-app-bar")
        .class(if expanded { "expanded" } else { "collapsed" })
        .background(p.surface)
        .min_height(58.0)
        .child(body)
        .into()
}

pub fn list_row(title: &str, detail: &str, on_click: impl Fn() + 'static) -> Element {
    let p = palette();
    rsx! {Button {class:"phone-row",min_height:68.0,padding:(0.0,7.0),automation_properties_name:title,
        on_click:move |event| {event.set_handled(true);on_click();},
        StackPanel {spacing:3.0,
            {text(title,24.0,p.foreground)}
            if !detail.is_empty() {TextBlock{text:detail,font_size:15.0,foreground:p.secondary,text_wrapping:TextWrapping::Wrap}}
        }
    }}
}
