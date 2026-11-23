//! Bounded native completions. Objective-C callbacks hold identifiers, never UI pointers.
use std::{
    cell::{Cell, RefCell},
    collections::BTreeMap,
    future::Future,
    pin::Pin,
    rc::{Rc, Weak},
    sync::{
        Arc, Mutex, OnceLock,
        atomic::{AtomicU64, Ordering},
    },
    task::{Context, Poll, Waker},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Settings,
    Share,
    Permission,
    Notify,
    Location,
    Device,
    DeviceRead,
}

#[derive(Debug)]
pub struct Reply {
    pub code: i32,
    pub first: f64,
    pub second: f64,
    pub message: String,
}

#[derive(Default)]
struct Completion {
    value: Option<Result<Reply, String>>,
    waker: Option<Waker>,
}

type Shared = Arc<Mutex<Completion>>;
fn registry() -> &'static Mutex<BTreeMap<u64, Shared>> {
    static REGISTRY: OnceLock<Mutex<BTreeMap<u64, Shared>>> = OnceLock::new();
    REGISTRY.get_or_init(Default::default)
}
static NEXT_ID: AtomicU64 = AtomicU64::new(1);

fn settle(shared: Shared, result: Result<Reply, String>) {
    let waker = {
        let mut state = shared.lock().unwrap_or_else(|error| error.into_inner());
        state.value = Some(result);
        state.waker.take()
    };
    if let Some(waker) = waker {
        waker.wake();
    }
}

/// Removes ownership before waking a continuation, so reentrant dispatch is safe.
pub fn complete(id: u64, result: Result<Reply, String>) -> bool {
    complete_with(id, result, || {})
}

pub fn complete_with(id: u64, result: Result<Reply, String>, before_wake: impl FnOnce()) -> bool {
    let shared = registry()
        .lock()
        .unwrap_or_else(|error| error.into_inner())
        .remove(&id);
    if let Some(shared) = shared {
        before_wake();
        settle(shared, result);
        true
    } else {
        false
    }
}

#[derive(Default)]
pub struct Session {
    requests: RefCell<BTreeMap<u64, Kind>>,
    closed: Cell<bool>,
}

impl Session {
    pub fn start(self: &Rc<Self>, kind: Kind) -> Result<Pending, String> {
        if self.closed.get() {
            return Err("The iOS host is closed.".into());
        }
        if self.requests.borrow().len() >= 16 {
            return Err("Too many iOS operations are pending.".into());
        }
        let id = NEXT_ID
            .try_update(Ordering::Relaxed, Ordering::Relaxed, |id| id.checked_add(1))
            .map_err(|_| "The native operation identifier space is exhausted.")?;
        let shared = Arc::new(Mutex::new(Completion::default()));
        {
            let mut active = registry().lock().unwrap_or_else(|error| error.into_inner());
            if active.len() >= 16 {
                return Err("Too many iOS operations are pending.".into());
            }
            active.insert(id, shared.clone());
        }
        self.requests.borrow_mut().insert(id, kind);
        Ok(Pending {
            id,
            shared,
            owner: Rc::downgrade(self),
            done: false,
        })
    }

    pub fn cancel_locations(&self) {
        self.cancel_where(|kind| kind == Kind::Location);
    }

    pub fn cancel_device_reads(&self) {
        self.cancel_where(|kind| kind == Kind::DeviceRead);
    }

    pub fn close(&self) {
        self.closed.set(true);
        self.cancel_where(|_| true);
    }

    fn cancel_where(&self, predicate: impl Fn(Kind) -> bool) {
        let ids: Vec<_> = self
            .requests
            .borrow()
            .iter()
            .filter_map(|(&id, &kind)| predicate(kind).then_some(id))
            .collect();
        for id in ids {
            self.requests.borrow_mut().remove(&id);
            let shared = registry()
                .lock()
                .unwrap_or_else(|error| error.into_inner())
                .remove(&id);
            crate::native::cancel(id);
            if let Some(shared) = shared {
                settle(shared, Err("The iOS operation was cancelled.".into()));
            }
        }
    }
}

impl Drop for Session {
    fn drop(&mut self) {
        self.close();
    }
}

pub struct Pending {
    pub id: u64,
    shared: Shared,
    owner: Weak<Session>,
    done: bool,
}

impl Future for Pending {
    type Output = Result<Reply, String>;
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.get_mut();
        let result = {
            let mut state = this
                .shared
                .lock()
                .unwrap_or_else(|error| error.into_inner());
            if let Some(result) = state.value.take() {
                Some(result)
            } else {
                if state
                    .waker
                    .as_ref()
                    .is_none_or(|waker| !waker.will_wake(cx.waker()))
                {
                    state.waker = Some(cx.waker().clone());
                }
                None
            }
        };
        match result {
            Some(result) => {
                this.done = true;
                if let Some(owner) = this.owner.upgrade() {
                    owner.requests.borrow_mut().remove(&this.id);
                }
                Poll::Ready(result)
            }
            None => Poll::Pending,
        }
    }
}

impl Drop for Pending {
    fn drop(&mut self) {
        if let Some(owner) = self.owner.upgrade() {
            owner.requests.borrow_mut().remove(&self.id);
        }
        if !self.done {
            registry()
                .lock()
                .unwrap_or_else(|error| error.into_inner())
                .remove(&self.id);
            crate::native::cancel(self.id);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::task::Wake;
    struct Counter(AtomicU64);
    impl Wake for Counter {
        fn wake(self: Arc<Self>) {
            self.0.fetch_add(1, Ordering::Relaxed);
        }
    }
    #[test]
    fn completion_is_one_shot_and_cancelled_identifiers_cannot_deliver() {
        let session = Rc::new(Session::default());
        let mut task = Box::pin(session.start(Kind::Location).unwrap());
        let id = task.id;
        let counter = Arc::new(Counter(AtomicU64::new(0)));
        let waker = Waker::from(counter.clone());
        let mut cx = Context::from_waker(&waker);
        assert!(task.as_mut().poll(&mut cx).is_pending());
        assert!(complete(
            id,
            Ok(Reply {
                code: 3,
                first: 50.0,
                second: 18.0,
                message: String::new()
            })
        ));
        assert_eq!(counter.0.load(Ordering::Relaxed), 1);
        assert!(!complete(id, Err("duplicate".into())));
        let Poll::Ready(Ok(value)) = task.as_mut().poll(&mut cx) else {
            panic!("missing completion")
        };
        assert_eq!((value.first, value.second), (50.0, 18.0));
        let task = session.start(Kind::Location).unwrap();
        let id = task.id;
        drop(task);
        assert!(!complete(id, Err("late".into())));
        assert!(session.requests.borrow().is_empty());
    }

    #[test]
    fn suspension_cancels_location_and_closing_cancels_every_remaining_operation() {
        let session = Rc::new(Session::default());
        let location = session.start(Kind::Location).unwrap();
        let notification = session.start(Kind::Permission).unwrap();
        session.cancel_locations();
        assert!(!complete(location.id, Err("late location".into())));
        assert!(session.requests.borrow().contains_key(&notification.id));
        session.close();
        assert!(!complete(notification.id, Err("late permission".into())));
        assert!(session.start(Kind::Share).is_err());
    }
}
