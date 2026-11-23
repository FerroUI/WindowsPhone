use super::{file::Completion, validation, FilePickerOptions, FileResult, MAX_PENDING_FILE_TASKS};
use crate::{session::Session, AppContext};
use phone_state::PhoneViewModel;
use std::{
    cell::RefCell,
    collections::BTreeMap,
    rc::{Rc, Weak},
};

struct Pending {
    session: Weak<Session>,
    vm: Weak<PhoneViewModel>,
    app: &'static str,
    epoch: u64,
    options: FilePickerOptions,
    completion: Rc<RefCell<Completion>>,
    completed: bool,
    cancelled: Option<String>,
}

impl Pending {
    fn valid(&self) -> Result<(), String> {
        if let Some(error) = &self.cancelled {
            return Err(error.clone());
        }
        let session = self
            .session
            .upgrade()
            .ok_or("The app session has closed.")?;
        let vm = self.vm.upgrade().ok_or("The phone has closed.")?;
        if vm.document_epoch() != self.epoch {
            return Err("File selection was cancelled because phone data changed.".into());
        }
        let registry = session.registry();
        let app = registry
            .by_id(self.app)
            .ok_or("The app is no longer registered.")?;
        let available = app.manifest.builtin.map_or_else(
            || vm.extension_enabled(self.app),
            |id| vm.data().installed.contains(&id),
        );
        if !available {
            return Err("The app is disabled or unavailable.".into());
        }
        Ok(())
    }
}

struct Registry {
    next: Option<u32>,
    pending: BTreeMap<u32, Pending>,
}
ferroui_base::ferro_thread_local! {
    static FILE_TASKS: RefCell<Registry> = RefCell::new(Registry {next: Some(1), pending: BTreeMap::new()});
}

pub(super) fn insert(
    context: &AppContext,
    options: FilePickerOptions,
    completion: Rc<RefCell<Completion>>,
) -> Result<u32, String> {
    let pending = Pending {
        session: Rc::downgrade(&context.session),
        vm: Rc::downgrade(&context.vm),
        app: context.id,
        epoch: context.vm.document_epoch(),
        options,
        completion,
        completed: false,
        cancelled: None,
    };
    pending.valid()?;
    FILE_TASKS.with(|slot| {
        let mut registry = slot.borrow_mut();
        if registry.pending.len() >= MAX_PENDING_FILE_TASKS {
            return Err("There are already 16 pending file selections.".into());
        }
        let id = registry
            .next
            .ok_or("The file request identifier space is exhausted.")?;
        registry.next = id.checked_add(1);
        registry.pending.insert(id, pending);
        Ok(id)
    })
}

pub(super) fn abandon(id: u32) {
    let removed = FILE_TASKS.with(|slot| slot.borrow_mut().pending.remove(&id));
    drop(removed);
}

/// Keep the scope alive until consumption: a ready result must still be
/// rejected if restore, closing or disabling occurred before the next poll.
pub(super) fn take_result(id: u32, result: FileResult) -> FileResult {
    let pending = FILE_TASKS.with(|slot| slot.borrow_mut().pending.remove(&id));
    let pending = pending.ok_or("The file selection is no longer available.")?;
    pending.valid().and(result)
}

/// Resolve exactly one known request. Unknown, dropped or already-completed
/// identifiers return false and never allocate application state.
pub fn complete_file(id: u32, result: FileResult) -> bool {
    let completed = FILE_TASKS.with(|slot| {
        let mut registry = slot.borrow_mut();
        let pending = registry.pending.get_mut(&id)?;
        if pending.completed {
            return None;
        }
        let result = pending
            .valid()
            .and_then(|_| validation::result(result, &pending.options));
        pending.completed = true;
        Some((pending.completion.clone(), result))
    });
    let Some((completion, result)) = completed else {
        return false;
    };
    let waker = {
        let mut completion = completion.borrow_mut();
        completion.result = Some(result);
        completion.waker.take()
    };
    if let Some(waker) = waker {
        waker.wake();
    }
    true
}

/// Host adapters can avoid reading bytes for an already abandoned selection.
pub fn is_file_task_pending(id: u32) -> bool {
    let status = FILE_TASKS.with(|slot| {
        slot.borrow()
            .pending
            .get(&id)
            .filter(|pending| !pending.completed)
            .map(Pending::valid)
    });
    match status {
        Some(Ok(())) => true,
        Some(Err(error)) => {
            complete_file(id, Err(error));
            false
        }
        None => false,
    }
}

pub fn cancel_for_app(session: &Session, app: &str) {
    super::http::cancel(session, Some(app));
    cancel(
        session,
        Some(app),
        "File selection was cancelled because the app closed.",
    );
}
pub fn cancel_for_session(session: &Session) {
    super::http::cancel(session, None);
    cancel(
        session,
        None,
        "File selection was cancelled because the phone session changed.",
    );
}
fn cancel(session: &Session, app: Option<&str>, message: &str) {
    let ids: Vec<_> = FILE_TASKS.with(|slot| {
        slot.borrow_mut()
            .pending
            .iter_mut()
            .filter(|(_, pending)| {
                std::ptr::eq(pending.session.as_ptr(), session)
                    && app.is_none_or(|id| id == pending.app)
            })
            .map(|(id, pending)| {
                pending.cancelled.get_or_insert_with(|| message.into());
                *id
            })
            .collect()
    });
    for id in ids {
        complete_file(id, Err(message.into()));
    }
}
