use super::{session::CalendarPicker, CalendarView};
use ferroui_view::prelude::*;
use phone_controls::{action, app_bar, Icon};
use phone_domain::{CalendarRepeat, CalendarStatus};
use phone_sdk::{common::*, session::Session};
use std::rc::Rc;

pub fn view(ui: Rc<Session>, picker: CalendarPicker) -> Element {
    if picker == CalendarPicker::JumpDate {
        return jump_date(ui);
    }
    let (title, options): (&str, Vec<(String, bool, Rc<dyn Fn()>)>) = match picker {
        CalendarPicker::View => (
            "choose view",
            CalendarView::ALL
                .into_iter()
                .map(|value| {
                    let state = ui.clone();
                    (
                        value.label().into(),
                        ui.state::<crate::CalendarSession>().view.get() == value,
                        Rc::new(move || {
                            state.state::<crate::CalendarSession>().view.set(value);
                            state.state::<crate::CalendarSession>().selected.set(None);
                            state
                                .state::<crate::CalendarSession>()
                                .expanded_day
                                .set(None);
                        }) as Rc<dyn Fn()>,
                    )
                })
                .collect(),
        ),
        CalendarPicker::Repeat => (
            "occurs",
            CalendarRepeat::ALL
                .into_iter()
                .map(|value| {
                    let state = ui.clone();
                    (
                        value.label().into(),
                        ui.state::<crate::CalendarSession>().repeat.get() == value,
                        Rc::new(move || state.state::<crate::CalendarSession>().repeat.set(value))
                            as Rc<dyn Fn()>,
                    )
                })
                .collect(),
        ),
        CalendarPicker::Status => (
            "status",
            CalendarStatus::ALL
                .into_iter()
                .map(|value| {
                    let state = ui.clone();
                    (
                        value.label().into(),
                        ui.state::<crate::CalendarSession>().status.get() == value,
                        Rc::new(move || state.state::<crate::CalendarSession>().status.set(value))
                            as Rc<dyn Fn()>,
                    )
                })
                .collect(),
        ),
        CalendarPicker::Reminder => (
            "reminder",
            [
                None,
                Some(0),
                Some(5),
                Some(15),
                Some(30),
                Some(60),
                Some(1440),
                Some(10080),
            ]
            .into_iter()
            .map(|value| {
                let state = ui.clone();
                (
                    reminder_label(value),
                    ui.state::<crate::CalendarSession>().reminder.get() == value,
                    Rc::new(move || state.state::<crate::CalendarSession>().reminder.set(value))
                        as Rc<dyn Fn()>,
                )
            })
            .collect(),
        ),
        CalendarPicker::Duration => {
            let all_day = ui.state::<crate::CalendarSession>().all_day.get();
            let values = if all_day {
                vec![1440, 2880, 4320, 5760, 7200, 8640, 10080]
            } else {
                vec![15, 30, 45, 60, 90, 120, 180, 240, 480, 1440]
            };
            (
                "how long",
                values
                    .into_iter()
                    .map(|value| {
                        let state = ui.clone();
                        (
                            duration_label(value, all_day),
                            ui.state::<crate::CalendarSession>().duration.get() == value,
                            Rc::new(move || {
                                state.state::<crate::CalendarSession>().duration.set(value)
                            }) as Rc<dyn Fn()>,
                        )
                    })
                    .collect(),
            )
        }
        CalendarPicker::JumpDate => unreachable!("date picker handled above"),
    };
    let rows=options.into_iter().map(|(label,selected,choose)|{
        let state=ui.clone();
        let palette=phone_controls::palette();
        let foreground=if selected{palette.accent.as_str()}else{palette.foreground};
        rsx!{Button{class:"phone-flat",padding:(20.0,12.0),min_height:56.0,horizontal_content_alignment:ferroui_base::layout::HorizontalAlignment::Left,
            automation_properties_name:format!("Choose {label}"),on_click:move|_|{choose();state.state::<crate::CalendarSession>().picker.set(None);},
            TextBlock{text:label,font_size:28.0,foreground:foreground}}}
    }).collect::<Vec<_>>();
    let cancel = ui.clone();
    page(
        "calendar",
        scroll(rsx! {StackPanel{{phone_sdk::common::title(title)}for item in rows{{item}}}}),
        app_bar(
            vec![action(Icon::Close, "cancel", move || {
                cancel.state::<crate::CalendarSession>().picker.set(None)
            })],
            false,
            Vec::new(),
        ),
    )
}

fn jump_date(ui: Rc<Session>) -> Element {
    let save = ui.clone();
    let cancel = ui.clone();
    page(
        "calendar",
        scroll(rsx! {StackPanel{margin:20.0,spacing:24.0,
            {text("jump to date",40.0)}
            DatePicker{name:"CalendarJumpDate",class:"phone-picker",selected_date:ui.state::<crate::CalendarSession>().date_value.two_way(),
                with:|picker|phone_controls::attach_picker_focus(picker),
                min_year:ferroui_base::utilities::DateTimeOffset::MIN_VALUE,max_year:ferroui_base::utilities::DateTimeOffset::MAX_VALUE,
                automation_properties_name:"jump to date",horizontal_alignment:ferroui_base::layout::HorizontalAlignment::Stretch}
        }}),
        app_bar(
            vec![
                action(Icon::Check, "done", move || {
                    if let Some(date) = save
                        .state::<crate::CalendarSession>()
                        .date_value
                        .get_untracked()
                    {
                        save.state::<crate::CalendarSession>()
                            .date
                            .set(super::session::stored_date(date));
                        save.state::<crate::CalendarSession>()
                            .expanded_day
                            .set(None);
                        save.state::<crate::CalendarSession>().picker.set(None);
                    }
                }),
                action(Icon::Close, "cancel", move || {
                    cancel.state::<crate::CalendarSession>().picker.set(None)
                }),
            ],
            false,
            Vec::new(),
        ),
    )
}

pub fn duration_label(minutes: u32, all_day: bool) -> String {
    if all_day {
        let days = minutes / 1440;
        format!("{days} day{}", if days == 1 { "" } else { "s" })
    } else if minutes % 60 == 0 {
        let hours = minutes / 60;
        format!("{hours} hour{}", if hours == 1 { "" } else { "s" })
    } else {
        format!("{minutes} minutes")
    }
}
pub fn reminder_label(minutes: Option<u32>) -> String {
    match minutes {
        None => "none".into(),
        Some(0) => "at start time".into(),
        Some(1440) => "1 day before".into(),
        Some(10080) => "1 week before".into(),
        Some(value) => format!("{} before", duration_label(value, false)),
    }
}
