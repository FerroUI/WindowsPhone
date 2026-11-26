//! Every mutation validates the current namespace and expected task revision
//! before one SDK persistence transaction. Retired controls cannot edit a new
//! phone document, or overwrite an independently edited/completed task.
use super::model::{Fields, Library, Task, MAX_TASKS};
use phone_sdk::AppStorage;
use phone_state::PhoneViewModel;
use std::{collections::BTreeSet, rc::Rc};

pub const APP_ID: &str = "phone.calendar";

pub fn read(vm: &Rc<PhoneViewModel>) -> Result<Library, String> {
    let library = AppStorage::new(vm.clone(), APP_ID).read::<Library>()?;
    library.validate()?;
    Ok(library)
}

fn current(vm: &Rc<PhoneViewModel>, epoch: u64) -> Result<Library, String> {
    if vm.document_epoch() != epoch {
        return Err("The phone data changed. Reopen the task before saving changes.".into());
    }
    read(vm)
}

fn expected_index(library: &Library, expected: &Task) -> Result<usize, String> {
    let index = library
        .tasks
        .iter()
        .position(|task| task.id == expected.id)
        .ok_or("This task was deleted. Your unsaved text is still in the editor.")?;
    if library.tasks[index] != *expected {
        return Err("This task changed since it was opened. Reopen it before saving; your unsaved text is still in the editor.".into());
    }
    Ok(index)
}

fn write(vm: &Rc<PhoneViewModel>, library: &Library) -> Result<(), String> {
    library.validate()?;
    AppStorage::new(vm.clone(), APP_ID).write(library)
}

pub fn save(
    vm: &Rc<PhoneViewModel>,
    epoch: u64,
    original: Option<&Task>,
    mut fields: Fields,
) -> Result<Task, String> {
    fields.title = fields.title.trim().to_owned();
    fields.validate()?;
    let mut library = current(vm, epoch)?;
    let task = if let Some(original) = original {
        let index = expected_index(&library, original)?;
        if original.fields == fields {
            return Ok(original.clone());
        }
        let mut task = original.clone();
        task.revision = task
            .revision
            .checked_add(1)
            .ok_or("The task revision is exhausted.")?;
        task.fields = fields;
        task.updated = vm.now();
        library.tasks[index] = task.clone();
        task
    } else {
        if library.tasks.len() >= MAX_TASKS {
            return Err(
                "There are already 512 tasks. Delete completed tasks before adding another.".into(),
            );
        }
        let id = library.next_id;
        library.next_id = id.checked_add(1).ok_or("Task identifiers are exhausted.")?;
        let task = Task {
            id,
            revision: 1,
            fields,
            created: vm.now(),
            updated: vm.now(),
            completed_at: None,
        };
        library.tasks.push(task.clone());
        task
    };
    write(vm, &library)?;
    Ok(task)
}

pub fn complete(
    vm: &Rc<PhoneViewModel>,
    epoch: u64,
    original: &Task,
    completed: bool,
) -> Result<Task, String> {
    let mut library = current(vm, epoch)?;
    let index = expected_index(&library, original)?;
    if original.completed() == completed {
        return Ok(original.clone());
    }
    let mut task = original.clone();
    task.revision = task
        .revision
        .checked_add(1)
        .ok_or("The task revision is exhausted.")?;
    task.completed_at = completed.then(|| vm.now());
    task.updated = vm.now();
    library.tasks[index] = task.clone();
    write(vm, &library)?;
    Ok(task)
}

pub fn delete(vm: &Rc<PhoneViewModel>, epoch: u64, expected: &[Task]) -> Result<(), String> {
    let mut library = current(vm, epoch)?;
    let mut ids = BTreeSet::new();
    for task in expected {
        if !ids.insert(task.id) {
            return Err("The selected task appears more than once.".into());
        }
        expected_index(&library, task)?;
    }
    if ids.is_empty() {
        return Ok(());
    }
    library.tasks.retain(|task| !ids.contains(&task.id));
    write(vm, &library)
}

#[cfg(test)]
mod tests;
