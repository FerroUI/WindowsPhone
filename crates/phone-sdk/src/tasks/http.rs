//! Credential-free HTTPS tasks scoped to a registered app and document epoch.
use crate::{session::Session, AppContext};
use phone_state::{HostRequest, PhoneViewModel};
use std::{
    cell::RefCell,
    collections::BTreeMap,
    future::Future,
    pin::Pin,
    rc::{Rc, Weak},
    task::{Context, Poll, Waker},
};

pub type HttpResult = Result<Vec<u8>, String>;
pub const MAX_PENDING_HTTP_TASKS: usize = 8;
struct Pending {
    session: Weak<Session>,
    vm: Weak<PhoneViewModel>,
    app: &'static str,
    epoch: u64,
    max_bytes: usize,
    result: Option<HttpResult>,
    waker: Option<Waker>,
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
            return Err("The network request was cancelled because phone data changed.".into());
        }
        let registry = session.registry();
        let entry = registry
            .by_id(self.app)
            .ok_or("The app is no longer registered.")?;
        if entry.manifest.builtin.map_or_else(
            || vm.extension_enabled(self.app),
            |app| vm.data().installed.contains(&app),
        ) {
            Ok(())
        } else {
            Err("The app is disabled or unavailable.".into())
        }
    }
}
struct Registry {
    next: Option<u32>,
    pending: BTreeMap<u32, Pending>,
}
ferroui_base::ferro_thread_local! {
    static REQUESTS: RefCell<Registry> = RefCell::new(Registry { next: Some(1), pending: BTreeMap::new() });
}

#[must_use = "Dropping the task cancels delivery of the response."]
pub struct HttpTask {
    id: Option<u32>,
    error: Option<String>,
    consumed: bool,
}
impl HttpTask {
    fn error(error: String) -> Self {
        Self {
            id: None,
            error: Some(error),
            consumed: false,
        }
    }
}
pub fn get(context: &AppContext, url: &str, max_bytes: u32) -> HttpTask {
    let url = match phone_host::http::validate_request(url, max_bytes) {
        Ok(url) => url,
        Err(error) => return HttpTask::error(error),
    };
    start(context, max_bytes, move |request_id| HostRequest::HttpGet {
        request_id,
        url,
        max_bytes,
    })
}
/// Loads a visible map tile using the host's bounded persistent cache. This is
/// intentionally separate from uncached feed/account-independent HTTPS reads.
pub fn map_tile(context: &AppContext, zoom: u8, x: u32, y: u32) -> HttpTask {
    if let Err(error) = phone_host::maps::validate(zoom, x, y) {
        return HttpTask::error(error);
    }
    start(
        context,
        phone_host::maps::MAX_TILE_BYTES,
        move |request_id| HostRequest::MapTile {
            request_id,
            zoom,
            x,
            y,
        },
    )
}
fn start(
    context: &AppContext,
    max_bytes: u32,
    request: impl FnOnce(u32) -> HostRequest,
) -> HttpTask {
    let pending = Pending {
        session: Rc::downgrade(&context.session),
        vm: Rc::downgrade(&context.vm),
        app: context.id,
        epoch: context.vm.document_epoch(),
        max_bytes: max_bytes as usize,
        result: None,
        waker: None,
        completed: false,
        cancelled: None,
    };
    if let Err(error) = pending.valid() {
        return HttpTask::error(error);
    }
    let id = REQUESTS.with(|slot| {
        let mut registry = slot.borrow_mut();
        if registry.pending.len() >= MAX_PENDING_HTTP_TASKS {
            return Err("Eight network requests are already pending.".into());
        }
        let id = registry
            .next
            .ok_or("The network request identifier space is exhausted.")?;
        registry.next = id.checked_add(1);
        registry.pending.insert(id, pending);
        Ok(id)
    });
    let id = match id {
        Ok(id) => id,
        Err(error) => return HttpTask::error(error),
    };
    if let Err(error) = context.request(request(id)) {
        complete_http(id, Err(error));
    }
    HttpTask {
        id: Some(id),
        error: None,
        consumed: false,
    }
}

pub fn complete_http(id: u32, result: HttpResult) -> bool {
    let wake = REQUESTS.with(|slot| {
        let mut registry = slot.borrow_mut();
        let pending = registry.pending.get_mut(&id)?;
        if pending.completed {
            return None;
        }
        pending.result = Some(pending.valid().and_then(|_| result).and_then(|bytes| {
            if bytes.len() <= pending.max_bytes {
                Ok(bytes)
            } else {
                Err("The response exceeds this request's size limit.".into())
            }
        }));
        pending.completed = true;
        Some(pending.waker.take())
    });
    match wake {
        Some(waker) => {
            if let Some(waker) = waker {
                waker.wake();
            }
            true
        }
        None => false,
    }
}
pub fn is_http_task_pending(id: u32) -> bool {
    let valid = REQUESTS.with(|slot| {
        slot.borrow()
            .pending
            .get(&id)
            .filter(|p| !p.completed)
            .map(Pending::valid)
    });
    match valid {
        Some(Ok(())) => true,
        Some(Err(error)) => {
            complete_http(id, Err(error));
            false
        }
        None => false,
    }
}
pub(super) fn cancel(session: &Session, app: Option<&str>) {
    let ids = REQUESTS.with(|slot| {
        slot.borrow_mut()
            .pending
            .iter_mut()
            .filter(|(_, p)| {
                std::ptr::eq(p.session.as_ptr(), session) && app.is_none_or(|app| p.app == app)
            })
            .map(|(id, pending)| {
                pending.cancelled = Some(
                    "The network request was cancelled because the app session changed.".into(),
                );
                *id
            })
            .collect::<Vec<_>>()
    });
    for id in ids {
        complete_http(id, Err("The network request was cancelled.".into()));
    }
}
impl Future for HttpTask {
    type Output = HttpResult;
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<HttpResult> {
        let this = self.get_mut();
        assert!(
            !this.consumed,
            "a completed HTTP task cannot be polled again"
        );
        if let Some(error) = this.error.take() {
            this.consumed = true;
            return Poll::Ready(Err(error));
        }
        let Some(id) = this.id else { unreachable!() };
        is_http_task_pending(id);
        let result = REQUESTS.with(|slot| {
            let mut registry = slot.borrow_mut();
            let pending = registry
                .pending
                .get_mut(&id)
                .expect("live HTTP task registry entry");
            if pending.completed {
                let mut pending = registry.pending.remove(&id).unwrap();
                Some(pending.valid().and_then(|_| pending.result.take().unwrap()))
            } else {
                if pending
                    .waker
                    .as_ref()
                    .is_none_or(|w| !w.will_wake(cx.waker()))
                {
                    pending.waker = Some(cx.waker().clone());
                }
                None
            }
        });
        match result {
            Some(result) => {
                this.id = None;
                this.consumed = true;
                Poll::Ready(result)
            }
            None => Poll::Pending,
        }
    }
}
impl Drop for HttpTask {
    fn drop(&mut self) {
        if let Some(id) = self.id {
            REQUESTS.with(|slot| slot.borrow_mut().pending.remove(&id));
        }
    }
}

#[cfg(test)]
#[path = "http_tests.rs"]
mod tests;
