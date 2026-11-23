use super::{commands, details};
use ferroui_base::{
    layout::{HorizontalAlignment as H, VerticalAlignment as V},
    media::{TextTrimming, TextWrapping},
};
use ferroui_controls::primitives::ScrollBarVisibility;
use ferroui_view::prelude::*;
use phone_domain::*;
use phone_sdk::{common::*, session::Session};
use phone_state::PhoneViewModel;
use std::rc::Rc;

const HOUR_HEIGHT: f64 = 64.0;

struct LaneItem {
    item: CalendarOccurrence,
    top: f64,
    height: f64,
    lane: usize,
}
#[derive(Default)]
struct Group {
    top: f64,
    bottom: f64,
    lanes: Vec<f64>,
    items: Vec<LaneItem>,
}

/// Pack only visually overlapping appointments into columns. Short appointments
/// retain a readable hit area, and their expanded visual interval participates
/// in collision detection so adjacent targets do not cover one another.
fn groups(items: &[CalendarOccurrence], day: i64) -> Vec<Group> {
    let mut result: Vec<Group> = Vec::new();
    for item in items {
        let start = ((item.start - day * 86_400).max(0) as f64 / 3600.0 * HOUR_HEIGHT)
            .min(24.0 * HOUR_HEIGHT);
        let end = ((item.end - day * 86_400).min(86_400) as f64 / 3600.0 * HOUR_HEIGHT)
            .max(start + 30.0)
            .min(24.0 * HOUR_HEIGHT);
        if result.last().is_none_or(|group| start >= group.bottom) {
            result.push(Group {
                top: start,
                bottom: end,
                ..Default::default()
            });
        }
        let group = result.last_mut().expect("overlap group");
        let lane = group
            .lanes
            .iter()
            .position(|end| *end <= start)
            .unwrap_or(group.lanes.len());
        if lane == group.lanes.len() {
            group.lanes.push(end);
        } else {
            group.lanes[lane] = end;
        }
        group.bottom = group.bottom.max(end);
        group.items.push(LaneItem {
            item: item.clone(),
            top: start,
            height: end - start,
            lane,
        });
    }
    result
}

pub fn view(vm: Rc<PhoneViewModel>, ui: Rc<Session>, day: i64) -> Element {
    let data = vm.data();
    let events = data.events.clone();
    let is24 = data.settings.time_24h;
    let line = data.settings.secondary().to_owned();
    drop(data);
    let items = event_occurrences(&events, day, day + 1);
    let color = accent(&vm);
    let date = calendar_date(day);
    let (all_day, timed): (Vec<_>, Vec<_>) = items.into_iter().partition(|item| {
        events
            .iter()
            .any(|event| event.id == item.event_id && event.all_day)
    });
    let mut timeline_children: Vec<Element> = Vec::new();
    for hour in 0..24 {
        let create = ui.clone();
        let chosen = date.clone();
        let label = if is24 {
            format!("{hour:02}:00")
        } else {
            format!(
                "{} {}",
                if hour % 12 == 0 { 12 } else { hour % 12 },
                if hour < 12 { "am" } else { "pm" }
            )
        };
        timeline_children.push(rsx!{TextBlock{Grid.row:hour as i32,text:label,font_size:12.0,foreground:line.as_str(),vertical_alignment:V::Top,margin:(0.0,3.0,4.0,0.0)}});
        timeline_children.push(rsx!{Button{Grid.row:hour as i32,Grid.column:1,class:"phone-flat",name:format!("CalendarHour{hour}"),
                automation_properties_name:format!("New appointment {date} {hour:02}:00"),padding:0.0,horizontal_content_alignment:H::Stretch,vertical_content_alignment:V::Stretch,
                on_click:move|_|commands::new_event(&create,chosen.clone(),hour*60),
                Border{border_brush:line.as_str(),border_thickness:(0.0,0.5,0.0,0.0)}}});
    }
    for (group_index, group) in groups(&timed, day).into_iter().enumerate() {
        let columns = vec!["*"; group.lanes.len()].join(",");
        let mut appointments: Vec<Element> = Vec::new();
        for entry in group.items {
            let Some(event) = events.iter().find(|event| event.id == entry.item.event_id) else {
                continue;
            };
            let open = ui.clone();
            let item = entry.item.clone();
            let label = format!("{} {}", event.title, time_text(item.start, is24));
            appointments.push(rsx!{Button{Grid.column:entry.lane as i32,class:"phone-flat",name:format!("CalendarAppointment{}",event.id),
                key:format!("appointment-{}-{}",event.id,item.date),automation_properties_name:label,
                margin:(2.0,entry.top-group.top,2.0,0.0),height:entry.height,padding:0.0,
                vertical_alignment:V::Top,vertical_content_alignment:V::Stretch,horizontal_content_alignment:H::Stretch,
                on_click:move|_|open.state::<crate::CalendarSession>().open_occurrence(item.clone()),
                Border{background:if vm.data().settings.dark{"#202020"}else{"#EFEFEF"},border_brush:color.as_str(),border_thickness:(4.0,0.0,0.0,0.0),padding:(7.0,3.0),clip_to_bounds:true,
                    StackPanel{spacing:2.0,
                        TextBlock{text:event.title.clone(),font_size:18.0,foreground:color.as_str(),text_wrapping:TextWrapping::Wrap,max_lines:2,text_trimming:<dyn TextTrimming>::character_ellipsis()}
                        if entry.height>=48.0{TextBlock{text:format!("{} – {}",time_text(entry.item.start,is24),time_text(entry.item.end,is24)),font_size:12.0,text_trimming:<dyn TextTrimming>::character_ellipsis()}}
                        if entry.height>=76.0&&!event.location.is_empty(){TextBlock{text:event.location.clone(),font_size:13.0,text_trimming:<dyn TextTrimming>::character_ellipsis()}}
                    }
                }
            }});
        }
        timeline_children.push(rsx!{Border{Grid.column:1,Grid.row_span:24,key:format!("calendar-overlap-{group_index}"),vertical_alignment:V::Stretch,
            Grid{column_definitions:columns.as_str(),height:group.bottom-group.top,
                margin:(0.0,group.top,0.0,0.0),vertical_alignment:V::Top,
                for appointment in appointments{{appointment}}
            }
        }});
    }
    let rows = vec!["64"; 24].join(",");
    let timeline = rsx! {Grid{name:"CalendarTimeline",column_definitions:"48,*",
        row_definitions:rows.as_str(),margin:(12.0,0.0,14.0,20.0),
        for child in timeline_children{{child}}
    }};
    let scroll_ui = ui.clone();
    let scroll_date = date.clone();
    let initial = timed
        .first()
        .map(|item| {
            ((item.start - day * 86_400).max(0) as f64 / 3600.0 - 1.0).clamp(0.0, 20.0)
                * HOUR_HEIGHT
        })
        .unwrap_or(8.0 * HOUR_HEIGHT);
    rsx! {Grid{row_definitions:"Auto,*",
        if !all_day.is_empty(){Border{margin:(14.0,0.0),max_height:160.0,{scroll(details::list(&vm,&ui,&events,&all_day,day))}}}
        ScrollViewer{Grid.row:1,key:format!("calendar-day-{date}"),name:"CalendarDayScroll",
            vertical_scroll_bar_visibility:ScrollBarVisibility::Hidden,horizontal_scroll_bar_visibility:ScrollBarVisibility::Disabled,
            with:move|scroll|super::scroll::restore_day(scroll,&scroll_ui,scroll_date.clone(),initial),
            {timeline}
        }
    }}
}
