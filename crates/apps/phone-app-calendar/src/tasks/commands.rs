use super::{
    model::{Filter, Task},
    state::State,
    store,
};
use phone_sdk::{session::Session, AppContext};
use phone_state::PhoneViewModel;
use std::rc::Rc;

pub fn save(vm: &Rc<PhoneViewModel>, ui: &Rc<Session>) {
    let state = ui.state::<State>();
    let Some(edit) = state.editor.get_untracked() else {
        return;
    };
    let result = state
        .fields()
        .and_then(|fields| store::save(vm, edit.epoch, edit.original.as_ref(), fields));
    if !state.matches(vm, &edit) {
        return;
    }
    match result {
        Ok(task) => {
            state.close_editor();
            state.selected.set(Some(task.id));
        }
        Err(error) => state.error.set(error),
    }
}

pub fn complete(vm: &Rc<PhoneViewModel>, ui: &Rc<Session>, epoch: u64, task: &Task, value: bool) {
    match store::complete(vm, epoch, task, value) {
        Ok(_) => {
            if vm.document_epoch() == epoch {
                ui.state::<State>().close_overlays();
                ui.state::<State>().error.set(String::new());
            }
        }
        Err(error) => {
            if vm.document_epoch() == epoch {
                ui.state::<State>().error.set(error);
            }
        }
    }
}

pub fn shift(vm: &Rc<PhoneViewModel>, ui: &Rc<Session>, epoch: u64, task: &Task, today: bool) {
    let day = vm.local_now().div_euclid(86_400);
    let fields = if today {
        task.fields.due_today(day)
    } else {
        task.fields.postponed(day)
    };
    let result = fields.and_then(|fields| store::save(vm, epoch, Some(task), fields));
    if vm.document_epoch() != epoch {
        return;
    }
    match result {
        Ok(_) => {
            ui.state::<State>().close_overlays();
            ui.state::<State>().error.set(String::new());
        }
        Err(error) => ui.state::<State>().error.set(error),
    }
}

pub fn remove(vm: &Rc<PhoneViewModel>, ui: &Rc<Session>, epoch: u64, tasks: Vec<Task>) {
    if tasks.is_empty() {
        return;
    }
    if vm.document_epoch() != epoch {
        return;
    }
    let title = if tasks.len() == 1 {
        "delete this task?"
    } else {
        "delete completed tasks?"
    };
    let model = vm.clone();
    let session = ui.clone();
    ui.state::<State>().close_overlays();
    ui.menu.set(false);
    ui.form_with_action(
        title,
        vec![],
        "delete",
        Rc::new(move |_| {
            if model.document_epoch() != epoch {
                return;
            }
            match store::delete(&model, epoch, &tasks) {
                Ok(()) => {
                    if model.document_epoch() != epoch {
                        return;
                    }
                    let state = session.state::<State>();
                    if state
                        .selected
                        .get_untracked()
                        .is_some_and(|id| tasks.iter().any(|task| task.id == id))
                    {
                        state.selected.set(None);
                    }
                    state.error.set(String::new());
                }
                Err(error) => {
                    if model.document_epoch() == epoch {
                        session.form_error.set(error);
                    }
                }
            }
        }),
    );
}

pub fn pin(vm: &Rc<PhoneViewModel>, ui: &Rc<Session>, epoch: u64, task: &Task) {
    if vm.document_epoch() != epoch {
        return;
    }
    if !store::read(vm).is_ok_and(|library| library.task(task.id) == Some(task)) {
        ui.state::<State>()
            .error
            .set("This task changed. Reopen it before pinning.".into());
        return;
    }
    let title: String = task.fields.title.chars().take(80).collect();
    let result = AppContext {
        vm: vm.clone(),
        session: ui.clone(),
        id: store::APP_ID,
    }
    .pin(&format!("task/{}", task.id), &title);
    if vm.document_epoch() == epoch {
        match result {
            Ok(()) => vm.home(),
            Err(error) => ui.state::<State>().error.set(error),
        }
    }
}

pub fn completed(vm: &Rc<PhoneViewModel>, ui: &Rc<Session>) {
    match store::read(vm) {
        Ok(library) => {
            let tasks = library
                .tasks
                .into_iter()
                .filter(Task::completed)
                .collect::<Vec<_>>();
            if tasks.is_empty() {
                ui.state::<State>().filter.set(Filter::Completed);
                ui.menu.set(false);
            } else {
                remove(vm, ui, vm.document_epoch(), tasks);
            }
        }
        Err(error) => ui.state::<State>().error.set(error),
    }
}
