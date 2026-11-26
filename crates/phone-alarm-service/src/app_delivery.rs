use super::{AlarmState, Alert};
use phone_sdk::{session::Session, AppContext, AppRegistry};
use phone_state::{HostRequest, PhoneViewModel};
use std::{
    cell::Cell,
    rc::{Rc, Weak},
};

struct CollectionGuard<'a>(&'a Cell<bool>);
impl Drop for CollectionGuard<'_> {
    fn drop(&mut self) {
        self.0.set(false);
    }
}

impl AlarmState {
    /// The registry owns provider code. Keep only a weak reference so an old
    /// retained alert cannot keep a disposed application session alive.
    pub fn tick_with_registry(&self, vm: &PhoneViewModel, registry: &Rc<AppRegistry>) {
        let weak = Rc::downgrade(registry);
        if !Weak::ptr_eq(&weak, &self.app_registry.borrow()) {
            *self.app_registry.borrow_mut() = weak;
            self.app_reminders.borrow_mut().clear();
        }
        self.tick(vm);
    }

    pub(super) fn refresh_app_reminders(&self, vm: &PhoneViewModel) -> bool {
        if self.refreshing_app_reminders.get() {
            return false;
        }
        let tag = (vm.document_epoch(), vm.revision());
        if self.app_reminders.borrow().tag == Some(tag) {
            return true;
        }
        let binding = self.app_registry.borrow().clone();
        let registry = binding.upgrade();
        let Some(registry) = registry else {
            self.app_reminders.borrow_mut().clear();
            return true;
        };
        self.refreshing_app_reminders.set(true);
        let _guard = CollectionGuard(&self.refreshing_app_reminders);
        // No schedule or queue borrow survives a provider call. A provider is
        // required to be read-only; epoch/revision checks reject reentrant work.
        let snapshot = registry.reminder_snapshot(vm);
        // Provider callbacks can synchronously close/suspend the host or bind a
        // replacement registry without mutating PhoneData. Never cache the old
        // provider's result under that replacement source's revision tag.
        if !self.can_deliver() || !Weak::ptr_eq(&binding, &self.app_registry.borrow()) {
            return false;
        }
        if (snapshot.document_epoch, snapshot.document_revision)
            != (vm.document_epoch(), vm.revision())
        {
            self.app_reminders.borrow_mut().clear();
            self.queue
                .borrow_mut()
                .retain(|item| !matches!(item, Alert::App(_)));
            if matches!(self.active.get_untracked(), Some(Alert::App(_))) {
                self.active.set(None);
                if self.last_audio.take().is_some_and(|audio| audio.0) {
                    vm.request(HostRequest::AlarmAlert {
                        active: false,
                        volume: 0,
                        vibration: false,
                    });
                }
            }
            return false;
        }
        self.app_reminders.borrow_mut().replace(snapshot);
        true
    }

    fn current_alert(&self, vm: &PhoneViewModel, item: &Alert) -> Option<Alert> {
        match item {
            Alert::App(reminder) => self
                .app_reminders
                .borrow()
                .current(reminder)
                .map(Alert::App),
            _ => item.current(vm).then(|| item.clone()),
        }
    }

    pub(super) fn reconcile_alerts(&self, vm: &PhoneViewModel) {
        let previous = self.active.get_untracked();
        let current = previous
            .as_ref()
            .and_then(|item| self.current_alert(vm, item));
        self.queue.borrow_mut().retain_mut(|item| {
            if let Some(current) = self.current_alert(vm, item) {
                *item = current;
                true
            } else {
                false
            }
        });
        if previous != current {
            self.active.set(current);
        }
    }

    pub(super) fn open_app_reminder(
        &self,
        vm: &Rc<PhoneViewModel>,
        ui: &Rc<Session>,
        item: &phone_sdk::ScopedAppReminder,
    ) {
        let registry = ui.registry();
        let epoch = item.document_epoch;
        if !self.owns_open_context(vm, ui, &registry, epoch)
            || !self
                .active
                .get_untracked()
                .is_some_and(|active| active.key() == Alert::App(item.clone()).key())
        {
            return;
        }
        let current = self.current_open_item(vm, ui, &registry, epoch, item);
        let Some(current) = current else {
            if self.owns_open_context(vm, ui, &registry, epoch) {
                self.dismiss(vm);
                if self.owns_open_context(vm, ui, &registry, epoch) {
                    vm.set_status("This reminder is no longer current.".into());
                }
            }
            return;
        };
        if !self
            .active
            .get_untracked()
            .is_some_and(|active| active.key() == Alert::App(item.clone()).key())
        {
            return;
        }
        self.dismiss(vm);
        // Dismissal publishes native alert state and can dispatch host audio or
        // notifications. Those callbacks may replace the document/session or
        // retire this host before control returns to this click handler.
        let Some(current) = self.current_open_item(vm, ui, &registry, epoch, &current) else {
            return;
        };
        ui.locked.set(false);
        // Unlock itself publishes a signal. Keep the same source boundary
        // through that reconciliation too, before the typed launch parser runs.
        let Some(current) = self.current_open_item(vm, ui, &registry, epoch, &current) else {
            return;
        };
        AppContext {
            vm: vm.clone(),
            session: ui.clone(),
            id: current.app_id,
        }
        .navigate(&current.reminder.path);
    }

    fn owns_open_context(
        &self,
        vm: &PhoneViewModel,
        ui: &Rc<Session>,
        registry: &Rc<AppRegistry>,
        epoch: u64,
    ) -> bool {
        self.can_deliver()
            && vm.document_epoch() == epoch
            && phone_sdk::current_session().is_some_and(|current| Rc::ptr_eq(&current, ui))
            && Rc::ptr_eq(&ui.registry(), registry)
            && Weak::ptr_eq(&self.app_registry.borrow(), &Rc::downgrade(registry))
            && std::ptr::eq(self, ui.state::<AlarmState>().as_ref())
    }

    fn current_open_item(
        &self,
        vm: &PhoneViewModel,
        ui: &Rc<Session>,
        registry: &Rc<AppRegistry>,
        epoch: u64,
        item: &phone_sdk::ScopedAppReminder,
    ) -> Option<phone_sdk::ScopedAppReminder> {
        if !self.owns_open_context(vm, ui, registry, epoch)
            || !self.refresh_app_reminders(vm)
            || !self.owns_open_context(vm, ui, registry, epoch)
        {
            return None;
        }
        self.app_reminders.borrow().current(item)
    }
}

#[cfg(test)]
mod tests;
