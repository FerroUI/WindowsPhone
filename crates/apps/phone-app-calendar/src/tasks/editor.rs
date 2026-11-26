use super::{
    commands,
    state::{Choice, State},
};
use ferroui_base::{
    layout::HorizontalAlignment as H, media::TextWrapping, utilities::DateTimeOffset,
};
use ferroui_view::prelude::*;
use phone_controls::{action, app_bar, toggle, Icon};
use phone_sdk::{common::*, session::Session};
use phone_state::PhoneViewModel;
use std::rc::Rc;

pub fn view(vm: Rc<PhoneViewModel>, ui: Rc<Session>) -> Element {
    let state = ui.state::<State>();
    let edit = state.editor.get().expect("task editor state");
    let save_vm = vm.clone();
    let save_ui = ui.clone();
    let cancel_vm = vm.clone();
    let cancel_ui = ui.clone();
    let due_enabled = state.due_enabled.get();
    let due_toggle = state.due_enabled;
    let reminder_enabled = state.reminder_enabled.get();
    let reminder_toggle = state.reminder_enabled;
    let priority = state.priority.get();
    let choose_priority = ui.clone();
    let body = rsx! {StackPanel{name:"CalendarTaskEditor",margin:(20.0,8.0),spacing:16.0,
        {text(if edit.original.is_some(){"edit task"}else{"new task"},37.0)}
        StackPanel{spacing:5.0,{muted("subject",16.0)}
            TextBox{name:"CalendarTaskSubject",key:"task-subject",class:"phone-input",text:state.subject.two_way(),
                automation_properties_name:"task subject",font_size:24.0,min_height:48.0,horizontal_alignment:H::Stretch}}
        {toggle("due date",due_enabled,move|value|due_toggle.set(value))}
        if due_enabled {DatePicker{name:"CalendarTaskDue",class:"phone-picker",selected_date:state.due.two_way(),
            with:|picker|phone_controls::attach_picker_focus(picker),min_year:DateTimeOffset::MIN_VALUE,max_year:DateTimeOffset::MAX_VALUE,
            automation_properties_name:"task due date",horizontal_alignment:H::Stretch}}
        StackPanel{spacing:5.0,{muted("priority",16.0)}
            Button{name:"CalendarTaskPriority",class:"phone-button",content:priority.label(),min_height:48.0,horizontal_alignment:H::Stretch,
                automation_properties_name:format!("task priority, {}",priority.label()),on_click:move |_|choose_priority.state::<State>().choice.set(Some(Choice::Priority))}}
        {toggle("reminder",reminder_enabled,move|value|reminder_toggle.set(value))}
        if reminder_enabled {StackPanel{spacing:12.0,
            DatePicker{name:"CalendarTaskReminderDate",class:"phone-picker",selected_date:state.reminder_date.two_way(),
                with:|picker|phone_controls::attach_picker_focus(picker),min_year:DateTimeOffset::MIN_VALUE,max_year:DateTimeOffset::MAX_VALUE,
                automation_properties_name:"task reminder date",horizontal_alignment:H::Stretch}
            TimePicker{name:"CalendarTaskReminderTime",class:"phone-picker",selected_time:state.reminder_time.two_way(),minute_increment:1,
                with:|picker|phone_controls::attach_picker_focus(picker),clock_identifier:if vm.data().settings.time_24h{"24HourClock"}else{"12HourClock"},
                automation_properties_name:"task reminder time",horizontal_alignment:H::Stretch}
            {muted("Reminders appear while Windows Phone is running in the foreground.",15.0)}
        }}
        StackPanel{spacing:5.0,{muted("notes",16.0)}
            TextBox{name:"CalendarTaskNotes",key:"task-notes",class:"phone-input",text:state.notes.two_way(),
                automation_properties_name:"task notes",font_size:20.0,min_height:145.0,accepts_return:true,text_wrapping:TextWrapping::Wrap,horizontal_alignment:H::Stretch}}
        TextBlock{name:"CalendarTaskError",text:state.error.get(),font_size:16.0,foreground:accent(&vm).as_str(),text_wrapping:TextWrapping::Wrap}
    }};
    page(
        "calendar",
        scroll(body),
        app_bar(
            vec![
                action(Icon::Save, "save", move || {
                    commands::save(&save_vm, &save_ui)
                }),
                action(Icon::Close, "cancel", move || {
                    super::back(&cancel_vm, &cancel_ui);
                }),
            ],
            false,
            Vec::new(),
        ),
    )
}
