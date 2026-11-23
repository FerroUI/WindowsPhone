//! Shared native alarm/reminder presentation; hosts supply only sound/vibration.
mod pickers;
use crate::{apps::alarms::editor::AlarmEditor, common, session::Session};
use ferroui_base::layout::{HorizontalAlignment as H, VerticalAlignment as V};
use ferroui_view::prelude::*;
use phone_domain::{
    reminder_is_current, Alarm, AlarmSchedule, CalendarReminder, Id, ReminderSchedule,
};
use phone_state::{HostRequest, PhoneViewModel};
use std::{
    cell::{Cell, RefCell},
    collections::VecDeque,
    rc::Rc,
};

#[derive(Clone, PartialEq, Eq)]
pub enum Alert {
    Alarm(Alarm),
    Calendar(CalendarReminder),
}
impl Alert {
    fn key(&self) -> (u8, Id, i64) {
        match self {
            Self::Alarm(item) => (0, item.id, 0),
            Self::Calendar(item) => (1, item.occurrence.event_id, item.occurrence.start),
        }
    }
    fn current(&self, vm: &PhoneViewModel) -> bool {
        let data = vm.data();
        match self {
            Self::Alarm(item) => data.alarms.iter().any(|alarm| {
                alarm.id == item.id
                    && (alarm.enabled || item.weekdays.is_empty())
                    && alarm.hour == item.hour
                    && alarm.minute == item.minute
                    && alarm.weekdays == item.weekdays
            }),
            Self::Calendar(item) => reminder_is_current(item, &data.events),
        }
    }
}

pub struct AlarmState {
    pub active: Signal<Option<Alert>>,
    pub snooze_minutes: Signal<u8>,
    pub choosing_snooze: Signal<bool>,
    pub editor: AlarmEditor,
    queue: RefCell<VecDeque<Alert>>,
    alarms: RefCell<AlarmSchedule>,
    reminders: RefCell<ReminderSchedule>,
    last_audio: Cell<Option<(bool, u8, bool)>>,
    schedule_revision: Cell<Option<u64>>,
}
impl AlarmState {
    pub fn new() -> Self {
        Self {
            active: signal(None),
            snooze_minutes: signal(5),
            choosing_snooze: signal(false),
            editor: AlarmEditor::new(),
            queue: RefCell::new(VecDeque::new()),
            alarms: RefCell::new(AlarmSchedule::default()),
            reminders: RefCell::new(ReminderSchedule::default()),
            last_audio: Cell::new(None),
            schedule_revision: Cell::new(None),
        }
    }

    pub fn clear(&self) {
        self.active.set(None);
        self.queue.borrow_mut().clear();
        self.alarms.borrow_mut().clear();
        self.reminders.borrow_mut().clear();
        self.choosing_snooze.set(false);
        self.editor.close();
        self.last_audio.set(None);
        self.schedule_revision.set(None);
    }

    fn enqueue(&self, item: Alert) {
        let key = item.key();
        if self
            .active
            .get_untracked()
            .is_some_and(|item| item.key() == key)
            || self.queue.borrow().iter().any(|item| item.key() == key)
        {
            return;
        }
        let mut queue = self.queue.borrow_mut();
        // A long-lived visible alert must not accumulate every occurrence of a
        // recurring series. Keep its newest queued occurrence and a fixed cap.
        if let Alert::Calendar(reminder) = &item {
            queue.retain(|queued| {
                !matches!(queued, Alert::Calendar(previous)
                if previous.occurrence.event_id == reminder.occurrence.event_id)
            });
        }
        if queue.len() >= phone_domain::MAX_ALARMS + phone_domain::MAX_CALENDAR_EVENTS {
            queue.pop_front();
        }
        queue.push_back(item);
    }
    fn next(&self) {
        if self.active.get_untracked().is_some() {
            return;
        }
        let item = self.queue.borrow_mut().pop_front();
        if let Some(item) = &item {
            self.snooze_minutes.set(match item {
                Alert::Alarm(alarm) => alarm.snooze_minutes,
                _ => 5,
            });
        }
        self.choosing_snooze.set(false);
        self.active.set(item);
    }
    fn audio(&self, vm: &PhoneViewModel) {
        let active = self.active.get_untracked().is_some();
        let settings = &vm.data().settings;
        let next = (
            active,
            if active && settings.sounds {
                settings.ringer_volume
            } else {
                0
            },
            active && settings.vibration,
        );
        if self.last_audio.replace(Some(next)) != Some(next) {
            vm.request(HostRequest::AlarmAlert {
                active: next.0,
                volume: next.1,
                vibration: next.2,
            });
        }
    }
    pub fn fire_alarm(&self, vm: &PhoneViewModel, id: Id) {
        let alarm = vm
            .data()
            .alarms
            .iter()
            .find(|alarm| alarm.id == id && alarm.enabled)
            .cloned();
        if let Some(alarm) = alarm {
            if !self
                .alarms
                .borrow_mut()
                .mark_fired(id, vm.now(), vm.utc_offset())
            {
                return;
            }
            if alarm.weekdays.is_empty() {
                vm.toggle_alarm(id);
            }
            self.enqueue(Alert::Alarm(alarm));
            self.next();
            self.audio(vm);
        }
    }
    pub fn tick(&self, vm: &PhoneViewModel) {
        if self.schedule_revision.replace(Some(vm.schedule_revision()))
            != Some(vm.schedule_revision())
        {
            self.alarms.borrow_mut().invalidate();
            self.reminders.borrow_mut().invalidate();
        }
        if self
            .active
            .get_untracked()
            .is_some_and(|item| !item.current(vm))
        {
            self.active.set(None);
        }
        self.queue.borrow_mut().retain(|item| item.current(vm));
        let data = vm.data();
        let alarms = self
            .alarms
            .borrow_mut()
            .poll(&data.alarms, vm.now(), vm.utc_offset());
        let reminders = self
            .reminders
            .borrow_mut()
            .poll(&data.events, vm.now(), vm.utc_offset());
        drop(data);
        for alarm in alarms {
            if alarm.weekdays.is_empty() && alarm.enabled {
                vm.toggle_alarm(alarm.id);
            }
            self.enqueue(Alert::Alarm(alarm));
        }
        for reminder in reminders {
            self.enqueue(Alert::Calendar(reminder));
        }
        self.next();
        self.audio(vm);
    }
    pub fn dismiss(&self, vm: &PhoneViewModel) {
        if let Some(item) = self.active.get_untracked() {
            match item {
                Alert::Alarm(alarm) => self.alarms.borrow_mut().cancel(alarm.id),
                Alert::Calendar(reminder) => self.reminders.borrow_mut().cancel(&reminder),
            }
        }
        self.active.set(None);
        self.next();
        self.audio(vm);
    }
    pub fn cancel_alarm(&self, vm: &PhoneViewModel, id: Id) {
        self.alarms.borrow_mut().cancel(id);
        self.queue
            .borrow_mut()
            .retain(|item| !matches!(item, Alert::Alarm(alarm) if alarm.id == id));
        if matches!(self.active.get_untracked(), Some(Alert::Alarm(alarm)) if alarm.id == id) {
            self.active.set(None);
            self.next();
        }
        self.audio(vm);
    }
    pub fn snooze(&self, vm: &PhoneViewModel, minutes: u8) {
        if let Some(item) = self.active.get_untracked() {
            match item {
                Alert::Alarm(alarm) => self.alarms.borrow_mut().snooze(alarm, vm.now(), minutes),
                Alert::Calendar(reminder) => {
                    self.reminders
                        .borrow_mut()
                        .snooze(reminder, vm.now(), minutes)
                }
            }
        }
        self.active.set(None);
        self.next();
        self.audio(vm);
    }
    pub fn back(&self, vm: &PhoneViewModel) -> bool {
        let Some(item) = self.active.get_untracked() else {
            return false;
        };
        if self.choosing_snooze.get_untracked() {
            self.choosing_snooze.set(false);
        } else {
            self.snooze(
                vm,
                if matches!(item, Alert::Alarm(_)) {
                    2
                } else {
                    5
                },
            );
        }
        true
    }
}

pub fn view(vm: Rc<PhoneViewModel>, ui: Rc<Session>) -> Element {
    let Some(item) = ui.alarms.active.get() else {
        return rsx! {Border{}};
    };
    let calendar = matches!(&item, Alert::Calendar(_));
    let (label, time, details) = match &item {
        Alert::Alarm(alarm) => (
            alarm.label.clone(),
            phone_domain::time_text(
                i64::from(alarm.hour) * 3600 + i64::from(alarm.minute) * 60,
                vm.data().settings.time_24h,
            ),
            String::new(),
        ),
        Alert::Calendar(reminder) => {
            let (_, month, day) = phone_domain::date_parts(reminder.occurrence.start);
            let all_day = vm
                .data()
                .events
                .iter()
                .any(|event| event.id == reminder.occurrence.event_id && event.all_day);
            (
                reminder.title.clone(),
                if all_day {
                    "all day".into()
                } else {
                    phone_domain::time_text(reminder.occurrence.start, vm.data().settings.time_24h)
                },
                format!(
                    "{day} {}{}",
                    phone_domain::month_name(month),
                    if reminder.location.is_empty() {
                        String::new()
                    } else {
                        format!(" · {}", reminder.location)
                    }
                ),
            )
        }
    };
    let toggle = ui.clone();
    let snooze = ui.clone();
    let snooze_vm = vm.clone();
    let dismiss = ui.clone();
    let dismiss_vm = vm.clone();
    let open = ui.clone();
    let open_vm = vm.clone();
    let open_item = item.clone();
    let choices=phone_domain::ALARM_SNOOZE_MINUTES.into_iter().map(|minutes| {
        let choose=ui.clone();rsx!{Button{class:"phone-flat",content:format!("{minutes} minutes"),min_height:44.0,font_size:23.0,
            automation_properties_name:format!("Remind in {minutes} minutes"),on_click:move |_|{choose.alarms.snooze_minutes.set(minutes);choose.alarms.choosing_snooze.set(false);}}}
    }).collect::<Vec<_>>();
    rsx! {Grid{name:"PhoneAlarm",focusable:true,background:common::bg(&vm).as_str(),row_definitions:"Auto,*,Auto",automation_properties_name:if calendar{"Calendar reminder"}else{"Alarm ringing"},
        on_loaded:move |event|if let Some(control)=event.source().and_then(|source|source.cast::<ferroui_controls::Control>()) {pickers::dismiss_phone_pickers(&control);control.focus();},
        TextBlock{text:if calendar{"reminder"}else{"alarm"},font_size:42.0,font_weight:ferroui_base::media::FontWeight::Light,margin:(20.0,24.0,20.0,12.0)},
        ScrollViewer{Grid.row:1,vertical_scroll_bar_visibility:ferroui_controls::primitives::ScrollBarVisibility::Hidden,
            StackPanel{margin:(20.0,8.0,20.0,12.0),spacing:14.0,
                TextBlock{text:time,font_size:56.0,font_weight:ferroui_base::media::FontWeight::Light},
                if calendar {Button{class:"phone-flat",padding:0.0,horizontal_content_alignment:H::Left,automation_properties_name:"Open reminder appointment",
                    on_click:move |_|if let Alert::Calendar(reminder)=&open_item{
                        open.alarms.dismiss(&open_vm);open.locked.set(false);let route=phone_domain::Route::App(phone_domain::AppId::Calendar);
                        open_vm.navigate(route.clone());open.route_changed(&route,open_vm.now());open.calendar.open_occurrence(reminder.occurrence.clone());
                    },{common::text(&label,28.0)}}}else{{common::text(&label,28.0)}}
                if !details.is_empty(){{common::muted(&details,18.0)}}
                Button{class:"phone-flat",horizontal_content_alignment:H::Stretch,padding:(0.0,10.0),automation_properties_name:"Change reminder snooze time",
                    on_click:move |_|toggle.alarms.choosing_snooze.update(|value|*value=!*value),
                    Grid{column_definitions:"*,32",StackPanel{{common::muted("snooze for",16.0)}{common::text(&format!("{} minutes",ui.alarms.snooze_minutes.get()),25.0)}}
                        Border{Grid.column:1,vertical_alignment:V::Center,{phone_controls::icon(phone_controls::Icon::ChevronDown,20.0,&common::fg(&vm))}}}}
                if ui.alarms.choosing_snooze.get(){StackPanel{spacing:2.0,
                    for choice in choices{{choice}}
                }}
            }
        }
        Grid{Grid.row:2,column_definitions:"*,*",column_spacing:12.0,margin:(20.0,12.0,20.0,20.0),
            Button{name:"AlarmSnooze",class:"phone-button",content:"snooze",horizontal_alignment:H::Stretch,automation_properties_name:if calendar{"Snooze reminder"}else{"Snooze alarm"},on_click:move |_|snooze.alarms.snooze(&snooze_vm,snooze.alarms.snooze_minutes.get_untracked())}
            Button{name:"AlarmDismiss",Grid.column:1,class:"phone-button",content:"dismiss",horizontal_alignment:H::Stretch,automation_properties_name:if calendar{"Dismiss reminder"}else{"Dismiss alarm"},on_click:move |_|dismiss.alarms.dismiss(&dismiss_vm)}
        }
    }}
}
