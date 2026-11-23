//! A bounded worker for native file/codec work. Futures resume on their caller's executor.
use std::{
    future::Future,
    pin::Pin,
    sync::{
        Arc, Mutex, OnceLock,
        atomic::{AtomicBool, Ordering},
        mpsc::{SyncSender, sync_channel},
    },
    task::{Context, Poll, Waker},
};

type Job = Box<dyn FnOnce() + Send>;
static WORKER: OnceLock<Result<SyncSender<Job>, String>> = OnceLock::new();

struct Completion<T> {
    result: Option<Result<T, String>>,
    waker: Option<Waker>,
}

pub struct Task<T> {
    completion: Arc<Mutex<Completion<T>>>,
    cancelled: Arc<AtomicBool>,
}

pub fn run<T: Send + 'static>(
    operation: impl FnOnce() -> Result<T, String> + Send + 'static,
) -> Task<T> {
    let completion = Arc::new(Mutex::new(Completion {
        result: None,
        waker: None,
    }));
    let cancelled = Arc::new(AtomicBool::new(false));
    let task = Task {
        completion: completion.clone(),
        cancelled: cancelled.clone(),
    };
    let job = Box::new(move || {
        if cancelled.load(Ordering::Acquire) {
            return;
        }
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(operation))
            .unwrap_or_else(|_| Err("The native file operation could not finish.".into()));
        if cancelled.load(Ordering::Acquire) {
            return;
        }
        complete(&completion, result);
    });
    let sender = WORKER.get_or_init(|| {
        let (sender, receiver) = sync_channel::<Job>(32);
        std::thread::Builder::new()
            .name("phone-file-services".into())
            .spawn(move || {
                while let Ok(job) = receiver.recv() {
                    job();
                }
            })
            .map_err(|error| error.to_string())?;
        Ok(sender)
    });
    let error = match sender {
        Ok(sender) => sender
            .try_send(job)
            .err()
            .map(|_| "Native file services are busy. Try again shortly.".to_owned()),
        Err(error) => Some(error.clone()),
    };
    if let Some(error) = error {
        complete(&task.completion, Err(error));
    }
    task
}

fn complete<T>(completion: &Mutex<Completion<T>>, result: Result<T, String>) {
    let waker = {
        let mut state = completion.lock().unwrap_or_else(|error| error.into_inner());
        state.result = Some(result);
        state.waker.take()
    };
    if let Some(waker) = waker {
        waker.wake();
    }
}

impl<T> Future for Task<T> {
    type Output = Result<T, String>;
    fn poll(self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<Self::Output> {
        let mut state = self
            .completion
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        if let Some(result) = state.result.take() {
            return Poll::Ready(result);
        }
        if state
            .waker
            .as_ref()
            .is_none_or(|waker| !waker.will_wake(context.waker()))
        {
            state.waker = Some(context.waker().clone());
        }
        Poll::Pending
    }
}

impl<T> Drop for Task<T> {
    fn drop(&mut self) {
        self.cancelled.store(true, Ordering::Release);
    }
}

#[cfg(test)]
mod tests;
