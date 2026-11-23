use crate::checklist;
use ferroui_base::{
    layout::{HorizontalAlignment as H, VerticalAlignment as V},
    media::{TextDecorations, TextWrapping},
    BoxedValue,
};
use ferroui_controls::ItemsSource;
use ferroui_view::prelude::*;
use phone_domain::Note;
use phone_state::PhoneViewModel;
use std::{cell::RefCell, rc::Rc};
#[derive(Clone, PartialEq)]
struct Block {
    id: phone_domain::Id,
    epoch: u64,
    line: usize,
    label: String,
    checked: Option<bool>,
}
struct Rows {
    note: Note,
    epoch: u64,
    source: ItemsSource,
}
impl Rows {
    fn new(note: Note, epoch: u64) -> Self {
        let mut blocks = Vec::new();
        let mut paragraph: Option<String> = None;
        for (line, raw) in note.body.split('\n').enumerate() {
            if let Some(item) = checklist::parse(raw) {
                if let Some(label) = paragraph.take() {
                    blocks.push(Block {
                        id: note.id,
                        epoch,
                        line,
                        label,
                        checked: None,
                    });
                }
                blocks.push(Block {
                    id: note.id,
                    epoch,
                    line,
                    label: item.label.to_owned(),
                    checked: Some(item.checked),
                });
            } else {
                match paragraph.as_mut() {
                    Some(paragraph) => {
                        paragraph.push('\n');
                        paragraph.push_str(raw.trim_end_matches('\r'));
                    }
                    None => paragraph = Some(raw.trim_end_matches('\r').to_owned()),
                }
            }
        }
        if let Some(paragraph) = paragraph {
            blocks.push(Block {
                id: note.id,
                epoch,
                line: 0,
                label: paragraph,
                checked: None,
            });
        }
        let source = ItemsSource::from_items(
            blocks
                .into_iter()
                .map(|block| Some(Rc::new(block) as BoxedValue)),
        );
        Self {
            note,
            epoch,
            source,
        }
    }
}
pub fn view(note: Note, vm: Rc<PhoneViewModel>) -> Element {
    let epoch = vm.document_epoch();
    rerender(move || {
        let cached = use_hook(|| Rc::new(RefCell::new(Rows::new(note.clone(), epoch))));
        let source = {
            let mut cache = cached.borrow_mut();
            if cache.note != note || cache.epoch != epoch {
                *cache = Rows::new(note.clone(), epoch);
            }
            cache.source.clone()
        };
        let template = use_hook({
            let vm = vm.clone();
            move || {
                template(move |block: Block| {
                    if let Some(checked) = block.checked {
                        let vm = vm.clone();
                        let p = phone_controls::palette();
                        let expected = block.label.clone();
                        rsx! {ToggleButton{class:"phone-toggle",is_checked:Some(checked),min_height:48.0,padding:(20.0,8.0),
                            horizontal_alignment:H::Stretch,horizontal_content_alignment:H::Stretch,
                            automation_properties_name:format!("{}: {}",if checked{"completed"}else{"to-do"},block.label),
                            on_click:move |event| {
                                event.set_handled(true);
                                if vm.document_epoch()!=block.epoch{return;}
                                let Some(mut note)=vm.data().note(block.id).cloned() else{return};
                                if note.body.lines().nth(block.line).and_then(checklist::parse).is_none_or(|item|item.label!=expected||item.checked!=checked) {vm.set_status("This to-do item changed. Try again.".into());return;}
                                match checklist::toggle(&note.body,block.line){Ok(body)=>{note.body=body;vm.save_note(note);},Err(error)=>vm.set_status(error)}
                            },
                            Grid{column_definitions:"26,*",column_spacing:12.0,
                                Border{width:24.0,height:24.0,border_thickness:2.0,border_brush:p.foreground,vertical_alignment:V::Top,
                                    if checked {{phone_controls::icon(phone_controls::Icon::Check,18.0,p.foreground)}}}
                                TextBlock{Grid.column:1,text:block.label,font_size:22.0,text_wrapping:TextWrapping::Wrap,
                                    foreground:if checked{p.secondary}else{p.foreground},text_decorations:if checked{Some(TextDecorations::strikethrough())}else{None}}}
                        }}
                    } else {
                        let minimum = if block.label.is_empty() { 31.0 } else { 0.0 };
                        rsx! {TextBlock{min_height:minimum,text:block.label,font_size:22.0,text_wrapping:TextWrapping::Wrap,line_height:31.0,margin:(20.0,8.0)}}
                    }
                })
            }
        });
        rsx! {ListBox{class:"phone-list",items_source:source,item_template:template}}
    })
}
