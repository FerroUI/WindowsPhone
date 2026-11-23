use super::PhoneViewModel;
use phone_host::platform::{validate_applications, DeviceApplication, PlatformState};
impl PhoneViewModel {
    /// Host observations are transient; they never enter backups or app data.
    pub fn platform_state(&self) -> PlatformState {
        self.platform_state.borrow().clone()
    }
    pub fn set_platform_state(&self, state: PlatformState) -> Result<(), String> {
        state.validate()?;
        if *self.platform_state.borrow() != state {
            *self.platform_state.borrow_mut() = state;
            self.notify();
        }
        Ok(())
    }
    pub fn device_applications(&self) -> Vec<DeviceApplication> {
        self.device_applications.borrow().clone()
    }
    /// Advances on every admitted catalog response, including unchanged rows.
    pub fn device_applications_revision(&self) -> u64 {
        self.device_applications_revision.get()
    }
    pub fn set_device_applications(&self, mut apps: Vec<DeviceApplication>) -> Result<(), String> {
        validate_applications(&apps)?;
        apps.sort_by_cached_key(|app| (app.title.to_lowercase(), app.id.clone()));
        let revision = self
            .device_applications_revision
            .get()
            .checked_add(1)
            .ok_or("Device catalog revision exhausted. Restart the host.")?;
        if *self.device_applications.borrow() != apps {
            *self.device_applications.borrow_mut() = apps;
        }
        self.device_applications_revision.set(revision);
        self.notify();
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn catalog_completion_advances_without_row_changes_and_rejects_invalid_updates() {
        let vm = PhoneViewModel::new(phone_domain::PhoneData::demo(), std::rc::Rc::new(|_| {}));
        let app = DeviceApplication {
            id: "org.example/.Main".into(),
            title: "Example".into(),
        };
        vm.set_device_applications(vec![app.clone()]).unwrap();
        let first = vm.device_applications_revision();
        vm.set_device_applications(vec![app.clone()]).unwrap();
        assert_eq!(vm.device_applications_revision(), first + 1);
        assert!(vm
            .set_device_applications(vec![app.clone(), app.clone()])
            .is_err());
        assert_eq!(vm.device_applications_revision(), first + 1);
        assert_eq!(vm.device_applications(), vec![app]);
    }
}
