use super::{registry, FilePickerOptions, FileResult};
use crate::AppContext;
use phone_host::HostRequest;
use std::{
    cell::RefCell,
    future::Future,
    pin::Pin,
    rc::Rc,
    task::{Context, Poll, Waker},
};

#[derive(Default)]
pub(super) struct Completion {
    pub result: Option<FileResult>,
    pub waker: Option<Waker>,
}

/// Dropping this future abandons its request. The host may still finish a
/// platform picker, but the obsolete result cannot reach an application.
#[must_use = "Dropping the file task cancels delivery of its selected file."]
pub struct FileTask {
    id: Option<u32>,
    state: Rc<RefCell<Completion>>,
    consumed: bool,
}

impl FileTask {
    fn error(message: String) -> Self {
        Self {
            id: None,
            state: Rc::new(RefCell::new(Completion {
                result: Some(Err(message)),
                waker: None,
            })),
            consumed: false,
        }
    }
}

/// Create from a synchronous click callback; move the returned task into the
/// dispatcher future only after this function has requested the host picker.
pub fn choose_file(context: &AppContext, options: FilePickerOptions) -> FileTask {
    if let Err(error) =
        phone_host::file_picker::validate_options(&options.extensions, options.max_bytes)
    {
        return FileTask::error(error);
    }
    let state = Rc::new(RefCell::new(Completion::default()));
    let id = match registry::insert(context, options.clone(), state.clone()) {
        Ok(id) => id,
        Err(error) => return FileTask::error(error),
    };
    let task = FileTask {
        id: Some(id),
        state,
        consumed: false,
    };
    if let Err(error) = context.request(HostRequest::ChooseFile {
        request_id: id,
        extensions: options.extensions,
        max_bytes: options.max_bytes,
    }) {
        registry::complete_file(id, Err(error));
    }
    task
}

impl Future for FileTask {
    type Output = FileResult;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.get_mut();
        assert!(
            !this.consumed,
            "a completed file task cannot be polled again"
        );
        if let Some(id) = this.id {
            registry::is_file_task_pending(id);
        }
        let result = this.state.borrow_mut().result.take();
        if let Some(result) = result {
            this.consumed = true;
            Poll::Ready(match this.id.take() {
                Some(id) => registry::take_result(id, result),
                None => result,
            })
        } else {
            let mut state = this.state.borrow_mut();
            if state
                .waker
                .as_ref()
                .is_none_or(|waker| !waker.will_wake(cx.waker()))
            {
                state.waker = Some(cx.waker().clone());
            }
            Poll::Pending
        }
    }
}

impl Drop for FileTask {
    fn drop(&mut self) {
        if let Some(id) = self.id.take() {
            registry::abandon(id);
        }
    }
}
