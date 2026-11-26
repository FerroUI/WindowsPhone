use crate::PhoneViewModel;
use phone_domain::AppId;
use phone_host::platform::{DevicePermission, DeviceRole, HostFeature, HostRole, PermissionState};

impl PhoneViewModel {
    /// One authority for Start and lock-screen counts. A native phone provider
    /// never falls back to sample/local calls when access is denied or unknown.
    pub fn app_badge(&self, app: AppId) -> u32 {
        if app == AppId::Phone && self.platform_state().supports(HostFeature::Telephony) {
            return self
                .device_state
                .borrow()
                .missed_calls
                .as_ref()
                .map_or(0, |value| value.count);
        }
        if app == AppId::Messaging {
            let state = self.device_state.borrow();
            if state.snapshot.permission(DevicePermission::ReadSms) == PermissionState::Granted
                || state.snapshot.role(DeviceRole::Sms) == HostRole::Active
            {
                return state
                    .threads
                    .iter()
                    .fold(0u32, |sum, row| sum.saturating_add(row.unread));
            }
        }
        self.data().badge(app).min(u32::MAX as usize) as u32
    }
}
