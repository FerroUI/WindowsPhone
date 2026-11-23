//! Office hub and native mobile document editors.
pub(crate) mod commands;
mod excel;
mod hub;
mod powerpoint;
mod word;

use crate::{common::*, session::Session};
use ferroui_base::{
    layout::{HorizontalAlignment as H, VerticalAlignment as V},
    media::FontWeight,
};
use ferroui_view::prelude::*;
use phone_controls::{Icon, icon};
use phone_domain::{Id, OfficeDocument, Route};
use phone_state::PhoneViewModel;
use std::rc::Rc;

pub struct OfficeSession {
    pub paragraph: Signal<usize>,
    pub sheet: Signal<usize>,
    pub row_page: Signal<usize>,
    pub col_page: Signal<usize>,
    pub slide: Signal<usize>,
    pub show_new: Signal<bool>,
    pub show_find: Signal<bool>,
    pub formatting: Signal<bool>,
    pub excel_selection: Signal<Option<(Id, usize, String)>>,
}
impl Default for OfficeSession {
    fn default() -> Self {
        Self {
            paragraph: signal(0),
            sheet: signal(0),
            row_page: signal(0),
            col_page: signal(0),
            slide: signal(0),
            show_new: signal(false),
            show_find: signal(false),
            formatting: signal(false),
            excel_selection: signal(None),
        }
    }
}

pub fn view(vm: Rc<PhoneViewModel>, ui: Rc<Session>) -> Element {
    if let Route::Document(id) = vm.route() {
        let document = vm.data().document(id).cloned();
        if let Some(document) = document {
            return match document.kind() {
                phone_domain::DocumentKind::Word => word::view(vm, ui, document),
                phone_domain::DocumentKind::Excel => excel::view(vm, ui, document),
                phone_domain::DocumentKind::PowerPoint => powerpoint::view(vm, ui, document),
            };
        }
    }
    hub::view(vm, ui)
}

pub(crate) fn document_row(
    document: &OfficeDocument,
    detail: &str,
    on_click: impl Fn() + 'static,
) -> Element {
    let kind = document.kind();
    rsx! {Button{class:"phone-flat",horizontal_alignment:H::Stretch,horizontal_content_alignment:H::Stretch,
        automation_properties_name:document.file_name(),automation_properties_help_text:detail,
        padding:(20.0,10.0),min_height:76.0,on_click:move|_|on_click(),
        Grid{column_definitions:"46,*",column_spacing:14.0,
            Border{width:44.0,height:52.0,background:kind.color(),vertical_alignment:V::Center,
                TextBlock{text:kind.letter(),font_size:31.0,font_weight:FontWeight::SemiBold,foreground:"White",horizontal_alignment:H::Center,vertical_alignment:V::Center}}
            StackPanel{Grid.column:1,spacing:4.0,vertical_alignment:V::Center,
                {text(&document.name,25.0)}{muted(detail,14.0)}}
        }
    }}
}
pub(crate) fn folder_row(name: &str, detail: &str, on_click: impl Fn() + 'static) -> Element {
    rsx! {Button{class:"phone-flat",horizontal_alignment:H::Stretch,horizontal_content_alignment:H::Stretch,
        automation_properties_name:name,automation_properties_help_text:detail,padding:(20.0,10.0),min_height:76.0,on_click:move|_|on_click(),
        Grid{column_definitions:"46,*",column_spacing:14.0,
            Border{width:44.0,height:48.0,background:"#F0A000",{icon(Icon::Folder,28.0,"White")}}
            StackPanel{Grid.column:1,spacing:4.0,{text(name,25.0)}{muted(detail,14.0)}}}
    }}
}
pub(crate) fn date_text(timestamp: i64) -> String {
    let (y, m, d) = phone_domain::date_parts(timestamp);
    format!("{d:02}/{m:02}/{y}")
}
pub(crate) fn compatibility(document: &OfficeDocument) -> Element {
    if document.import_notice.is_empty() {
        Element::Empty
    } else {
        rsx! {Border{margin:(18.0,8.0),padding:10.0,background:"#FFF4CE",
        TextBlock{text:document.import_notice.clone(),font_size:13.0,foreground:"#664D00",text_wrapping:ferroui_base::media::TextWrapping::Wrap}}}
    }
}
