//! Shared native alarm/reminder presentation; hosts supply only sound/vibration.
mod app_delivery;
mod app_reminders;
#[cfg(test)]
mod app_service_tests;
pub mod editor;
mod lifecycle;
#[cfg(test)]
mod lifecycle_tests;
mod notifications;
mod pickers;
use editor::AlarmEditor;
use ferroui_base::layout::{HorizontalAlignment as H, VerticalAlignment as V};
use ferroui_view::prelude::*;
use phone_domain::{
    reminder_is_current, Alarm, AlarmSchedule, CalendarReminder, Id, ReminderSchedule,
};
use phone_sdk::{common, session::Session};
use phone_state::{HostRequest, PhoneViewModel};
use std::{
    cell::{Cell, RefCell},
    collections::VecDeque,
    rc::Rc,
    rc::Weak,
};

#[derive(Clone, PartialEq, Eq)]
pub enum Alert {
    Alarm(Alarm),
    Calendar(CalendarReminder),
    App(phone_sdk::ScopedAppReminder),
}

#[derive(PartialEq, Eq)]
enum AlertKey {
    Alarm(Id),
    Calendar(Id, i64),
    App(&'static str, String, i64, u64),
}
impl Alert {
    fn key(&self) -> AlertKey {
        match self {
            Self::Alarm(item) => AlertKey::Alarm(item.id),
            Self::Calendar(item) => {
                AlertKey::Calendar(item.occurrence.event_id, item.occurrence.start)
            }
            Self::App(item) => AlertKey::App(
                item.app_id,
                item.reminder.key.clone(),
                item.reminder.scheduled_local,
                item.document_epoch,
            ),
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
            // App reminders are reconciled against the fresh scoped provider
            // snapshot by AlarmState::current_alert, not against PhoneData fields.
            Self::App(_) => false,
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
    app_reminders: RefCell<app_reminders::Schedule>,
    app_registry: RefCell<Weak<phone_sdk::AppRegistry>>,
    refreshing_app_reminders: Cell<bool>,
    last_audio: Cell<Option<(bool, u8, bool)>>,
    schedule_revision: Cell<Option<u64>>,
    host_visible: Cell<bool>,
    closed: Cell<bool>,
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
            app_reminders: RefCell::new(app_reminders::Schedule::default()),
            app_registry: RefCell::new(Weak::new()),
            refreshing_app_reminders: Cell::new(false),
            last_audio: Cell::new(None),
            schedule_revision: Cell::new(None),
            host_visible: Cell::new(true),
            closed: Cell::new(false),
        }
    }

    pub fn clear(&self) {
        self.active.set(None);
        self.queue.borrow_mut().clear();
        self.alarms.borrow_mut().clear();
        self.reminders.borrow_mut().clear();
        self.app_reminders.borrow_mut().clear();
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
        if queue.len()
            >= phone_domain::MAX_ALARMS
                + phone_domain::MAX_CALENDAR_EVENTS
                + phone_sdk::MAX_APP_REMINDERS
        {
            queue.pop_front();
        }
        queue.push_back(item);
    }
    fn next(&self, vm: &PhoneViewModel) {
        if !self.can_deliver() || !self.refresh_app_reminders(vm) {
            return;
        }
        self.reconcile_alerts(vm);
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
        self.active.set(item.clone());
        if let Some(item) = &item {
            notifications::deliver(vm, item);
        }
    }
    fn audio(&self, vm: &PhoneViewModel) {
        if !self.can_deliver() || !self.refresh_app_reminders(vm) {
            return;
        }
        self.reconcile_alerts(vm);
        let active = self.active.get_untracked().is_some();
        let next = {
            let data = vm.data();
            let settings = &data.settings;
            (
                active,
                if active && settings.sounds {
                    settings.ringer_volume
                } else {
                    0
                },
                active && settings.vibration,
            )
        };
        if self.last_audio.replace(Some(next)) != Some(next) {
            vm.request(HostRequest::AlarmAlert {
                active: next.0,
                volume: next.1,
                vibration: next.2,
            });
        }
    }
    pub fn fire_alarm(&self, vm: &PhoneViewModel, id: Id) {
        if !self.can_deliver() {
            return;
        }
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
            self.next(vm);
            self.audio(vm);
        }
    }
    pub fn tick(&self, vm: &PhoneViewModel) {
        if !self.can_deliver() || !self.refresh_app_reminders(vm) {
            return;
        }
        if self.schedule_revision.replace(Some(vm.schedule_revision()))
            != Some(vm.schedule_revision())
        {
            self.alarms.borrow_mut().invalidate();
            self.reminders.borrow_mut().invalidate();
        }
        self.reconcile_alerts(vm);
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
        let app_reminders = self
            .app_reminders
            .borrow_mut()
            .poll(vm.now(), vm.utc_offset());
        for reminder in app_reminders {
            self.enqueue(Alert::App(reminder));
        }
        self.next(vm);
        self.audio(vm);
    }
    pub fn dismiss(&self, vm: &PhoneViewModel) {
        if self.refresh_app_reminders(vm) {
            self.reconcile_alerts(vm);
        }
        if let Some(item) = self.active.get_untracked() {
            match item {
                Alert::Alarm(alarm) => self.alarms.borrow_mut().cancel(alarm.id),
                Alert::Calendar(reminder) => self.reminders.borrow_mut().cancel(&reminder),
                Alert::App(reminder) => self.app_reminders.borrow_mut().cancel(&reminder),
            }
        }
        self.active.set(None);
        self.next(vm);
        self.audio(vm);
    }
    pub fn cancel_alarm(&self, vm: &PhoneViewModel, id: Id) {
        self.alarms.borrow_mut().cancel(id);
        self.queue
            .borrow_mut()
            .retain(|item| !matches!(item, Alert::Alarm(alarm) if alarm.id == id));
        if matches!(self.active.get_untracked(), Some(Alert::Alarm(alarm)) if alarm.id == id) {
            self.active.set(None);
            self.next(vm);
        }
        self.audio(vm);
    }
    pub fn snooze(&self, vm: &PhoneViewModel, minutes: u8) {
        if self.refresh_app_reminders(vm) {
            self.reconcile_alerts(vm);
        }
        if let Some(item) = self.active.get_untracked() {
            match item {
                Alert::Alarm(alarm) => self.alarms.borrow_mut().snooze(alarm, vm.now(), minutes),
                Alert::Calendar(reminder) => {
                    self.reminders
                        .borrow_mut()
                        .snooze(reminder, vm.now(), minutes)
                }
                Alert::App(reminder) => {
                    self.app_reminders
                        .borrow_mut()
                        .snooze(&reminder, vm.now(), minutes)
                }
            }
        }
        self.active.set(None);
        self.next(vm);
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
    let Some(item) = ui.state::<AlarmState>().active.get() else {
        return rsx! {Border{}};
    };
    let calendar = matches!(&item, Alert::Calendar(_));
    let reminder = !matches!(&item, Alert::Alarm(_));
    // Match the existing host-banner privacy preference in the in-phone
    // reminder too. Keep the actionable reminder, but never mount its private
    // caption, occurrence time, location or app detail while locked.
    let private = reminder
        && (ui.locked.get() || vm.route() == phone_domain::Route::Lock)
        && !vm.data().settings.notifications_on_lock;
    let (label, time, details) = if private {
        (
            "New reminder".to_owned(),
            String::new(),
            "Unlock to view this reminder.".to_owned(),
        )
    } else {
        match &item {
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
                        phone_domain::time_text(
                            reminder.occurrence.start,
                            vm.data().settings.time_24h,
                        )
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
            Alert::App(item) => {
                let (_, month, day) = phone_domain::date_parts(item.reminder.scheduled_local);
                (
                    item.reminder.title.clone(),
                    phone_domain::time_text(
                        item.reminder.scheduled_local,
                        vm.data().settings.time_24h,
                    ),
                    format!(
                        "{day} {} · {}{}",
                        phone_domain::month_name(month),
                        item.app_title,
                        if item.reminder.detail.is_empty() {
                            String::new()
                        } else {
                            format!("\n{}", item.reminder.detail)
                        }
                    ),
                )
            }
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
    let reminder_label = if reminder {
        rsx! {Button{name:"AppReminderOpen",class:"phone-flat",padding:0.0,horizontal_content_alignment:H::Left,
        automation_properties_name:if calendar{"Open reminder appointment"}else{"Open reminder item"},
        on_click:move |_|match &open_item {
            Alert::Calendar(reminder)=>{
                open.state::<AlarmState>().dismiss(&open_vm);open.locked.set(false);
                if let Ok(value)=serde_json::to_value(&reminder.occurrence){
                    if let Err(error)=open.queue_activation("phone.calendar",value){open_vm.set_status(error);return;}
                }
                open_vm.navigate(phone_domain::Route::App(phone_domain::AppId::Calendar));
            }
            Alert::App(item)=>open.state::<AlarmState>().open_app_reminder(&open_vm,&open,item),
            Alert::Alarm(_)=>{}
        },{common::text(&label,28.0)}}}
    } else {
        common::text(&label, 28.0)
    };
    let choices=phone_domain::ALARM_SNOOZE_MINUTES.into_iter().map(|minutes| {
        let choose=ui.clone();rsx!{Button{class:"phone-flat",content:format!("{minutes} minutes"),min_height:44.0,font_size:23.0,
            automation_properties_name:format!("Remind in {minutes} minutes"),on_click:move |_|{choose.state::<AlarmState>().snooze_minutes.set(minutes);choose.state::<AlarmState>().choosing_snooze.set(false);}}}
    }).collect::<Vec<_>>();
    rsx! {Grid{name:"PhoneAlarm",focusable:true,background:common::bg(&vm).as_str(),row_definitions:"Auto,*,Auto",automation_properties_name:if calendar{"Calendar reminder"}else if reminder{"App reminder"}else{"Alarm ringing"},
        on_loaded:move |event|if let Some(control)=event.source().and_then(|source|source.cast::<ferroui_controls::Control>()) {pickers::dismiss_phone_pickers(&control);control.focus();},
        TextBlock{text:if reminder{"reminder"}else{"alarm"},font_size:42.0,font_weight:ferroui_base::media::FontWeight::Light,margin:(20.0,24.0,20.0,12.0)},
        ScrollViewer{Grid.row:1,vertical_scroll_bar_visibility:ferroui_controls::primitives::ScrollBarVisibility::Hidden,
            StackPanel{margin:(20.0,8.0,20.0,12.0),spacing:14.0,
                if !private {TextBlock{name:"ReminderTime",text:time,font_size:56.0,font_weight:ferroui_base::media::FontWeight::Light}}
                {reminder_label}
                if !details.is_empty(){{common::muted(&details,18.0)}}
                Button{class:"phone-flat",horizontal_content_alignment:H::Stretch,padding:(0.0,10.0),automation_properties_name:"Change reminder snooze time",
                    on_click:move |_|toggle.state::<AlarmState>().choosing_snooze.update(|value|*value=!*value),
                    Grid{column_definitions:"*,32",StackPanel{{common::muted("snooze for",16.0)}{common::text(&format!("{} minutes",ui.state::<AlarmState>().snooze_minutes.get()),25.0)}}
                        Border{Grid.column:1,vertical_alignment:V::Center,{phone_controls::icon(phone_controls::Icon::ChevronDown,20.0,&common::fg(&vm))}}}}
                if ui.state::<AlarmState>().choosing_snooze.get(){StackPanel{spacing:2.0,
                    for choice in choices{{choice}}
                }}
            }
        }
        Grid{Grid.row:2,column_definitions:"*,*",column_spacing:12.0,margin:(20.0,12.0,20.0,20.0),
            Button{name:"AlarmSnooze",class:"phone-button",content:"snooze",horizontal_alignment:H::Stretch,automation_properties_name:if reminder{"Snooze reminder"}else{"Snooze alarm"},on_click:move |_|snooze.state::<AlarmState>().snooze(&snooze_vm,snooze.state::<AlarmState>().snooze_minutes.get_untracked())}
            Button{name:"AlarmDismiss",Grid.column:1,class:"phone-button",content:"dismiss",horizontal_alignment:H::Stretch,automation_properties_name:if reminder{"Dismiss reminder"}else{"Dismiss alarm"},on_click:move |_|dismiss.state::<AlarmState>().dismiss(&dismiss_vm)}
        }
    }}
}

impl Default for AlarmState {
    fn default() -> Self {
        Self::new()
    }
}
