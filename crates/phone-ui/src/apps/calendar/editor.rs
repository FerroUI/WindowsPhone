use super::{commands, pickers, session::CalendarPicker};
use crate::{common::*, session::Session};
use ferroui_base::{
    layout::HorizontalAlignment as H, media::TextWrapping, utilities::DateTimeOffset,
};
use ferroui_view::prelude::*;
use phone_controls::{Icon, action, app_bar, flat, toggle};
use phone_domain::CalendarRepeat;
use phone_state::PhoneViewModel;
use std::rc::Rc;

pub fn view(vm: Rc<PhoneViewModel>, ui: Rc<Session>) -> Element {
    let state = &ui.calendar;
    let target = state.editor.get().expect("calendar editor state");
    let save_vm = vm.clone();
    let save_ui = ui.clone();
    let cancel = ui.clone();
    let all_day = state.all_day.get();
    let all_day_ui = ui.clone();
    let show_more = state.more.get();
    let more = state.more;
    let one_occurrence = target.occurrence_date.is_some();
    let repeat = state.repeat.get();
    let ends = state.repeat_ends.get();
    let end_ui = ui.clone();
    let body = rsx! {StackPanel{margin:(20.0,8.0),spacing:17.0,
        {text(if target.original.id==0{"new appointment"}else{"edit appointment"},37.0)}
        if one_occurrence {{muted("this appointment",16.0)}}
        StackPanel{spacing:6.0,{muted("subject",16.0)}
            TextBox{name:"CalendarSubject",key:"calendar-subject",class:"phone-input",text:state.subject.two_way(),
                automation_properties_name:"appointment subject",font_size:24.0,min_height:48.0,horizontal_alignment:H::Stretch}}
        StackPanel{spacing:6.0,{muted("location",16.0)}
            TextBox{name:"CalendarLocation",key:"calendar-location",class:"phone-input",text:state.location.two_way(),
                automation_properties_name:"appointment location",font_size:21.0,min_height:48.0,horizontal_alignment:H::Stretch}}
        StackPanel{spacing:6.0,{muted("when",16.0)}
            DatePicker{name:"CalendarDate",class:"phone-picker",selected_date:state.date_value.two_way(),
                min_year:DateTimeOffset::MIN_VALUE,max_year:DateTimeOffset::MAX_VALUE,
                automation_properties_name:"appointment date",horizontal_alignment:H::Stretch}}
        {toggle("all day",all_day,move|value|{all_day_ui.calendar.all_day.set(value);
            if value {all_day_ui.calendar.duration.set(1440);}else if all_day_ui.calendar.duration.get_untracked()%1440==0{all_day_ui.calendar.duration.set(60);}})}
        if !all_day {StackPanel{spacing:6.0,{muted("time",16.0)}
            TimePicker{name:"CalendarTime",class:"phone-picker",selected_time:state.time_value.two_way(),minute_increment:1,
                clock_identifier:if vm.data().settings.time_24h{"24HourClock"}else{"12HourClock"},
                automation_properties_name:"appointment time",horizontal_alignment:H::Stretch}}}
        {choice(&ui,"how long",&pickers::duration_label(state.duration.get(),all_day),CalendarPicker::Duration)}
        if !show_more {{flat("more details",move||more.set(true))}}
        if show_more {
            StackPanel{spacing:17.0,
                {choice(&ui,"reminder",&pickers::reminder_label(state.reminder.get()),CalendarPicker::Reminder)}
                if !one_occurrence {
                    {choice(&ui,"occurs",repeat.label(),CalendarPicker::Repeat)}
                    if repeat!=CalendarRepeat::Never {
                        {toggle("end repeating",ends,move|value|end_ui.calendar.repeat_ends.set(value))}
                        if ends {DatePicker{name:"CalendarRepeatEnd",class:"phone-picker",selected_date:state.until.two_way(),
                            min_year:DateTimeOffset::MIN_VALUE,max_year:DateTimeOffset::MAX_VALUE,
                            automation_properties_name:"repeat end date",horizontal_alignment:H::Stretch}}
                    }
                }
                {choice(&ui,"status",state.status.get().label(),CalendarPicker::Status)}
                StackPanel{spacing:6.0,{muted("notes",16.0)}
                    TextBox{name:"CalendarNotes",key:"calendar-notes",class:"phone-input",text:state.notes.two_way(),
                        automation_properties_name:"appointment notes",font_size:20.0,min_height:145.0,
                        accepts_return:true,text_wrapping:TextWrapping::Wrap,horizontal_alignment:H::Stretch}}
            }
        }
        TextBlock{name:"CalendarError",text:state.error.get(),font_size:16.0,foreground:accent(&vm).as_str(),text_wrapping:TextWrapping::Wrap}
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
                    cancel.calendar.editor.set(None);
                    cancel.calendar.error.set(String::new());
                }),
            ],
            false,
            Vec::new(),
        ),
    )
}

fn choice(ui: &Rc<Session>, label: &str, value: &str, picker: CalendarPicker) -> Element {
    let open = ui.clone();
    rsx! {StackPanel{spacing:5.0,{muted(label,16.0)}
        Button{class:"phone-button",name:format!("CalendarChoice{}",label.replace(' ',"")),
            automation_properties_name:format!("{label}, {value}"),padding:(12.0,8.0),min_height:48.0,
            horizontal_alignment:H::Stretch,horizontal_content_alignment:H::Stretch,
            on_click:move|_|open.calendar.picker.set(Some(picker)),
            Grid{column_definitions:"*,Auto",TextBlock{text:value,font_size:22.0,text_wrapping:TextWrapping::Wrap}
                Border{Grid.column:1,margin:(10.0,0.0,0.0,0.0),{phone_controls::icon(Icon::ChevronDown,14.0,phone_controls::palette().foreground)}}}}
    }}
}
