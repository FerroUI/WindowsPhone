use phone_host::platform::{PlatformResponse, PlatformTask};
use std::{
    cell::{Cell, RefCell},
    collections::BTreeMap,
    future::Future,
    pin::Pin,
    rc::{Rc, Weak},
    task::{Context, Poll, Waker},
};
#[derive(Default)]
pub(crate) struct Gate {
    closed: Cell<bool>,
    next: Cell<u64>,
    pending: RefCell<BTreeMap<u64, Option<Waker>>>,
}
impl Gate {
    pub fn reserve(self: &Rc<Self>) -> Result<Reservation, String> {
        if self.closed.get() {
            return Err("The host is closed.".into());
        }
        if self.pending.borrow().len() >= 8 {
            return Err("Too many platform operations are in progress.".into());
        }
        let id = self
            .next
            .get()
            .checked_add(1)
            .ok_or("Platform request identities exhausted.")?;
        self.next.set(id);
        self.pending.borrow_mut().insert(id, None);
        Ok(Reservation {
            id,
            gate: Rc::downgrade(self),
        })
    }
    pub fn close(&self) {
        self.closed.set(true);
        let pending = std::mem::take(&mut *self.pending.borrow_mut());
        for waker in pending.into_values().flatten() {
            waker.wake();
        }
    }
}
pub(crate) struct Reservation {
    id: u64,
    gate: Weak<Gate>,
}
impl Reservation {
    pub fn cancellation(&self) -> Cancellation {
        Cancellation {
            id: self.id,
            gate: self.gate.clone(),
        }
    }
    pub fn bind(self, future: PlatformTask) -> Pending {
        Pending {
            future,
            reservation: self,
        }
    }
}
pub(crate) struct Cancellation {
    id: u64,
    gate: Weak<Gate>,
}
impl Cancellation {
    pub fn cancel(self) {
        if let Some(gate) = self.gate.upgrade() {
            // Release the RefCell before waking: dispatchers may poll inline.
            let pending = gate.pending.borrow_mut().remove(&self.id);
            if let Some(Some(waker)) = pending {
                waker.wake();
            }
        }
    }
}
impl Drop for Reservation {
    fn drop(&mut self) {
        if let Some(gate) = self.gate.upgrade() {
            gate.pending.borrow_mut().remove(&self.id);
        }
    }
}
pub(crate) struct Pending {
    future: PlatformTask,
    reservation: Reservation,
}
impl Future for Pending {
    type Output = Result<PlatformResponse, String>;
    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let Some(gate) = self.reservation.gate.upgrade().filter(|gate| {
            !gate.closed.get() && gate.pending.borrow().contains_key(&self.reservation.id)
        }) else {
            return Poll::Ready(Err("The platform operation was cancelled.".into()));
        };
        gate.pending
            .borrow_mut()
            .insert(self.reservation.id, Some(cx.waker().clone()));
        self.future.as_mut().poll(cx)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use std::task::Wake;
    struct Noop;
    impl Wake for Noop {
        fn wake(self: std::sync::Arc<Self>) {}
    }
    #[test]
    fn closing_cancels_before_polling_the_provider() {
        let gate = Rc::new(Gate::default());
        let used = Rc::new(Cell::new(false));
        let mark = used.clone();
        let mut task = Box::pin(gate.reserve().unwrap().bind(Box::pin(async move {
            mark.set(true);
            Ok(PlatformResponse::Accepted {
                message: String::new(),
            })
        })));
        gate.close();
        let waker = Waker::from(std::sync::Arc::new(Noop));
        let mut context = Context::from_waker(&waker);
        assert!(matches!(
            task.as_mut().poll(&mut context),
            Poll::Ready(Err(_))
        ));
        assert!(!used.get());
    }
    #[test]
    fn pending_operations_are_bounded_and_dropping_releases_capacity() {
        let gate = Rc::new(Gate::default());
        let mut requests = (0..8).map(|_| gate.reserve().unwrap()).collect::<Vec<_>>();
        assert!(gate.reserve().is_err());
        requests.pop();
        assert!(gate.reserve().is_ok());
    }
}
