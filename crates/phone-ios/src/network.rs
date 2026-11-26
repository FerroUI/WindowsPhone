//! Lifecycle-owned observations of an available network path. This is not an
//! Internet reachability probe: no connection is opened and no data is sent.
use std::{
    cell::{Cell, RefCell},
    collections::BTreeMap,
    rc::{Rc, Weak},
    sync::atomic::{AtomicU64, Ordering},
};

type Observation = Cell<Option<bool>>;
thread_local! {
    static OBSERVATIONS: RefCell<BTreeMap<u64, Weak<Observation>>> = const { RefCell::new(BTreeMap::new()) };
}
static NEXT_ID: AtomicU64 = AtomicU64::new(1);

/// Each resume receives a new identity. Queued updates from an older monitor
/// cannot populate the new observation or retain an embedding host after drop.
#[derive(Default)]
pub struct NetworkMonitor {
    identifier: Cell<Option<u64>>,
    observed: Rc<Observation>,
}

impl NetworkMonitor {
    pub fn online(&self) -> Option<bool> {
        self.observed.get()
    }

    pub fn set_enabled(&self, enabled: bool) {
        if !enabled {
            self.stop();
            return;
        }
        if self.identifier.get().is_some() {
            return;
        }
        self.observed.set(None);
        let Ok(identifier) =
            NEXT_ID.try_update(Ordering::Relaxed, Ordering::Relaxed, |id| id.checked_add(1))
        else {
            return;
        };
        let registered = OBSERVATIONS.with(|observations| {
            let mut observations = observations.borrow_mut();
            if observations.len() >= 16 {
                return false;
            }
            observations.insert(identifier, Rc::downgrade(&self.observed));
            true
        });
        if !registered {
            return;
        }
        self.identifier.set(Some(identifier));
        if !start_native(identifier) {
            self.stop();
        }
    }

    fn stop(&self) {
        if let Some(identifier) = self.identifier.take() {
            // Revoke Rust ownership before native cancellation, whose queued
            // callbacks are allowed to complete asynchronously.
            OBSERVATIONS.with(|observations| observations.borrow_mut().remove(&identifier));
            stop_native(identifier);
        }
        self.observed.set(None);
    }
}

impl Drop for NetworkMonitor {
    fn drop(&mut self) {
        self.stop();
    }
}

#[cfg(any(target_os = "ios", test))]
fn observe(identifier: u64, status: i32) -> bool {
    let observed = OBSERVATIONS.with(|observations| {
        observations
            .borrow()
            .get(&identifier)
            .and_then(Weak::upgrade)
    });
    let Some(observed) = observed else {
        return false;
    };
    observed.set(match status {
        1 => Some(true),
        0 => Some(false),
        _ => None,
    });
    true
}

#[cfg(target_os = "ios")]
unsafe extern "C" fn completed(identifier: u64, status: i32) {
    // Network.framework delivers this on the main queue selected by our native
    // monitor. Only integer IDs cross the ABI; no Rust/UI pointer escapes.
    observe(identifier, status);
}

fn start_native(identifier: u64) -> bool {
    #[cfg(target_os = "ios")]
    {
        unsafe extern "C" {
            fn wp_ios_network_start(
                identifier: u64,
                callback: unsafe extern "C" fn(u64, i32),
            ) -> i32;
        }
        unsafe { wp_ios_network_start(identifier, completed) == 1 }
    }
    #[cfg(not(target_os = "ios"))]
    {
        let _ = identifier;
        // Portable tests drive the actual ownership and completion path, with
        // only Network.framework construction replaced by an inert transport.
        cfg!(test)
    }
}

fn stop_native(identifier: u64) {
    #[cfg(target_os = "ios")]
    unsafe {
        unsafe extern "C" {
            fn wp_ios_network_cancel(identifier: u64);
        }
        wp_ios_network_cancel(identifier);
    }
    #[cfg(not(target_os = "ios"))]
    let _ = identifier;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resume_observes_a_fresh_path_and_rejects_updates_after_suspension() {
        let monitor = NetworkMonitor::default();
        assert_eq!(monitor.online(), None);
        monitor.set_enabled(true);
        let first = monitor.identifier.get().unwrap();
        assert_eq!(monitor.online(), None);
        assert!(observe(first, 1));
        assert_eq!(monitor.online(), Some(true));
        monitor.set_enabled(true);
        assert_eq!(monitor.identifier.get(), Some(first));
        assert_eq!(monitor.online(), Some(true));
        monitor.set_enabled(false);
        assert_eq!(monitor.online(), None);
        assert!(!observe(first, 1));
        monitor.set_enabled(true);
        let resumed = monitor.identifier.get().unwrap();
        assert_ne!(first, resumed);
        assert_eq!(monitor.online(), None);
        assert!(!observe(first, 1));
        assert!(observe(resumed, 0));
        assert_eq!(monitor.online(), Some(false));
        assert!(observe(resumed, -1));
        assert_eq!(monitor.online(), None);
    }

    #[test]
    fn dropped_monitor_cannot_repopulate_or_cancel_another_owner() {
        let first = NetworkMonitor::default();
        let second = NetworkMonitor::default();
        first.set_enabled(true);
        second.set_enabled(true);
        let old = first.identifier.get().unwrap();
        let current = second.identifier.get().unwrap();
        assert!(observe(current, 1));
        drop(first);
        assert!(!observe(old, 0));
        assert_eq!(second.online(), Some(true));
        assert!(observe(current, 0));
        assert_eq!(second.online(), Some(false));
        second.set_enabled(false);
        second.set_enabled(false);
        assert!(!observe(current, 1));
        assert_eq!(second.online(), None);
    }

    #[test]
    fn monitor_capacity_is_bounded_and_drop_releases_its_slot() {
        let mut owners = (0..16)
            .map(|_| NetworkMonitor::default())
            .collect::<Vec<_>>();
        for owner in &owners {
            owner.set_enabled(true);
            assert!(owner.identifier.get().is_some());
        }
        let waiting = NetworkMonitor::default();
        waiting.set_enabled(true);
        assert_eq!(waiting.identifier.get(), None);
        assert_eq!(waiting.online(), None);
        let closed = owners.pop().unwrap();
        let old = closed.identifier.get().unwrap();
        drop(closed);
        waiting.set_enabled(true);
        let current = waiting.identifier.get().unwrap();
        assert_ne!(current, old);
        assert!(!observe(old, 1));
        assert!(observe(current, 0));
        assert_eq!(waiting.online(), Some(false));
    }
}
