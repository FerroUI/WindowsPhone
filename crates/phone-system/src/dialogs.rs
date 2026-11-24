use crate::{common, session::Session};
use ferroui_base::layout::{HorizontalAlignment as H, VerticalAlignment as V};
use ferroui_base::media::TextWrapping;
use ferroui_view::prelude::*;
use phone_controls::command_button;
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
        Border{Grid.row:2,background:if vm.data().settings.dark{"#1F1F1F"}else{"#E7E7E7"},min_height:76.0,
            Grid{column_definitions:"*,16,*",margin:(20.0,12.0),vertical_alignment:V::Center,
                {command_button(&dialog.action_label,move||{
                    save_ui.form_error.set(String::new());
                    save(save_ui.fields.iter().take(field_count).map(|field|field.get_untracked()).collect());
                    if save_ui.form_error.get_untracked().is_empty(){save_ui.dialog.set(None);}
                }).name("DialogAction")}
                Border{Grid.column:2,
                    {command_button("cancel",move||cancel_ui.dialog.set(None)).name("DialogCancel")}
                }
            }
        }
    }}
}

#[cfg(test)]
mod tests {
    use super::*;
    use ferroui_base::interactivity::RoutedEventArgs;
    use ferroui_controls::automation::AutomationProperties;
    use phone_domain::PhoneData;
    use phone_sdk::AppRegistry;
    use std::cell::Cell;

    fn fixture() -> (Rc<PhoneViewModel>, Rc<Session>) {
        let vm = PhoneViewModel::new(PhoneData::default(), Rc::new(|_| {}));
        let registry = Rc::new(AppRegistry::new(Vec::new()).unwrap());
        (vm, Rc::new(Session::new(0, registry)))
    }

    fn button(
        root: &ferroui_base::Ref<Control>,
        name: &str,
        label: &str,
    ) -> ferroui_base::Ref<Button> {
        let button = root
            .get_logical_descendants()
            .filter_map(|element| element.cast::<Button>())
            .find(|element| element.has_name(name))
            .expect("mounted dialog command");
        assert_eq!(
            AutomationProperties::get_name(&button).as_deref(),
            Some(label)
        );
        let caption = button
            .get_logical_descendants()
            .filter_map(|element| element.cast::<TextBlock>())
            .find(|element| element.text().as_deref() == Some(label))
            .expect("the command's intent is visible text, not an inferred icon");
        assert!(caption.is_visible());
        button
    }

    #[test]
    fn generic_confirmation_labels_and_native_buttons_preserve_cancel_and_confirm() {
        let (vm, ui) = fixture();
        let calls = Rc::new(Cell::new(0));
        let called = calls.clone();
        ui.confirm(
            "remove selected items?",
            Rc::new(move || called.set(called.get() + 1)),
        );
        let (root, owner) = view(vm.clone(), ui.clone()).mount_root();
        let _confirm = button(&root, "DialogAction", "confirm");
        button(&root, "DialogCancel", "cancel")
            .raise_event(&RoutedEventArgs::with_event(Button::click_event()));
        assert!(ui.dialog.get_untracked().is_none());
        assert_eq!(calls.get(), 0, "cancel never runs the confirmed operation");
        owner.dispose();

        let called = calls.clone();
        ui.confirm(
            "remove selected items?",
            Rc::new(move || called.set(called.get() + 1)),
        );
        let (root, owner) = view(vm, ui.clone()).mount_root();
        button(&root, "DialogAction", "confirm")
            .raise_event(&RoutedEventArgs::with_event(Button::click_event()));
        assert_eq!(
            calls.get(),
            1,
            "explicit confirmation invokes the operation once"
        );
        assert!(ui.dialog.get_untracked().is_none());
        owner.dispose();
    }

    #[test]
    fn explicit_command_keeps_its_caption_and_failed_validation_keeps_the_dialog() {
        let (vm, ui) = fixture();
        let calls = Rc::new(Cell::new(0));
        let called = calls.clone();
        let error = ui.form_error;
        ui.form_with_action(
            "replace item?",
            vec![("name", "invalid".into())],
            "replace",
            Rc::new(move |values| {
                called.set(called.get() + 1);
                if values != ["valid"] {
                    error.set("Choose a valid name.".into());
                }
            }),
        );
        let (root, owner) = view(vm, ui.clone()).mount_root();
        let replace = button(&root, "DialogAction", "replace");
        replace.raise_event(&RoutedEventArgs::with_event(Button::click_event()));
        assert_eq!(calls.get(), 1);
        assert!(ui.dialog.get_untracked().is_some());
        assert_eq!(ui.form_error.get_untracked(), "Choose a valid name.");
        ui.fields[0].set("valid".into());
        replace.raise_event(&RoutedEventArgs::with_event(Button::click_event()));
        assert_eq!(calls.get(), 2);
        assert!(ui.form_error.get_untracked().is_empty());
        assert!(ui.dialog.get_untracked().is_none());
        owner.dispose();
    }
}
