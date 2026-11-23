use crate::{common, images, session::Session};
use chrono::Datelike;
use ferroui_base::layout::{HorizontalAlignment as H, Orientation};
use ferroui_base::media::{FontWeight, TextWrapping};
use ferroui_view::prelude::*;
use phone_controls::icon;
use phone_domain::{AppId, PhoneData, Route};
use phone_state::PhoneViewModel;
use std::rc::Rc;

pub fn view(vm: Rc<PhoneViewModel>, ui: Rc<Session>) -> Element {
    let data = vm.data();
    let settings = data.settings.clone();
    let wallpaper = if !settings.lock_wallpaper.is_empty() {
        settings.lock_wallpaper.clone()
    } else if !settings.wallpaper.is_empty() {
        settings.wallpaper.clone()
    } else {
        "coast".into()
    };
    let date = chrono::DateTime::from_timestamp(vm.local_now(), 0).unwrap_or_default();
    let today = date.format("%Y-%m-%d").to_string();
    let details = settings
        .lock_app(&settings.lock_detail_app)
        .filter(|_| settings.notifications_on_lock)
        .and_then(|app| detail(&data, app, date.naive_utc()));
    let mut status = Vec::new();
    if settings.notifications_on_lock {
        for name in &settings.lock_quick_apps {
            let Some(app) = settings.lock_app(name) else {
                continue;
            };
            let count = quick_count(&data, app, &today);
            if count == 0 {
                continue;
            }
            status.push(rsx! { StackPanel {
                orientation:Orientation::Horizontal,spacing:6.0,
                automation_properties_name:format!("{}: {count}",app.label()),
                {icon(common::app_icon(app),20.0,"White")}
                TextBlock{text:count.to_string(),font_size:20.0,foreground:"White"}
            }});
        }
    }
    drop(data);
    let compact = crate::shell::phone_size(&vm).1 < 590.0;
    let unlock = ui.clone();
    let unlock_vm = vm.clone();
    let action_ui = ui.clone();
    let status_color = "White";
    rsx! {Grid{name:"LockScreen",background:"Black",row_definitions:"26,*,Auto,58",clip_to_bounds:true,
        if !settings.high_contrast {Border{Grid.row_span:4,{images::photo(&wallpaper)}}}
        Border{Grid.row_span:4,background:"#33000000"}
        Button{class:"phone-flat",height:26.0,horizontal_content_alignment:H::Stretch,
            automation_properties_name:"Open action center from lock screen",
            on_click:move |_|{action_ui.notification_peek.set(false);action_ui.notification_open.set(true);},
            Grid{column_definitions:"*,Auto",margin:(15.0,3.0),
                StackPanel{orientation:Orientation::Horizontal,spacing:7.0,
                    {icon(if settings.airplane{phone_controls::Icon::Airplane}else{phone_controls::Icon::Signal},14.0,status_color)}
                    if settings.wi_fi{{icon(phone_controls::Icon::Wifi,14.0,status_color)}}
                }
                Border{Grid.column:1,{crate::navigation::battery(&vm,status_color)}}
            }
        }
        StackPanel{Grid.row:2,margin:(22.0,0.0,20.0,22.0),spacing:0.0,
            TextBlock{text:common::clock(&vm),font_size:if compact{64.0}else{86.0},font_weight:FontWeight::Light,foreground:"White"}
            TextBlock{text:date.format("%A").to_string(),font_size:if compact{24.0}else{29.0},font_weight:FontWeight::Light,foreground:"White"}
            TextBlock{text:date.format("%e %B").to_string().trim().to_owned(),font_size:if compact{24.0}else{29.0},font_weight:FontWeight::Light,foreground:"White"}
            if let Some((heading,body))=details {StackPanel{margin:(0.0,20.0,0.0,0.0),spacing:4.0,
                TextBlock{text:heading,font_size:22.0,foreground:"White",max_lines:2,text_wrapping:TextWrapping::Wrap}
                TextBlock{text:body,font_size:16.0,foreground:"White",max_lines:2,text_wrapping:TextWrapping::Wrap}
            }}
            StackPanel{orientation:Orientation::Horizontal,spacing:16.0,margin:(0.0,20.0,0.0,0.0),for item in status{{item}}}
        }
        Button{Grid.row:3,class:"phone-flat",automation_properties_name:"Unlock",horizontal_alignment:H::Stretch,horizontal_content_alignment:H::Center,
            on_click:move |_|{unlock.locked.set(false);if unlock_vm.route()==Route::Lock{unlock_vm.back();}},
            TextBlock{text:"swipe up to unlock",font_size:14.0,foreground:"White"}
        }
    }}
}

fn quick_count(data: &PhoneData, app: AppId, today: &str) -> usize {
    match app {
        AppId::Calendar => phone_domain::calendar_day(today)
            .map(|day| phone_domain::event_occurrences(&data.events, day, day + 1).len())
            .unwrap_or(0),
        AppId::Alarms => data.alarms.iter().filter(|alarm| alarm.enabled).count(),
        AppId::OneNote => data.notes.len(),
        _ => data.badge(app),
    }
}

fn detail(data: &PhoneData, app: AppId, now: chrono::NaiveDateTime) -> Option<(String, String)> {
    let today = now.format("%Y-%m-%d").to_string();
    match app {
        AppId::Calendar => {
            phone_domain::next_calendar_occurrence(&data.events, now.and_utc().timestamp())
                .and_then(|occurrence| {
                    let event = data
                        .events
                        .iter()
                        .find(|event| event.id == occurrence.event_id)?;
                    let time = if event.all_day {
                        "all day".into()
                    } else {
                        phone_domain::time_text(occurrence.start, data.settings.time_24h)
                    };
                    let when = if occurrence.date == today {
                        time
                    } else {
                        format!("{} {time}", occurrence.date)
                    };
                    Some((
                        event.title.clone(),
                        [when, event.location.clone()]
                            .into_iter()
                            .filter(|value| !value.is_empty())
                            .collect::<Vec<_>>()
                            .join(" · "),
                    ))
                })
        }
        AppId::Messaging => data
            .conversations
            .iter()
            .filter(|item| item.unread)
            .max_by_key(|item| item.timestamp())
            .map(|item| (data.contact_name(item), item.preview().into())),
        AppId::Outlook => data
            .mail
            .iter()
            .filter(|item| !item.read && item.folder == "inbox")
            .max_by_key(|item| item.timestamp)
            .map(|item| (item.from.clone(), item.subject.clone())),
        AppId::Phone => data
            .call_history
            .iter()
            .filter(|item| item.missed)
            .max_by_key(|item| item.timestamp)
            .map(|item| (item.name.clone(), "missed call".into())),
        AppId::Alarms => data
            .alarms
            .iter()
            .filter_map(|item| next_alarm(item, now).map(|next| (next, item)))
            .min_by_key(|(next, _)| *next)
            .map(|(next, item)| {
                let day = if next.date() == now.date() {
                    "today".into()
                } else if next.date() == now.date() + chrono::Duration::days(1) {
                    "tomorrow".into()
                } else {
                    next.format("%A").to_string()
                };
                (
                    item.label.clone(),
                    format!(
                        "{} · {day}",
                        phone_domain::time_text(next.and_utc().timestamp(), data.settings.time_24h)
                    ),
                )
            }),
        AppId::OneNote => data
            .notes
            .iter()
            .max_by_key(|item| item.modified)
            .map(|item| {
                (
                    item.title.clone(),
                    item.body.lines().next().unwrap_or_default().into(),
                )
            }),
        _ => None,
    }
}

fn next_alarm(
    alarm: &phone_domain::Alarm,
    now: chrono::NaiveDateTime,
) -> Option<chrono::NaiveDateTime> {
    if !alarm.enabled {
        return None;
    }
    let time = chrono::NaiveTime::from_hms_opt(u32::from(alarm.hour), u32::from(alarm.minute), 0)?;
    (0..=7).find_map(|offset| {
        let date = now.date() + chrono::Duration::days(offset);
        let next = date.and_time(time);
        (next >= now
            && (alarm.weekdays.is_empty()
                || alarm
                    .weekdays
                    .contains(&(date.weekday().num_days_from_monday() as u8))))
        .then_some(next)
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn lock_details_skip_finished_events_and_choose_the_next_alarm_day() {
        let now = chrono::NaiveDate::from_ymd_opt(2026, 11, 23)
            .unwrap()
            .and_hms_opt(10, 0, 0)
            .unwrap();
        let mut data = PhoneData::default();
        data.events = vec![
            phone_domain::CalendarEvent {
                id: 1,
                title: "Finished".into(),
                date: "2026-11-23".into(),
                time: "08:00".into(),
                duration_minutes: 30,
                ..Default::default()
            },
            phone_domain::CalendarEvent {
                id: 2,
                title: "Overnight".into(),
                date: "2026-11-22".into(),
                time: "23:00".into(),
                duration_minutes: 720,
                ..Default::default()
            },
        ];
        assert_eq!(detail(&data, AppId::Calendar, now).unwrap().0, "Overnight");
        let alarm = phone_domain::Alarm {
            id: 3,
            label: "weekly".into(),
            hour: 9,
            minute: 0,
            enabled: true,
            weekdays: vec![0],
            ..Default::default()
        };
        assert_eq!(
            next_alarm(&alarm, now)
                .unwrap()
                .format("%Y-%m-%d %H:%M")
                .to_string(),
            "2026-11-30 09:00"
        );
        let daily = phone_domain::Alarm {
            weekdays: vec![],
            ..alarm
        };
        assert_eq!(
            next_alarm(&daily, now)
                .unwrap()
                .format("%Y-%m-%d %H:%M")
                .to_string(),
            "2026-11-24 09:00"
        );
    }
}
