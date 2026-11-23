use super::AlarmState;
use phone_state::{HostRequest, PhoneViewModel};

impl AlarmState {
    pub(super) fn can_deliver(&self) -> bool {
        self.host_visible.get() && !self.closed.get()
    }

    /// Suspend presentation without discarding active, queued or snoozed alerts.
    /// Resume defers audio until a normal tick validates the current document.
    pub fn host_visibility(&self, vm: &PhoneViewModel, visible: bool) {
        if self.closed.get() {
            return;
        }
        if visible {
            self.host_visible.set(true);
        } else {
            self.stop_for_host(vm);
        }
    }

    /// Stop while the host broker is available, then reject stale delivery work.
    pub fn host_closing(&self, vm: &PhoneViewModel) {
        if self.closed.replace(true) {
            return;
        }
        self.stop_for_host(vm);
        self.clear();
    }

    fn stop_for_host(&self, vm: &PhoneViewModel) {
        let was_visible = self.host_visible.replace(false);
        let was_active = self.last_audio.take().is_some_and(|audio| audio.0);
        // Invalidate before dispatch: the stop may synchronously trigger host work.
        // A repeated suspend/close need not start another native stop operation.
        if was_visible || was_active {
            vm.request(HostRequest::AlarmAlert {
                active: false,
                volume: 0,
                vibration: false,
            });
        }
    }
}
