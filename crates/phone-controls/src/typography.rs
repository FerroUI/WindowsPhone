use crate::palette;
use ferroui_base::media::{FontWeight, TextWrapping};
use ferroui_view::prelude::*;

pub fn text(value: impl Into<String>, size: f64, color: &str) -> Element {
    rsx! {TextBlock{text:value.into(),font_size:size,foreground:color}}
}

pub fn paragraph(value: impl Into<String>) -> Element {
    let p = palette();
    rsx! {TextBlock{text:value.into(),font_size:20.0,foreground:p.foreground,text_wrapping:TextWrapping::Wrap,line_height:28.0}}
}

pub fn page_title(app: &str, title: &str) -> Element {
    let p = palette();
    rsx! {StackPanel{spacing:4.0,margin:(0.0,7.0,0.0,16.0),
        TextBlock{text:app.to_uppercase(),font_size:13.0,foreground:p.foreground}
        TextBlock{text:title,font_size:46.0,font_weight:FontWeight::Light,foreground:p.foreground}
    }}
}

pub fn section_title(value: &str) -> Element {
    let p = palette();
    rsx! {TextBlock{text:value,font_size:22.0,foreground:p.accent.as_str(),margin:(0.0,15.0,0.0,8.0)}}
}
