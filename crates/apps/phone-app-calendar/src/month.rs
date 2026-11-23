use super::{details, CalendarView};
use ferroui_base::{
    layout::{HorizontalAlignment as H, VerticalAlignment as V},
    media::FontWeight,
};
use ferroui_view::prelude::*;
use phone_domain::*;
use phone_sdk::{common::*, session::Session};
use phone_state::PhoneViewModel;
use std::rc::Rc;

pub fn view(vm: Rc<PhoneViewModel>, ui: Rc<Session>, day: i64) -> Element {
    let (year, month, _) = date_parts(day * 86_400);
    let first = days_from_date(year, month, 1);
    let monday = vm.data().settings.first_day_monday;
    let start = first - (first + if monday { 3 } else { 4 }).rem_euclid(7);
    let weeks = ((first - start) as u32 + days_in_month(year, month)).div_ceil(7) as usize;
    let events = vm.data().events.clone();
    let items = event_occurrences(&events, start, start + weeks as i64 * 7);
    let today = vm.local_now().div_euclid(86_400);
    let expanded = ui
        .state::<crate::CalendarSession>()
        .expanded_day
        .get()
        .as_deref()
        .and_then(calendar_day);
    let color = accent(&vm);
    let foreground = fg(&vm);
    let secondary = vm.data().settings.secondary().to_owned();
    let headings = if monday {
        ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"]
    } else {
        ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"]
    };
    let mut header: Vec<Element> = Vec::with_capacity(7);
    for (column, label) in headings.iter().enumerate() {
        header.push(rsx!{TextBlock{Grid.column:column as i32,text:*label,font_size:13.0,foreground:secondary.as_str(),margin:(2.0,4.0,0.0,7.0)}});
    }
    let mut body: Vec<Element> = vec![rsx! {Grid{column_definitions:"*,*,*,*,*,*,*",
        for label in header{{label}}
    }}];
    for week in 0..weeks {
        let mut row: Vec<Element> = Vec::with_capacity(7);
        for col in 0..7 {
            let date_day = start + (week * 7 + col) as i64;
            let date = calendar_date(date_day);
            let (_, cell_month, num) = date_parts(date_day * 86_400);
            let matches = items
                .iter()
                .filter(|item| item.start < (date_day + 1) * 86_400 && item.end > date_day * 86_400)
                .count();
            let click = ui.clone();
            let chosen = date.clone();
            let active = expanded == Some(date_day);
            let is_today = date_day == today;
            let bars=(0..matches.min(7)).map(|index|rsx!{Border{height:2.0,width:9.0,background:color.as_str(),margin:(0.0,0.0,0.0,2.0),key:format!("busy-{date}-{index}")}}).collect::<Vec<_>>();
            row.push(rsx!{Button{Grid.column:col as i32,class:"phone-flat",name:format!("CalendarDay{date}"),
                automation_properties_name:format!("{date}, {matches} appointments"),padding:0.0,
                horizontal_content_alignment:H::Stretch,vertical_content_alignment:V::Stretch,
                on_click:move|_|{click.state::<crate::CalendarSession>().date.set(chosen.clone());click.state::<crate::CalendarSession>().expanded_day.set(if click.state::<crate::CalendarSession>().expanded_day.get_untracked().as_ref()==Some(&chosen){None}else{Some(chosen.clone())});},
                Border{border_brush:if active||is_today{color.as_str()}else{secondary.as_str()},border_thickness:if active||is_today{2.0}else{0.5},padding:4.0,
                    Grid{TextBlock{text:num.to_string(),font_size:14.0,font_weight:if is_today{FontWeight::SemiBold}else{FontWeight::Normal},
                        foreground:if cell_month==month{foreground.as_str()}else{secondary.as_str()},vertical_alignment:V::Bottom,horizontal_alignment:H::Left}
                        StackPanel{horizontal_alignment:H::Right,vertical_alignment:V::Top,for bar in bars{{bar}}}
                    }
                }
            }});
        }
        body.push(rsx! {Grid{column_definitions:"*,*,*,*,*,*,*",height:56.0,
            for cell in row{{cell}}
        }});
        if let Some(selected) = expanded.filter(|date| {
            *date >= start + week as i64 * 7 && *date < start + (week as i64 + 1) * 7
        }) {
            let jump = ui.clone();
            let chosen = calendar_date(selected);
            body.push(rsx!{Border{border_brush:color.as_str(),border_thickness:(0.0,2.0,0.0,2.0),padding:(0.0,8.0),
                StackPanel{{details::list(&vm,&ui,&events,&items,selected)}
                    Button{class:"phone-flat",content:"open day",automation_properties_name:"Open selected day",horizontal_alignment:H::Right,
                        on_click:move|_|super::commands::show_day(&jump,chosen.clone())}
                }
            }});
        }
    }
    scroll(rsx! {StackPanel{margin:(14.0,0.0),for child in body{{child}}}})
}

pub fn year(vm: Rc<PhoneViewModel>, ui: Rc<Session>, day: i64) -> Element {
    let (year, _, _) = date_parts(day * 86_400);
    let color = accent(&vm);
    let today = vm.local_now().div_euclid(86_400);
    let monday = vm.data().settings.first_day_monday;
    let first = days_from_date(year, 1, 1);
    let events = vm.data().events.clone();
    let items = event_occurrences(
        &events,
        first,
        first + if is_leap_year(year) { 366 } else { 365 },
    );
    let mut months: Vec<Element> = Vec::with_capacity(12);
    for month in 1..=12 {
        let mut days: Vec<Element> = Vec::with_capacity(days_in_month(year, month) as usize);
        let first = days_from_date(year, month, 1);
        let offset = (first + if monday { 3 } else { 4 }).rem_euclid(7) as usize;
        for number in 1..=days_in_month(year, month) {
            let day = first + i64::from(number) - 1;
            let position = offset + number as usize - 1;
            let has_event = items
                .iter()
                .any(|item| item.start < (day + 1) * 86_400 && item.end > day * 86_400);
            days.push(rsx!{Border{Grid.row:(position/7)as i32,Grid.column:(position%7)as i32,background:if day==today{color.as_str()}else{"Transparent"},
                TextBlock{text:number.to_string(),font_size:10.0,horizontal_alignment:H::Center,foreground:if day==today{"White"}else if has_event{color.as_str()}else{vm.data().settings.foreground()},
                    font_weight:if has_event{FontWeight::SemiBold}else{FontWeight::Normal}}}});
        }
        let choose = ui.clone();
        let date = date_string(year, month, 1);
        months.push(rsx!{Button{Grid.row:((month-1)/3)as i32,Grid.column:((month-1)%3)as i32,class:"phone-flat",padding:0.0,
            automation_properties_name:format!("{} {year}",month_name(month)),horizontal_content_alignment:H::Stretch,
            on_click:move|_|{choose.state::<crate::CalendarSession>().date.set(date.clone());choose.state::<crate::CalendarSession>().view.set(CalendarView::Month);choose.state::<crate::CalendarSession>().expanded_day.set(None);},
            StackPanel{spacing:8.0,TextBlock{text:month_name(month).to_lowercase(),font_size:20.0,foreground:color.as_str()}
                Grid{column_definitions:"*,*,*,*,*,*,*",row_definitions:"18,18,18,18,18,18",
                    for cell in days{{cell}}
                }
            }}});
    }
    scroll(
        rsx! {Grid{column_definitions:"*,*,*",row_definitions:"Auto,Auto,Auto,Auto",
            column_spacing:12.0,row_spacing:16.0,margin:(14.0,5.0),
            for month in months{{month}}
        }},
    )
}
