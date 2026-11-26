use super::{commands, model::Task, state::State};
use ferroui_base::{layout::HorizontalAlignment as H, BoxedValue};
use ferroui_controls::ItemsSource;
use ferroui_view::prelude::*;
use phone_sdk::{common::*, session::Session};
use phone_state::PhoneViewModel;
use std::{cell::RefCell, rc::Rc};

#[derive(Clone, PartialEq)]
struct Row {
    task: Task,
    epoch: u64,
    today: i64,
}
struct Cached {
    rows: Vec<Row>,
    source: ItemsSource,
}
impl Cached {
    fn new(rows: Vec<Row>) -> Self {
        let source = ItemsSource::from_items(
            rows.iter()
                .cloned()
                .map(|row| Some(Rc::new(row) as BoxedValue)),
        );
        Self { rows, source }
    }
}

pub fn view(vm: Rc<PhoneViewModel>, ui: Rc<Session>, tasks: Vec<Task>) -> Element {
    let rows = tasks
        .into_iter()
        .map(|task| Row {
            task,
            epoch: vm.document_epoch(),
            today: vm.local_now().div_euclid(86_400),
        })
        .collect::<Vec<_>>();
    rerender(move || {
        let cache = use_hook(|| Rc::new(RefCell::new(Cached::new(rows.clone()))));
        let source = {
            let mut cache = cache.borrow_mut();
            if cache.rows != rows {
                *cache = Cached::new(rows.clone());
            }
            cache.source.clone()
        };
        let template = use_hook({
            let vm = vm.clone();
            let ui = ui.clone();
            move || template(move |row: Row| task_row(vm.clone(), ui.clone(), row))
        });
        rsx! {ListBox{name:"CalendarTaskList",class:"phone-list",items_source:source,item_template:template}}
    })
}

fn task_row(vm: Rc<PhoneViewModel>, ui: Rc<Session>, row: Row) -> Element {
    let task = row.task;
    let id = task.id;
    let epoch = row.epoch;
    let done = task.completed();
    let title = task.fields.title.clone();
    let detail = super::view::summary(&task, row.today);
    let complete_vm = vm.clone();
    let complete_ui = ui.clone();
    let complete_task = task.clone();
    let open_vm = vm.clone();
    let open = ui.clone();
    let context_vm = vm.clone();
    let context = ui.clone();
    let foreground = if done {
        phone_controls::secondary_color().to_owned()
    } else {
        fg(&vm)
    };
    rsx! {Grid{column_definitions:"64,*",margin:(8.0,0.0,12.0,0.0),
        CheckBox{name:format!("CalendarTaskComplete{id}"),class:"phone-check",content:"",
            is_checked:phone_controls::checked_state(Some(done),move|value|commands::complete(&complete_vm,&complete_ui,epoch,&complete_task,value.unwrap_or(false))),
            automation_properties_name:format!("{} task {title}",if done{"Reopen"}else{"Complete"}),
            on_click:|event|event.set_handled(true)}
        Button{Grid.column:1,name:format!("CalendarTask{id}"),class:"phone-flat",padding:(4.0,13.0),min_height:76.0,
            horizontal_alignment:H::Stretch,horizontal_content_alignment:H::Stretch,
            automation_properties_name:format!("Task {title}"),automation_properties_help_text:detail.clone(),
            on_click:move |_|if open_vm.document_epoch()==epoch{open.state::<State>().selected.set(Some(id));open.state::<State>().error.set(String::new());},
            on_context_requested:move|event|{event.set_handled(true);if context_vm.document_epoch()==epoch{context.state::<State>().context.set(Some((epoch,task.clone())));}},
            StackPanel{spacing:5.0,TextBlock{text:title,font_size:25.0,foreground:foreground.as_str(),text_wrapping:ferroui_base::media::TextWrapping::Wrap}
                {muted(&detail,15.0)}}}
    }}
}
