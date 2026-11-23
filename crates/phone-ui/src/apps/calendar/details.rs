use super::{commands, pickers, session::ScopeAction};
use crate::{common::*, session::Session};
use ferroui_base::{
    layout::{HorizontalAlignment as H, VerticalAlignment as V},
    media::TextWrapping,
};
use ferroui_view::prelude::*;
use phone_controls::{Icon, action, app_bar};
use phone_domain::{CalendarEvent, CalendarOccurrence, CalendarRepeat, time_text};
use phone_state::PhoneViewModel;
use std::rc::Rc;

pub fn view(vm: Rc<PhoneViewModel>, ui: Rc<Session>, item: CalendarOccurrence) -> Element {
    let event = vm
        .data()
        .events
        .iter()
        .find(|event| event.id == item.event_id)
        .cloned();
    let Some(event) = event else {
        return page(
            "calendar",
            empty("Appointment unavailable", "This appointment was removed."),
            Element::Empty,
        );
    };
    if let Some(scope) = ui.calendar.scope.get() {
        return scope_view(vm, ui, scope);
    }
    let edit_vm = vm.clone();
    let edit_ui = ui.clone();
    let delete_vm = vm.clone();
    let delete_ui = ui.clone();
    let close = ui.clone();
    let is24 = vm.data().settings.time_24h;
    let when = if event.all_day {
        "all day".into()
    } else {
        format!(
            "{} – {}",
            time_text(item.start, is24),
            time_text(item.end, is24)
        )
    };
    let body = rsx! {StackPanel{margin:20.0,spacing:18.0,
        {text(&event.title,42.0)}
        StackPanel{spacing:4.0,{text(&super::long_date(&item.date),25.0)}{text(&when,25.0)}}
        if !event.location.is_empty(){StackPanel{spacing:4.0,{muted("location",16.0)}{text(&event.location,24.0)}}}
        if event.repeat!=CalendarRepeat::Never{StackPanel{spacing:4.0,{muted("occurs",16.0)}{text(event.repeat.label(),23.0)}
            if let Some(until)=&event.repeat_until{{muted(&format!("through {}",super::long_date(until)),16.0)}}}}
        StackPanel{spacing:4.0,{muted("status",16.0)}{text(event.status.label(),23.0)}}
        StackPanel{spacing:4.0,{muted("reminder",16.0)}{text(&pickers::reminder_label(event.reminder_minutes),23.0)}}
        if !event.details.is_empty(){StackPanel{spacing:4.0,{muted("notes",16.0)}{text(&event.details,22.0)}}}
    }};
    page(
        "calendar",
        scroll(body),
        app_bar(
            vec![
                action(Icon::Edit, "edit", move || {
                    commands::edit_selected(&edit_vm, &edit_ui)
                }),
                action(Icon::Delete, "delete", move || {
                    commands::delete_selected(delete_vm.clone(), delete_ui.clone())
                }),
                action(Icon::Back, "calendar", move || {
                    close.calendar.selected.set(None)
                }),
            ],
            false,
            Vec::new(),
        ),
    )
}

fn scope_view(vm: Rc<PhoneViewModel>, ui: Rc<Session>, action_kind: ScopeAction) -> Element {
    let one_vm = vm.clone();
    let one_ui = ui.clone();
    let series_vm = vm.clone();
    let series_ui = ui.clone();
    let cancel = ui.clone();
    page(
        "calendar",
        scroll(rsx! {StackPanel{
            {title(if action_kind==ScopeAction::Edit{"edit recurring appointment"}else{"delete recurring appointment"})}
            {row("this appointment","Keep the rest of this series.",move||commands::apply_scope(one_vm.clone(),one_ui.clone(),false))}
            {row("entire series","Apply this action to every occurrence.",move||commands::apply_scope(series_vm.clone(),series_ui.clone(),true))}
        }}),
        app_bar(
            vec![action(Icon::Close, "cancel", move || {
                cancel.calendar.scope.set(None)
            })],
            false,
            Vec::new(),
        ),
    )
}

pub fn list(
    vm: &Rc<PhoneViewModel>,
    ui: &Rc<Session>,
    events: &[CalendarEvent],
    items: &[CalendarOccurrence],
    day: i64,
) -> Element {
    let matches = items
        .iter()
        .filter(|item| item.start < (day + 1) * 86_400 && item.end > day * 86_400)
        .collect::<Vec<_>>();
    if matches.is_empty() {
        return rsx! {Border{margin:(10.0,12.0),{muted("No appointments",18.0)}}};
    }
    let is24 = vm.data().settings.time_24h;
    let color = accent(vm);
    let rows=matches.into_iter().filter_map(|item|{
        let event=events.iter().find(|event|event.id==item.event_id)?;
        let open=ui.clone();let target=item.clone();
        let when=if event.all_day{"all day".into()}else if item.start<day*86_400{"continues".into()}else{time_text(item.start,is24)};
        let label=format!("{} {when}",event.title);
        Some(rsx!{Button{class:"phone-flat",name:format!("CalendarEvent{}-{}",event.id,item.date),
            automation_properties_name:label,padding:(8.0,7.0),min_height:58.0,horizontal_alignment:H::Stretch,horizontal_content_alignment:H::Stretch,
            on_click:move|_|open.calendar.open_occurrence(target.clone()),
            Grid{column_definitions:"4,80,*",column_spacing:9.0,
                Border{background:color.as_str(),vertical_alignment:V::Stretch}
                TextBlock{Grid.column:1,text:when,font_size:14.0,vertical_alignment:V::Top,text_wrapping:TextWrapping::Wrap}
                StackPanel{Grid.column:2,spacing:3.0,
                    TextBlock{text:event.title.clone(),font_size:20.0,foreground:color.as_str(),text_wrapping:TextWrapping::Wrap}
                    if !event.location.is_empty(){{muted(&event.location,14.0)}}
                }
            }
        }})
    }).collect::<Vec<_>>();
    rsx! {StackPanel{for item in rows{{item}}}}
}
