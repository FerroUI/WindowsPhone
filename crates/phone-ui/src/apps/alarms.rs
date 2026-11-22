use crate::{
    common::{empty, muted, page, scroll, text, title},
    session::Session,
};
use ferroui_base::layout::{HorizontalAlignment as H, Orientation};
use ferroui_view::prelude::*;
use phone_controls::{action, app_bar, toggle, Icon};
use phone_domain::Alarm;
use phone_state::PhoneViewModel;
use std::rc::Rc;

pub fn view(vm: Rc<PhoneViewModel>, ui: Rc<Session>) -> Element {
    let mut alarms = vm.data().alarms.clone();
    alarms.sort_by_key(|alarm| (alarm.hour, alarm.minute));
    let rows = alarms.iter().map(|alarm| {
        let edit_vm=vm.clone();let edit_ui=ui.clone();let edited=alarm.clone();
        let toggle_vm=vm.clone();let id=alarm.id;let delete_vm=vm.clone();let delete_ui=ui.clone();
        rsx! { StackPanel {margin:(20.0,6.0,20.0,20.0),spacing:6.0,
            Grid {column_definitions:"*,Auto",
                Button {class:"phone-flat",padding:0.0,horizontal_content_alignment:H::Left,
                    automation_properties_name:format!("Edit alarm {} {}",alarm.time_text(),alarm.label),
                    on_click:move |_|edit(edit_vm.clone(),edit_ui.clone(),edited.clone()),
                    StackPanel {spacing:3.0,{text(&alarm.time_text(),52.0)}{text(&alarm.label,21.0)}{muted(&alarm.repeat_text(),16.0)}}
                }
                Border {Grid.column:1,{toggle("",alarm.enabled,move |_|toggle_vm.toggle_alarm(id))}}
            }
            Button {class:"phone-flat",content:"delete",font_size:15.0,foreground:"#929292",horizontal_alignment:H::Left,padding:0.0,
                on_click:move |_|{let model=delete_vm.clone();delete_ui.confirm("delete this alarm?",Rc::new(move ||model.delete_alarm(id)));}}
        } }
    }).collect::<Vec<_>>();
    let add_vm = vm.clone();
    let add_ui = ui.clone();
    let body = rsx! { StackPanel { {title("alarms")}
        if rows.is_empty() {{empty("No alarms","Tap add to set an alarm.")}}
        for item in rows {{item}}
        Border {margin:20.0,{muted("Alarms ring while this app is open.",14.0)}}
    } };
    let bar = app_bar(
        vec![action(Icon::Plus, "add", move || {
            edit(
                add_vm.clone(),
                add_ui.clone(),
                Alarm {
                    hour: 7,
                    enabled: true,
                    ..Alarm::default()
                },
            )
        })],
        false,
        Vec::new(),
    );
    page("alarms", scroll(body), bar)
}

fn edit(vm: Rc<PhoneViewModel>, ui: Rc<Session>, alarm: Alarm) {
    let error = ui.form_error;
    let repeat = alarm
        .weekdays
        .iter()
        .map(|day| (day + 1).to_string())
        .collect::<Vec<_>>()
        .join(", ");
    ui.form(
        if alarm.id == 0 {
            "new alarm"
        } else {
            "edit alarm"
        },
        vec![
            ("time (HH:MM)", alarm.time_text()),
            ("label", alarm.label.clone()),
            ("repeat (1 Mon — 7 Sun, blank once)", repeat),
        ],
        Rc::new(move |values| {
            let time = values.first().map(String::as_str).unwrap_or("");
            let Some((hours, minutes)) = time.split_once(':') else {
                error.set("Enter a time in HH:MM format.".into());
                return;
            };
            let (Ok(hour), Ok(minute)) = (hours.trim().parse::<u8>(), minutes.trim().parse::<u8>())
            else {
                error.set("Enter a valid time.".into());
                return;
            };
            if hour > 23 || minute > 59 {
                error.set("Use hours 0–23 and minutes 0–59.".into());
                return;
            }
            let mut weekdays = Vec::new();
            for token in values
                .get(2)
                .map(String::as_str)
                .unwrap_or("")
                .split([',', ' '])
                .filter(|value| !value.is_empty())
            {
                let Ok(day) = token.parse::<u8>() else {
                    error.set("Repeat days must be numbers 1–7 separated by commas.".into());
                    return;
                };
                if !(1..=7).contains(&day) {
                    error.set("Use 1 for Monday through 7 for Sunday.".into());
                    return;
                }
                weekdays.push(day - 1);
            }
            weekdays.sort_unstable();
            weekdays.dedup();
            let label = values.get(1).cloned().unwrap_or_default();
            vm.save_alarm(Alarm {
                id: alarm.id,
                label: if label.trim().is_empty() {
                    "Alarm".into()
                } else {
                    label
                },
                hour,
                minute,
                enabled: alarm.enabled,
                weekdays,
            });
        }),
    );
}
