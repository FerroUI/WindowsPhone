//! Windows Phone calendar views, native appointment editing and recurrence.
mod commands;
mod day;
mod details;
mod editor;
mod month;
mod pickers;
mod session;
mod week;

use crate::session::Session;
use ferroui_base::{
    layout::{HorizontalAlignment as H, Orientation},
    media::{FontWeight, TextWrapping},
};
use ferroui_view::prelude::*;
use phone_controls::{Icon, action, app_bar, flat};
use phone_domain::*;
use phone_state::PhoneViewModel;
use session::CalendarPicker;
pub use session::{CalendarSession, CalendarView};
use std::rc::Rc;

pub fn view(vm: Rc<PhoneViewModel>, ui: Rc<Session>) -> Element {
    if let Some(picker) = ui.calendar.picker.get() {
        return pickers::view(ui, picker);
    }
    if ui.calendar.editor.get().is_some() {
        return editor::view(vm, ui);
    }
    if let Some(selected) = ui.calendar.selected.get() {
        return details::view(vm, ui, selected);
    }
    let day = selected_day(&vm, &ui);
    let mode = ui.calendar.view.get();
    let body = match mode {
        CalendarView::Day => day::view(vm.clone(), ui.clone(), day),
        CalendarView::Week => week::view(vm.clone(), ui.clone(), day),
        CalendarView::Month => month::view(vm.clone(), ui.clone(), day),
        CalendarView::Year => month::year(vm.clone(), ui.clone(), day),
    };
    let today_vm = vm.clone();
    let today_ui = ui.clone();
    let new_ui = ui.clone();
    let date = calendar_date(day);
    let choose = ui.clone();
    let menu = ui.menu;
    let previous_vm = vm.clone();
    let previous_ui = ui.clone();
    let next_vm = vm.clone();
    let next_ui = ui.clone();
    let jump = ui.clone();
    let jump_date = calendar_date(day);
    let bar = app_bar(
        vec![
            action(Icon::Calendar, "today", move || {
                let date = calendar_date(today_vm.local_now().div_euclid(86_400));
                today_ui.calendar.date.set(date);
                today_ui.calendar.expanded_day.set(None);
            }),
            action(Icon::Plus, "new", move || {
                commands::new_event(&new_ui, date.clone(), 9 * 60)
            }),
            action(Icon::List, "view", move || {
                choose.calendar.picker.set(Some(CalendarPicker::View))
            }),
            action(Icon::More, "more", move || menu.set(!menu.get_untracked())),
        ],
        ui.menu.get(),
        vec![
            flat(&format!("previous {}", mode.label()), move || {
                previous_ui.menu.set(false);
                move_period(&previous_vm, &previous_ui, -1);
            }),
            flat(&format!("next {}", mode.label()), move || {
                next_ui.menu.set(false);
                move_period(&next_vm, &next_ui, 1);
            }),
            flat("jump to date", move || {
                jump.menu.set(false);
                jump.calendar
                    .date_value
                    .set(session::date_value(&jump_date));
                jump.calendar.picker.set(Some(CalendarPicker::JumpDate));
            }),
        ],
    );
    rsx! {Grid{name:"CalendarPage",row_definitions:"Auto,*,Auto",clip_to_bounds:true,
        {heading(&vm,&ui,day,mode)}
        Border{Grid.row:1,{body}}
        Border{Grid.row:2,{bar}}
    }}
}

fn heading(vm: &Rc<PhoneViewModel>, ui: &Rc<Session>, day: i64, mode: CalendarView) -> Element {
    let (year, month, date) = date_parts(day * 86_400);
    let advance_vm = vm.clone();
    let advance_ui = ui.clone();
    let (title, subtitle, next) = match mode {
        CalendarView::Day => (
            weekday_name(day * 86_400).to_lowercase(),
            format!("{date} {} {year}", month_name(month).to_lowercase()),
            weekday_name((day + 1) * 86_400).to_lowercase(),
        ),
        CalendarView::Week => {
            let monday = vm.data().settings.first_day_monday;
            let start = day - (day + if monday { 3 } else { 4 }).rem_euclid(7);
            let (_, first_month, first_day) = date_parts(start * 86_400);
            let (_, last_month, last_day) = date_parts((start + 6) * 86_400);
            (
                month_name(month).to_lowercase(),
                format!(
                    "{first_day} {} – {last_day} {} {year}",
                    month_name(first_month).to_lowercase(),
                    month_name(last_month).to_lowercase()
                ),
                "next week".into(),
            )
        }
        CalendarView::Month => (
            month_name(month).to_lowercase(),
            year.to_string(),
            month_name(month % 12 + 1).to_lowercase(),
        ),
        CalendarView::Year => (year.to_string(), "calendar".into(), (year + 1).to_string()),
    };
    rsx! {StackPanel{margin:(14.0,8.0,0.0,12.0),spacing:3.0,clip_to_bounds:true,
        TextBlock{text:subtitle,font_size:13.0}
        StackPanel{orientation:Orientation::Horizontal,spacing:14.0,
            TextBlock{name:"CalendarHeading",text:title,font_size:42.0,font_weight:FontWeight::Light,text_wrapping:TextWrapping::NoWrap}
            Button{class:"phone-flat",padding:0.0,min_height:50.0,automation_properties_name:format!("Next {}",mode.label()),
                horizontal_content_alignment:H::Left,on_click:move|_|move_period(&advance_vm,&advance_ui,1),
                TextBlock{text:next,font_size:42.0,font_weight:FontWeight::Light,foreground:vm.data().settings.secondary(),text_wrapping:TextWrapping::NoWrap}}
        }
    }}
}

pub fn selected_day(vm: &PhoneViewModel, ui: &Session) -> i64 {
    calendar_day(&ui.calendar.date.get()).unwrap_or_else(|| vm.local_now().div_euclid(86_400))
}

pub fn can_swipe(ui: &Session) -> bool {
    ui.calendar.editor.get_untracked().is_none()
        && ui.calendar.picker.get_untracked().is_none()
        && ui.calendar.selected.get_untracked().is_none()
        && ui.calendar.scope.get_untracked().is_none()
}

pub fn move_period(vm: &PhoneViewModel, ui: &Session, direction: i32) {
    if !can_swipe(ui) {
        return;
    }
    let day = calendar_day(&ui.calendar.date.get_untracked())
        .unwrap_or_else(|| vm.local_now().div_euclid(86_400));
    let (year, month, date) = date_parts(day * 86_400);
    let next = match ui.calendar.view.get_untracked() {
        CalendarView::Day => day.saturating_add(i64::from(direction)),
        CalendarView::Week => day.saturating_add(i64::from(direction) * 7),
        CalendarView::Month | CalendarView::Year => {
            let step = if ui.calendar.view.get_untracked() == CalendarView::Year {
                12
            } else {
                1
            };
            let index = year * 12 + month as i32 - 1 + direction * step;
            let year = index.div_euclid(12);
            let month = index.rem_euclid(12) as u32 + 1;
            if !(1..=9999).contains(&year) {
                return;
            }
            days_from_date(year, month, date.min(days_in_month(year, month)))
        }
    };
    if (days_from_date(1, 1, 1)..=days_from_date(9999, 12, 31)).contains(&next) {
        ui.calendar.date.set(calendar_date(next));
        ui.calendar.expanded_day.set(None);
    }
}

pub fn long_date(value: &str) -> String {
    calendar_day(value)
        .map(|day| {
            let (year, month, date) = date_parts(day * 86_400);
            format!(
                "{}, {date} {} {year}",
                weekday_name(day * 86_400),
                month_name(month)
            )
        })
        .unwrap_or_else(|| value.to_owned())
}
