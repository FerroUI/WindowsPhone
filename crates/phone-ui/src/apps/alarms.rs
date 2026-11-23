pub(crate) mod editor;
use crate::{
    common::{empty, muted, page, scroll, text, title},
    session::Session,
};
use ferroui_base::layout::HorizontalAlignment as H;
use ferroui_view::prelude::*;
use phone_controls::{action, app_bar, toggle, Icon};
use phone_domain::Alarm;
use phone_state::PhoneViewModel;
use std::rc::Rc;

pub fn view(vm: Rc<PhoneViewModel>, ui: Rc<Session>) -> Element {
    if ui.alarms.editor.draft.get().is_some() {
        return editor::view(vm, ui);
    }
    let mut alarms = vm.data().alarms.clone();
    alarms.sort_by_key(|alarm| (alarm.hour, alarm.minute, alarm.id));
    let rows = alarms.iter().map(|alarm| {
        let edit_ui=ui.clone();let edited=alarm.clone();let toggle_vm=vm.clone();let toggle_ui=ui.clone();let id=alarm.id;
        let time=phone_domain::time_text(i64::from(alarm.hour)*3600+i64::from(alarm.minute)*60,vm.data().settings.time_24h);
        rsx! {Grid{column_definitions:"*,78",column_spacing:8.0,margin:(20.0,4.0,20.0,22.0),
            Button{class:"phone-flat",padding:0.0,horizontal_content_alignment:H::Left,
                automation_properties_name:format!("Edit alarm {} {}",alarm.time_text(),alarm.label),
                on_click:move |_|edit_ui.alarms.editor.open(edited.clone()),
                StackPanel{spacing:2.0,opacity:if alarm.enabled{1.0}else{0.52},
                    {text(&time,48.0)}{text(&alarm.label,22.0)}{muted(&alarm.repeat_text(),16.0)}}}
            Border{Grid.column:1,{toggle("",alarm.enabled,move |_|{
                toggle_ui.alarms.cancel_alarm(&toggle_vm,id);
                toggle_vm.toggle_alarm(id);
            })}}
        }}
    }).collect::<Vec<_>>();
    let add = ui.clone();
    page(
        "alarms",
        scroll(rsx! {StackPanel{{title("alarms")}
            if rows.is_empty(){{empty("No alarms","Tap add to set an alarm.")}}
            for item in rows{{item}}
            Border{margin:20.0,{muted("Alarms and reminders ring while this app is open.",14.0)}}
        }}),
        app_bar(
            vec![action(Icon::Plus, "add", move || {
                add.alarms.editor.open(Alarm {
                    hour: 7,
                    enabled: true,
                    ..Alarm::default()
                })
            })],
            false,
            Vec::new(),
        ),
    )
}
