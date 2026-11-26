use super::{
    commands, editor, list,
    model::{Filter, Priority, Sort, Task},
    state::{Choice, State},
    store,
};
use ferroui_base::{
    layout::{HorizontalAlignment as H, VerticalAlignment as V},
    media::TextWrapping,
};
use ferroui_view::prelude::*;
use phone_controls::{action, app_bar, flat, Icon};
use phone_sdk::{common::*, session::Session};
use phone_state::PhoneViewModel;
use std::rc::Rc;

pub fn view(vm: Rc<PhoneViewModel>, ui: Rc<Session>) -> Element {
    let state = ui.state::<State>();
    if let Some(choice) = state.choice.get() {
        return choices(ui, choice);
    }
    if state.editor.get().is_some() {
        return editor::view(vm, ui);
    }
    let library = match store::read(&vm) {
        Ok(library) => library,
        Err(error) => {
            return page(
                "calendar",
                empty("tasks unavailable", &error),
                Element::Empty,
            )
        }
    };
    let body = if let Some(id) = state.selected.get() {
        match library.task(id) {
            Some(task) => details(vm.clone(), ui.clone(), task.clone()),
            None => page(
                "calendar",
                empty(
                    "task deleted",
                    "This task is no longer in your Calendar. Press Back to return to tasks.",
                ),
                Element::Empty,
            ),
        }
    } else {
        let today = vm.local_now().div_euclid(86_400);
        let rows = library.rows(
            state.filter.get(),
            state.sort.get(),
            &state.query.get(),
            today,
        );
        let count = rows.len();
        let filter = state.filter.get();
        let sort = state.sort.get();
        let change_filter = ui.clone();
        let change_sort = ui.clone();
        let new_vm = vm.clone();
        let new_ui = ui.clone();
        let search = ui.clone();
        let menu = ui.menu;
        let clear_vm = vm.clone();
        let clear_ui = ui.clone();
        let calendar = ui.clone();
        let list = if count == 0 {
            empty(
                if state.query.get().trim().is_empty() {
                    "no tasks"
                } else {
                    "no matching tasks"
                },
                if filter == Filter::Completed {
                    "Completed tasks appear here."
                } else if filter == Filter::Due {
                    "No active tasks are due today or overdue."
                } else {
                    "Tap new to add a task."
                },
            )
        } else {
            list::view(vm.clone(), ui.clone(), rows)
        };
        let content = rsx! {Grid{name:"CalendarTasks",row_definitions:"Auto,Auto,*,Auto",clip_to_bounds:true,
            StackPanel{margin:(20.0,0.0,20.0,8.0),spacing:6.0,
                {text("tasks",42.0)}
                Grid{column_definitions:"*,Auto",column_spacing:10.0,
                    Button{name:"CalendarTaskFilter",class:"phone-flat",padding:0.0,min_height:44.0,horizontal_content_alignment:H::Left,
                        automation_properties_name:format!("Filter tasks: {}",filter.label()),on_click:move |_|change_filter.state::<State>().choice.set(Some(Choice::Filter)),
                        {text(filter.label(),25.0)}}
                    Button{Grid.column:1,name:"CalendarTaskSort",class:"phone-flat",padding:0.0,min_height:44.0,
                        automation_properties_name:format!("Sort tasks: {}",sort.label()),on_click:move |_|change_sort.state::<State>().choice.set(Some(Choice::Sort)),
                        {muted(&format!("by {}",sort.label()),15.0)}}}
                {muted(&format!("{count} task{}",if count==1{""}else{"s"}),14.0)}
            }
            if state.searching.get() {TextBox{Grid.row:1,name:"CalendarTaskSearch",class:"phone-input",text:state.query.two_way(),margin:(20.0,0.0,20.0,12.0),
                automation_properties_name:"search tasks",placeholder_text:"search tasks",font_size:22.0,min_height:48.0}}
            Border{Grid.row:2,{list}}
            TextBlock{Grid.row:3,name:"CalendarTaskError",text:state.error.get(),margin:(20.0,4.0),font_size:16.0,foreground:accent(&vm).as_str(),text_wrapping:TextWrapping::Wrap}
        }};
        page(
            "calendar",
            content,
            app_bar(
                vec![
                    action(Icon::Plus, "new", move || {
                        new_ui.state::<State>().begin(&new_vm, None)
                    }),
                    action(Icon::Search, "search", move || {
                        let state = search.state::<State>();
                        state.searching.set(!state.searching.get_untracked());
                        if !state.searching.get_untracked() {
                            state.query.set(String::new());
                        }
                    }),
                    action(Icon::More, "more", move || menu.set(!menu.get_untracked())),
                ],
                ui.menu.get(),
                vec![
                    flat("delete completed", move || {
                        commands::completed(&clear_vm, &clear_ui)
                    }),
                    flat("calendar", move || {
                        calendar.menu.set(false);
                        calendar.state::<State>().active.set(false);
                    }),
                ],
            ),
        )
    };
    if let Some((epoch, task)) = state.context.get() {
        let overlay = context(vm, ui, epoch, task);
        rsx! {Grid{{body}{overlay}}}
    } else {
        body
    }
}

pub(super) fn summary(task: &Task, today: i64) -> String {
    let mut parts = Vec::new();
    if task.completed() {
        parts.push("completed".into());
    }
    if let Some(due) = &task.fields.due {
        parts.push(format!(
            "{}{}",
            if task.overdue(today) {
                "overdue · "
            } else {
                "due "
            },
            crate::long_date(due)
        ));
    }
    if task.fields.priority != Priority::Normal {
        parts.push(format!("{} priority", task.fields.priority.label()));
    }
    if task.fields.reminder.is_some() {
        parts.push("reminder".into());
    }
    if parts.is_empty() {
        "no due date".into()
    } else {
        parts.join(" · ")
    }
}

fn details(vm: Rc<PhoneViewModel>, ui: Rc<Session>, task: Task) -> Element {
    let epoch = vm.document_epoch();
    let complete_vm = vm.clone();
    let complete_ui = ui.clone();
    let complete_task = task.clone();
    let edit_vm = vm.clone();
    let edit_ui = ui.clone();
    let edit_task = task.clone();
    let pin_vm = vm.clone();
    let pin_ui = ui.clone();
    let pin_task = task.clone();
    let menu = ui.menu;
    let delete_vm = vm.clone();
    let delete_ui = ui.clone();
    let delete_task = task.clone();
    let today_vm = vm.clone();
    let today_ui = ui.clone();
    let today_task = task.clone();
    let later_vm = vm.clone();
    let later_ui = ui.clone();
    let later_task = task.clone();
    let done = task.completed();
    let body = rsx! {StackPanel{name:"CalendarTaskDetails",margin:20.0,spacing:20.0,
        TextBlock{name:"CalendarTaskTitle",text:task.fields.title.clone(),font_size:37.0,text_wrapping:TextWrapping::Wrap}
        {muted(&summary(&task,vm.local_now().div_euclid(86_400)),19.0)}
        {muted(&format!("{} priority",task.fields.priority.label()),17.0)}
        if let Some(reminder)=&task.fields.reminder {StackPanel{spacing:5.0,
            {muted("reminder",16.0)}{text(&format!("{} · {}",crate::long_date(&reminder.date),reminder.time),22.0)}}}
        if !task.fields.notes.is_empty() {StackPanel{spacing:5.0,{muted("notes",16.0)}
            TextBlock{name:"CalendarTaskDetailNotes",text:task.fields.notes.clone(),font_size:22.0,text_wrapping:TextWrapping::Wrap}}}
        TextBlock{name:"CalendarTaskError",text:ui.state::<State>().error.get(),font_size:16.0,foreground:accent(&vm).as_str(),text_wrapping:TextWrapping::Wrap}
    }};
    page(
        "calendar",
        scroll(body),
        app_bar(
            vec![
                action(
                    Icon::Check,
                    if done { "reopen" } else { "complete" },
                    move || {
                        commands::complete(&complete_vm, &complete_ui, epoch, &complete_task, !done)
                    },
                ),
                action(Icon::Edit, "edit", move || {
                    if edit_vm.document_epoch() == epoch {
                        edit_ui
                            .state::<State>()
                            .begin(&edit_vm, Some(edit_task.clone()));
                    }
                }),
                action(Icon::Pin, "pin", move || {
                    if pin_vm.document_epoch() == epoch {
                        commands::pin(&pin_vm, &pin_ui, epoch, &pin_task);
                    }
                }),
                action(Icon::More, "more", move || menu.set(!menu.get_untracked())),
            ],
            ui.menu.get(),
            vec![
                flat("due today", move || {
                    today_ui.menu.set(false);
                    commands::shift(&today_vm, &today_ui, epoch, &today_task, true);
                }),
                flat("postpone a day", move || {
                    later_ui.menu.set(false);
                    commands::shift(&later_vm, &later_ui, epoch, &later_task, false);
                }),
                flat("delete", move || {
                    commands::remove(&delete_vm, &delete_ui, epoch, vec![delete_task.clone()])
                }),
            ],
        ),
    )
}

fn choices(ui: Rc<Session>, choice: Choice) -> Element {
    let state = ui.state::<State>();
    let options: Vec<(&str, bool, Rc<dyn Fn()>)> = match choice {
        Choice::Filter => Filter::ALL
            .into_iter()
            .map(|value| {
                let s = ui.clone();
                (
                    value.label(),
                    state.filter.get() == value,
                    Rc::new(move || s.state::<State>().filter.set(value)) as Rc<dyn Fn()>,
                )
            })
            .collect(),
        Choice::Sort => Sort::ALL
            .into_iter()
            .map(|value| {
                let s = ui.clone();
                (
                    value.label(),
                    state.sort.get() == value,
                    Rc::new(move || s.state::<State>().sort.set(value)) as Rc<dyn Fn()>,
                )
            })
            .collect(),
        Choice::Priority => Priority::ALL
            .into_iter()
            .map(|value| {
                let s = ui.clone();
                (
                    value.label(),
                    state.priority.get() == value,
                    Rc::new(move || s.state::<State>().priority.set(value)) as Rc<dyn Fn()>,
                )
            })
            .collect(),
    };
    let title = match choice {
        Choice::Filter => "show tasks",
        Choice::Sort => "sort tasks",
        Choice::Priority => "priority",
    };
    let options = options
        .into_iter()
        .map(|(label, selected, choose)| {
            let close = ui.clone();
            phone_controls::radio(label, "calendar-task-choice", selected, move || {
                choose();
                close.state::<State>().choice.set(None);
            })
        })
        .collect::<Vec<_>>();
    let cancel = ui.clone();
    page(
        "calendar",
        scroll(
            rsx! {StackPanel{name:"CalendarTaskChoices",margin:20.0,spacing:10.0,{text(title,37.0)}for option in options{{option}}}},
        ),
        app_bar(
            vec![action(Icon::Close, "cancel", move || {
                cancel.state::<State>().choice.set(None)
            })],
            false,
            Vec::new(),
        ),
    )
}

fn context(vm: Rc<PhoneViewModel>, ui: Rc<Session>, epoch: u64, task: Task) -> Element {
    let close = ui.clone();
    let edit_vm = vm.clone();
    let edit_ui = ui.clone();
    let edit_task = task.clone();
    let complete_vm = vm.clone();
    let complete_ui = ui.clone();
    let complete_task = task.clone();
    let today_vm = vm.clone();
    let today_ui = ui.clone();
    let today_task = task.clone();
    let later_vm = vm.clone();
    let later_ui = ui.clone();
    let later_task = task.clone();
    let delete_vm = vm.clone();
    let delete_ui = ui.clone();
    let delete_task = task.clone();
    let cancel = ui.clone();
    let done = task.completed();
    let panel = scroll(rsx! {StackPanel{margin:20.0,spacing:4.0,
        {text(&task.fields.title,28.0)}
        {flat("edit",move||if edit_vm.document_epoch()==epoch{edit_ui.state::<State>().begin(&edit_vm,Some(edit_task.clone()));})}
        {flat(if done{"reopen"}else{"complete"},move||commands::complete(&complete_vm,&complete_ui,epoch,&complete_task,!done))}
        {flat("due today",move||commands::shift(&today_vm,&today_ui,epoch,&today_task,true))}
        {flat("postpone a day",move||commands::shift(&later_vm,&later_ui,epoch,&later_task,false))}
        {flat("delete",move||commands::remove(&delete_vm,&delete_ui,epoch,vec![delete_task.clone()]))}
        {flat("cancel",move||cancel.state::<State>().context.set(None))}
    }});
    // The menu lives inside the app's surface and lifetime. Native ContextMenu
    // popup hosts are not covered by the shell's app-overlay privacy teardown.
    rsx! {Grid{name:"CalendarTaskContext",background:"#A0000000",
        Button{class:"phone-flat",background:"Transparent",horizontal_alignment:H::Stretch,vertical_alignment:V::Stretch,
            automation_properties_name:"Close task menu",on_click:move|_|close.state::<State>().context.set(None)}
        Border{background:bg(&vm).as_str(),border_brush:fg(&vm).as_str(),border_thickness:1.0,
            margin:16.0,max_height:510.0,max_width:400.0,vertical_alignment:V::Center,horizontal_alignment:H::Stretch,{panel}}
    }}
}
