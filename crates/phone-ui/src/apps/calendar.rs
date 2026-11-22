use crate::{
    common::{accent, empty, muted, page, row, scroll, text},
    session::Session,
};
use chrono::{Datelike, Duration, NaiveDate};
use ferroui_base::layout::{HorizontalAlignment as H, Orientation};
use ferroui_base::media::FontWeight;
use ferroui_view::prelude::*;
use phone_controls::{Icon, action, app_bar, flat, pivot};
use phone_domain::{
    CalendarEvent, date_parts, date_string, days_from_date, days_in_month, first_weekday,
    month_name,
};
use phone_state::PhoneViewModel;
use std::rc::Rc;

pub fn view(vm: Rc<PhoneViewModel>, ui: Rc<Session>) -> Element {
    let (current_year, current_month, current_day) = date_parts(vm.local_now());
    let month_index = current_year * 12 + current_month as i32 - 1 + ui.calendar_month.get();
    let (year, month) = (
        month_index.div_euclid(12),
        month_index.rem_euclid(12) as u32 + 1,
    );
    let today = date_string(current_year, current_month, current_day);
    let selected = selected_date(&ui, year, month, &today);
    let mut events = vm.data().events.clone();
    events.sort_by(|a, b| (&a.date, &a.time, &a.title).cmp(&(&b.date, &b.time, &b.title)));
    let mode = ui.pivot.get().min(2);
    let previous = ui.clone();
    let next = ui.clone();
    let previous_date = selected.clone();
    let next_date = selected.clone();
    let heading = format!("{} {}", month_name(month).to_lowercase(), year);
    let mut body=stack_panel().spacing(12.0).child(rsx! {Grid{column_definitions:"Auto,*,Auto",margin:(10.0,0.0),
        {flat("‹",move ||move_period(&previous,&previous_date,mode,-1,current_year,current_month))}
        TextBlock{Grid.column:1,text:heading,font_size:30.0,font_weight:FontWeight::Light,vertical_alignment:ferroui_base::layout::VerticalAlignment::Center}
        Border{Grid.column:2,{flat("›",move ||move_period(&next,&next_date,mode,1,current_year,current_month))}}
    }});
    let choices = ["month", "week", "day"]
        .into_iter()
        .enumerate()
        .map(|(index, label)| {
            let value = ui.pivot;
            pivot(label, mode == index, move || value.set(index))
        })
        .collect::<Vec<_>>();
    body=body.child(rsx! {StackPanel{orientation:Orientation::Horizontal,margin:(18.0,0.0),for choice in choices {{choice}}}});
    if mode == 0 {
        body = body.child(month_grid(
            &vm, &ui, year, month, &today, &selected, &events,
        ));
    }
    if mode == 1 {
        let date = NaiveDate::parse_from_str(&selected, "%Y-%m-%d").unwrap_or_else(|_| {
            NaiveDate::from_ymd_opt(current_year, current_month, current_day).unwrap_or_default()
        });
        let day = days_from_date(date.year(), date.month(), date.day());
        let start = day - (day + 3).rem_euclid(7);
        for offset in 0..7 {
            let timestamp = (start + offset) * 86400;
            let (y, m, d) = date_parts(timestamp);
            let date = date_string(y, m, d);
            body=body.child(rsx! {Border{margin:(20.0,12.0,20.0,0.0),{text(&format!("{} {}",phone_domain::weekday_name(timestamp).to_lowercase(),d),26.0)}}});
            body = body.child(event_list(vm.clone(), ui.clone(), &events, &date, false));
        }
    } else {
        body = body
            .child(rsx! {Border{margin:(20.0,12.0,20.0,0.0),{text(&selected,24.0)}}})
            .child(event_list(vm.clone(), ui.clone(), &events, &selected, true));
    }
    let add_vm = vm.clone();
    let add_ui = ui.clone();
    let add_date = selected;
    let today_ui = ui.clone();
    let now_date = today;
    let bar = app_bar(
        vec![
            action(Icon::Plus, "new", move || {
                edit(
                    add_vm.clone(),
                    add_ui.clone(),
                    CalendarEvent {
                        date: add_date.clone(),
                        time: "09:00".into(),
                        duration_minutes: 60,
                        ..CalendarEvent::default()
                    },
                )
            }),
            action(Icon::Calendar, "today", move || {
                today_ui.calendar_month.set(0);
                today_ui.calendar_date.set(now_date.clone());
            }),
        ],
        false,
        Vec::new(),
    );
    page("calendar", scroll(body.into()), bar)
}

fn selected_date(ui: &Session, year: i32, month: u32, today: &str) -> String {
    let selected = ui.calendar_date.get();
    let prefix = format!("{year:04}-{month:02}-");
    if selected.starts_with(&prefix) && NaiveDate::parse_from_str(&selected, "%Y-%m-%d").is_ok() {
        selected
    } else if today.starts_with(&prefix) {
        today.to_owned()
    } else {
        date_string(year, month, 1)
    }
}

fn move_period(
    ui: &Session,
    date: &str,
    mode: usize,
    direction: i32,
    current_year: i32,
    current_month: u32,
) {
    let Ok(date) = NaiveDate::parse_from_str(date, "%Y-%m-%d") else {
        return;
    };
    let next = if mode == 0 {
        let index = date.year() * 12 + date.month() as i32 - 1 + direction;
        let year = index.div_euclid(12);
        let month = index.rem_euclid(12) as u32 + 1;
        if !(1..=9999).contains(&year) {
            return;
        }
        NaiveDate::from_ymd_opt(year, month, date.day().min(days_in_month(year, month)))
    } else {
        date.checked_add_signed(Duration::days(
            direction as i64 * if mode == 1 { 7 } else { 1 },
        ))
    };
    if let Some(next) = next.filter(|date| (1..=9999).contains(&date.year())) {
        ui.calendar_month
            .set(next.year() * 12 + next.month() as i32 - current_year * 12 - current_month as i32);
        ui.calendar_date.set(next.format("%Y-%m-%d").to_string());
    }
}

fn month_grid(
    vm: &PhoneViewModel,
    ui: &Session,
    year: i32,
    month: u32,
    today: &str,
    selected: &str,
    events: &[CalendarEvent],
) -> Element {
    let mut cells = grid()
        .column_definitions("*,*,*,*,*,*,*")
        .row_definitions("26,46,46,46,46,46,46")
        .row_spacing(2.0)
        .column_spacing(2.0)
        .margin((16.0, 0.0));
    for (column, day) in ["m", "t", "w", "t", "f", "s", "s"].into_iter().enumerate() {
        cells=cells.child(rsx!{TextBlock{Grid.column:column as i32,text:day,font_size:16.0,foreground:"#929292",horizontal_alignment:H::Center}});
    }
    let first = first_weekday(year, month);
    for day in 1..=days_in_month(year, month) {
        let date = date_string(year, month, day);
        let offset = first + day as usize - 1;
        let active = date == selected;
        let is_today = date == today;
        let count = events.iter().filter(|event| event.date == date).count();
        let chosen = ui.calendar_date;
        let color = accent(vm);
        cells=cells.child(rsx!{Button{Grid.row:(offset/7+1)as i32,Grid.column:(offset%7)as i32,class:"phone-flat",
            background:if active{color.as_str()}else{"Transparent"},foreground:if active{"White"}else{vm.data().settings.foreground()},
            border_brush:if is_today{color.as_str()}else{"Transparent"},border_thickness:if is_today{2.0}else{0.0},padding:2.0,
            horizontal_alignment:H::Stretch,automation_properties_name:format!("{date}, {count} appointments"),
            on_click:move |_|chosen.set(date.clone()),
            StackPanel{spacing:0.0,horizontal_alignment:H::Center,
                TextBlock{text:day.to_string(),font_size:22.0,horizontal_alignment:H::Center}
                TextBlock{text:if count>0{"•"}else{""},font_size:11.0,line_height:11.0,horizontal_alignment:H::Center}
            }
        }});
    }
    cells.into()
}

fn event_list(
    vm: Rc<PhoneViewModel>,
    ui: Rc<Session>,
    events: &[CalendarEvent],
    date: &str,
    large_empty: bool,
) -> Element {
    let matches = events
        .iter()
        .filter(|event| event.date == date)
        .cloned()
        .collect::<Vec<_>>();
    if matches.is_empty() {
        return if large_empty {
            empty(
                "No appointments",
                "Your day is clear. Tap new to add an event.",
            )
        } else {
            rsx! {Border{margin:(20.0,4.0),{muted("No appointments",15.0)}}}
        };
    }
    let rows=matches.into_iter().map(|event| {
        let model=vm.clone();let state=ui.clone();let editable=event.clone();let remove=vm.clone();let confirm=ui.clone();let id=event.id;
        let subtitle=format!("{} · {} min{}",event.time,event.duration_minutes,if event.location.is_empty(){String::new()}else{format!(" · {}",event.location)});
        rsx!{Grid{column_definitions:"*,Auto",
            {row(&event.title,&subtitle,move ||edit(model.clone(),state.clone(),editable.clone()))}
            Border{Grid.column:1,{action(Icon::Delete,"delete",move ||{let model=remove.clone();confirm.confirm("delete this appointment?",Rc::new(move ||model.delete_event(id)));})}}
        }}
    }).collect::<Vec<_>>();
    rsx! {StackPanel{for row in rows {{row}}}}
}

fn edit(vm: Rc<PhoneViewModel>, ui: Rc<Session>, event: CalendarEvent) {
    let error = ui.form_error;
    let id = event.id;
    ui.form(
        if id == 0 {
            "new appointment"
        } else {
            "edit appointment"
        },
        vec![
            ("subject", event.title),
            ("date (YYYY-MM-DD)", event.date),
            ("time (HH:MM)", event.time),
            ("duration in minutes", event.duration_minutes.to_string()),
            ("location", event.location),
            ("details", event.details),
        ],
        Rc::new(move |values| {
            let field = |index| values.get(index).cloned().unwrap_or_default();
            let title = field(0).trim().to_owned();
            let date = field(1).trim().to_owned();
            let time = field(2).trim().to_owned();
            if title.is_empty() {
                error.set("Add a subject for your appointment.".into());
                return;
            }
            if !NaiveDate::parse_from_str(&date, "%Y-%m-%d")
                .is_ok_and(|date| (1..=9999).contains(&date.year()))
            {
                error.set("Enter a valid date as YYYY-MM-DD.".into());
                return;
            }
            if chrono::NaiveTime::parse_from_str(&time, "%H:%M").is_err() {
                error.set("Enter a valid time as HH:MM.".into());
                return;
            }
            let Ok(duration_minutes) = field(3).trim().parse::<u32>() else {
                error.set("Enter the duration in minutes.".into());
                return;
            };
            if duration_minutes == 0 || duration_minutes > 10080 {
                error.set("Duration must be between 1 minute and 7 days.".into());
                return;
            }
            vm.save_event(CalendarEvent {
                id,
                title,
                date,
                time,
                duration_minutes,
                location: field(4),
                details: field(5),
            });
        }),
    );
}
