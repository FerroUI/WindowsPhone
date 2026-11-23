use super::commands;
use ferroui_base::{
    layout::{HorizontalAlignment as H, VerticalAlignment as V},
    media::{FontWeight, TextTrimming, TextWrapping},
};
use ferroui_view::prelude::*;
use phone_domain::*;
use phone_sdk::{common::*, session::Session};
use phone_state::PhoneViewModel;
use std::rc::Rc;

pub fn view(vm: Rc<PhoneViewModel>, ui: Rc<Session>, day: i64) -> Element {
    let monday = vm.data().settings.first_day_monday;
    let start = day - (day + if monday { 3 } else { 4 }).rem_euclid(7);
    let today = vm.local_now().div_euclid(86_400);
    let data = vm.data();
    let events = data.events.clone();
    let is24 = data.settings.time_24h;
    drop(data);
    let items = event_occurrences(&events, start, start + 7);
    let color = accent(&vm);
    let line = vm.data().settings.secondary().to_owned();
    let mut days: Vec<Element> = Vec::with_capacity(7);
    for index in 0..7 {
        let date_day = start + index;
        let date = calendar_date(date_day);
        let (_, _, number) = date_parts(date_day * 86_400);
        let open = ui.clone();
        let chosen = date.clone();
        let matches = items
            .iter()
            .filter(|item| item.start < (date_day + 1) * 86_400 && item.end > date_day * 86_400)
            .collect::<Vec<_>>();
        let mut content: Vec<Element> = Vec::new();
        for item in matches.iter().take(3) {
            let Some(event) = events.iter().find(|event| event.id == item.event_id) else {
                continue;
            };
            content.push(rsx!{Border{border_brush:color.as_str(),border_thickness:(3.0,0.0,0.0,0.0),padding:(6.0,0.0,0.0,0.0),
                StackPanel{spacing:2.0,
                    TextBlock{text:event.title.clone(),font_size:17.0,text_trimming:<dyn TextTrimming>::character_ellipsis(),max_lines:1}
                    TextBlock{text:if event.all_day{"all day".into()}else{time_text(item.start,is24)},font_size:12.0,foreground:line.as_str()}
                }}});
        }
        if matches.len() > 3 {
            content.push(muted(&format!("+{} more", matches.len() - 3), 13.0));
        }
        days.push(rsx!{Button{Grid.row:(index/2)as i32,Grid.column:(index%2)as i32,class:"phone-flat",min_height:150.0,
            name:format!("CalendarWeekDay{date}"),automation_properties_name:format!("{}, {} appointments",super::long_date(&date),matches.len()),
            padding:0.0,horizontal_content_alignment:H::Stretch,vertical_content_alignment:V::Stretch,
            on_click:move|_|commands::show_day(&open,chosen.clone()),
            Border{padding:10.0,border_brush:if date_day==today{color.as_str()}else{line.as_str()},border_thickness:if date_day==today{2.0}else{0.5},
                StackPanel{spacing:8.0,
                    Grid{column_definitions:"*,Auto",TextBlock{text:weekday_name(date_day*86_400).to_lowercase(),font_size:18.0,text_wrapping:TextWrapping::NoWrap},
                        TextBlock{Grid.column:1,text:number.to_string(),font_size:26.0,font_weight:FontWeight::Light,foreground:if date_day==today{color.as_str()}else{vm.data().settings.foreground()}}}
                    StackPanel{spacing:6.0,for appointment in content{{appointment}}}
                }
            }
        }});
    }
    scroll(
        rsx! {Grid{column_definitions:"*,*",row_definitions:"Auto,Auto,Auto,Auto",
            margin:(14.0,0.0),for day in days{{day}}
        }},
    )
}
