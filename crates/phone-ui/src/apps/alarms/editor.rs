use crate::{common, session::Session};
use ferroui_base::{animation::SignedDuration, layout::HorizontalAlignment as H};
use ferroui_view::prelude::*;
use phone_controls::{action, app_bar, Icon};
use phone_domain::{Alarm, ALARM_SNOOZE_MINUTES};
use phone_state::PhoneViewModel;
use std::rc::Rc;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Picker {
    Main,
    Repeat,
    Snooze,
}

pub struct AlarmEditor {
    pub draft: Signal<Option<Alarm>>,
    pub time: Signal<Option<SignedDuration>>,
    pub label: Signal<String>,
    pub weekdays: Signal<Vec<u8>>,
    pub snooze: Signal<u8>,
    pub page: Signal<Picker>,
    error: Signal<String>,
}

impl AlarmEditor {
    pub fn new() -> Self {
        Self {
            draft: signal(None),
            time: signal(None),
            label: signal(String::new()),
            weekdays: signal(Vec::new()),
            snooze: signal(5),
            page: signal(Picker::Main),
            error: signal(String::new()),
        }
    }
    pub fn open(&self, alarm: Alarm) {
        self.time.set(Some(SignedDuration::from_minutes(
            f64::from(alarm.hour) * 60.0 + f64::from(alarm.minute),
        )));
        self.label.set(alarm.label.clone());
        self.weekdays.set(alarm.weekdays.clone());
        self.snooze.set(alarm.snooze_minutes);
        self.page.set(Picker::Main);
        self.error.set(String::new());
        self.draft.set(Some(alarm));
    }
    pub fn close(&self) {
        self.draft.set(None);
        self.page.set(Picker::Main);
        self.error.set(String::new());
    }
    pub fn dismiss(&self) -> bool {
        if self.draft.get_untracked().is_none() {
            return false;
        }
        if self.page.get_untracked() != Picker::Main {
            self.page.set(Picker::Main);
        } else {
            self.close();
        }
        true
    }
    fn save(&self, vm: &PhoneViewModel) -> Option<phone_domain::Id> {
        let Some(mut alarm) = self.draft.get_untracked() else {
            return None;
        };
        let Some(minutes) = self
            .time
            .get_untracked()
            .map(|time| time.total_minutes().round())
            .filter(|minutes| minutes.is_finite() && (0.0..1440.0).contains(minutes))
        else {
            self.error.set("Choose a valid time.".into());
            return None;
        };
        alarm.hour = (minutes as u16 / 60) as u8;
        alarm.minute = (minutes as u16 % 60) as u8;
        alarm.label = self.label.get_untracked().trim().to_owned();
        if alarm.label.is_empty() {
            alarm.label = "Alarm".into();
        }
        alarm.weekdays = self.weekdays.get_untracked();
        alarm.snooze_minutes = self.snooze.get_untracked();
        vm.clear_status();
        let id = alarm.id;
        vm.save_alarm(alarm);
        if vm.status().is_empty() {
            self.close();
            Some(id)
        } else {
            self.error.set(vm.status());
            None
        }
    }
}

pub fn view(vm: Rc<PhoneViewModel>, ui: Rc<Session>) -> Element {
    let editor = &ui.alarms.editor;
    let Some(alarm) = editor.draft.get() else {
        return rsx! {Border{}};
    };
    let page = editor.page.get();
    let heading = match page {
        Picker::Main => {
            if alarm.id == 0 {
                "new alarm"
            } else {
                "edit alarm"
            }
        }
        Picker::Repeat => "repeats",
        Picker::Snooze => "snooze",
    };
    let body = match page {
        Picker::Main => {
            let repeat = ui.clone();
            let snooze = ui.clone();
            let mut preview = alarm.clone();
            preview.weekdays = editor.weekdays.get();
            rsx! {StackPanel{margin:(20.0,0.0,20.0,20.0),spacing:18.0,
                StackPanel{spacing:5.0,{common::muted("time",16.0)}
                    TimePicker{name:"AlarmTimePicker",class:"phone-picker",horizontal_alignment:H::Stretch,
                        with:|picker|phone_controls::attach_picker_focus(picker),
                        selected_time:editor.time.two_way(),minute_increment:1,
                        clock_identifier:if vm.data().settings.time_24h{"24HourClock"}else{"12HourClock"},automation_properties_name:"Alarm time"}
                }
                StackPanel{spacing:5.0,{common::muted("name",16.0)}
                    TextBox{name:"AlarmName",class:"phone-input",text:editor.label.two_way(),font_size:23.0,automation_properties_name:"Alarm name"}}
                Button{class:"phone-flat",padding:0.0,horizontal_content_alignment:H::Stretch,automation_properties_name:"Choose alarm repeat days",
                    on_click:move |_|repeat.alarms.editor.page.set(Picker::Repeat),
                    StackPanel{spacing:4.0,{common::muted("repeats",16.0)}{common::text(&preview.repeat_text().to_lowercase(),25.0)}}}
                Button{class:"phone-flat",padding:0.0,horizontal_content_alignment:H::Stretch,automation_properties_name:"Choose alarm snooze time",
                    on_click:move |_|snooze.alarms.editor.page.set(Picker::Snooze),
                    StackPanel{spacing:4.0,{common::muted("snooze time",16.0)}{common::text(&format!("{} minutes",editor.snooze.get()),25.0)}}}
                {common::muted(&editor.error.get(),16.0)}
            }}
        }
        Picker::Repeat => {
            let selected = editor.weekdays.get();
            let days = [
                "Monday",
                "Tuesday",
                "Wednesday",
                "Thursday",
                "Friday",
                "Saturday",
                "Sunday",
            ];
            let rows = days.into_iter().enumerate().map(|(index,name)| {
                let change=ui.clone();let checked=selected.contains(&(index as u8));
                rsx!{Button{class:"phone-flat",min_height:52.0,padding:(0.0,8.0),automation_properties_name:format!("{name}, {}",if checked{"selected"}else{"not selected"}),
                    on_click:move |_|change.alarms.editor.weekdays.update(|days|{let day=index as u8;if let Some(index)=days.iter().position(|item|*item==day){days.remove(index);}else{days.push(day);days.sort_unstable();}}),
                    Grid{column_definitions:"32,*",column_spacing:16.0,
                        Border{width:28.0,height:28.0,border_brush:common::fg(&vm).as_str(),border_thickness:2.0,
                            if checked {{phone_controls::icon(Icon::Check,19.0,&common::fg(&vm))}}}
                        TextBlock{Grid.column:1,text:name,font_size:24.0}
                    }
                }}
            }).collect::<Vec<_>>();
            rsx! {StackPanel{margin:(20.0,0.0),spacing:3.0,for row in rows{{row}}}}
        }
        Picker::Snooze => {
            let rows=ALARM_SNOOZE_MINUTES.into_iter().map(|minutes| {
                let choose=ui.clone();let color=if editor.snooze.get()==minutes{common::accent(&vm)}else{common::fg(&vm)};
                rsx!{Button{class:"phone-flat",content:format!("{minutes} minutes"),font_size:27.0,min_height:56.0,
                    foreground:color.as_str(),automation_properties_name:format!("Snooze {minutes} minutes"),
                    on_click:move |_|{choose.alarms.editor.snooze.set(minutes);choose.alarms.editor.page.set(Picker::Main);}}}
            }).collect::<Vec<_>>();
            rsx! {StackPanel{margin:(20.0,0.0),spacing:4.0,for row in rows{{row}}}}
        }
    };
    let save = ui.clone();
    let save_vm = vm.clone();
    let cancel = ui.clone();
    let delete = ui.clone();
    let delete_vm = vm.clone();
    let mut actions = vec![action(
        Icon::Save,
        if page == Picker::Main { "save" } else { "done" },
        move || {
            if save.alarms.editor.page.get_untracked() == Picker::Main {
                if let Some(id) = save.alarms.editor.save(&save_vm) {
                    save.alarms.cancel_alarm(&save_vm, id);
                }
            } else {
                save.alarms.editor.page.set(Picker::Main);
            }
        },
    )];
    if page == Picker::Main && alarm.id != 0 {
        actions.push(action(Icon::Delete, "delete", move || {
            delete_vm.delete_alarm(alarm.id);
            delete.alarms.cancel_alarm(&delete_vm, alarm.id);
            delete.alarms.editor.close();
        }));
    }
    actions.push(action(Icon::Close, "cancel", move || {
        cancel.alarms.editor.dismiss();
    }));
    common::page(
        "alarms",
        common::scroll(rsx! {StackPanel{{common::title(heading)}{body}}}),
        app_bar(actions, false, Vec::new()),
    )
}
