use crate::{common, session::Session};
use ferroui_base::layout::{HorizontalAlignment as H, VerticalAlignment as V};
use ferroui_base::media::TextWrapping;
use ferroui_view::prelude::*;
use phone_controls::{action, Icon};
use phone_state::PhoneViewModel;
use std::rc::Rc;

pub fn view(vm: Rc<PhoneViewModel>, ui: Rc<Session>) -> Element {
    let Some(dialog) = ui.dialog.get() else {
        return rsx! {Border{}};
    };
    let mut fields = stack_panel().spacing(18.0).margin((20.0, 12.0));
    for (index, label) in dialog.labels.iter().enumerate() {
        let multiline = matches!(
            label.to_ascii_lowercase().as_str(),
            "note" | "notes" | "body" | "details" | "message" | "description"
        );
        fields=fields.child(rsx!{StackPanel{spacing:7.0,{common::muted(label,16.0)}
            TextBox{name:format!("FormField{index}"),class:"phone-input",text:ui.fields[index].two_way(),
                automation_properties_name:label.clone(),font_size:21.0,min_height:if multiline{150.0}else{48.0},
                accepts_return:multiline,text_wrapping:TextWrapping::Wrap,horizontal_alignment:H::Stretch}
        }});
    }
    let save_ui = ui.clone();
    let cancel_ui = ui.clone();
    let save = dialog.save.clone();
    let field_count = dialog.labels.len();
    rsx! {Grid{row_definitions:"Auto,*,Auto",background:common::bg(&vm).as_str(),
        TextBlock{text:dialog.title,font_size:43.0,font_weight:ferroui_base::media::FontWeight::Light,margin:(18.0,28.0,12.0,12.0),text_wrapping:TextWrapping::Wrap},
        Border{Grid.row:1,{common::scroll(rsx!{StackPanel{{fields}{common::muted(&ui.form_error.get(),16.0)}}})}},
        Border{Grid.row:2,background:if vm.data().settings.dark{"#1F1F1F"}else{"#E7E7E7"},height:76.0,
            StackPanel{orientation:ferroui_base::layout::Orientation::Horizontal,horizontal_alignment:H::Center,vertical_alignment:V::Center,spacing:38.0,
                {action(if dialog.action_label.starts_with("open "){Icon::Send}else{Icon::Save},&dialog.action_label,move||{
                    save_ui.form_error.set(String::new());
                    save(save_ui.fields.iter().take(field_count).map(|field|field.get_untracked()).collect());
                    if save_ui.form_error.get_untracked().is_empty(){save_ui.dialog.set(None);}
                })}
                {action(Icon::Close,"cancel",move||cancel_ui.dialog.set(None))}
            }
        }
    }}
}
