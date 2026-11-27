//! Rendering snapshots have their own mutable document and no host actuators.
use crate::PhoneViewModel;
use phone_domain::{AppId, Route};
use std::rc::Rc;

impl PhoneViewModel {
    /// Whether this model is an isolated visual snapshot. Renderers can retain
    /// live content while suppressing a newly mounted decorative entrance.
    pub fn is_preview_snapshot(&self) -> bool {
        self.preview_badges.borrow().is_some()
    }

    /// Make a detached visual snapshot without navigating this model. The copy
    /// has no persistence/host callbacks, pending commands, navigation journal,
    /// provider records, permission grants or provider action tokens. Its
    /// catalog, descriptive host state, clock and badge numbers are observations.
    ///
    /// This isolates mutations through the supplied model. Registered native
    /// Rust renderers remain trusted code; independently captured/global state
    /// is not sandboxed by a rendering snapshot.
    pub fn preview_snapshot(&self, route: Route) -> Rc<Self> {
        let snapshot = Self::new(self.document.borrow().clone(), Rc::new(|_| {}));
        *snapshot.current_route.borrow_mut() = route;
        snapshot.document_epoch.set(self.document_epoch.get());
        snapshot.viewport_size.set(self.viewport_size.get());
        snapshot.clock.set(self.clock.get());
        snapshot.utc_offset.set(self.utc_offset.get());
        *snapshot.environment.borrow_mut() = self.environment.borrow().clone();
        snapshot.environment.borrow_mut().notification_permission =
            phone_host::HostPermission::Unknown;
        *snapshot.platform_state.borrow_mut() = self.platform_state.borrow().clone();
        *snapshot.device_applications.borrow_mut() = self.device_applications.borrow().clone();
        snapshot
            .device_applications_revision
            .set(self.device_applications_revision.get());
        *snapshot.registered_apps.borrow_mut() = self.registered_apps.borrow().clone();
        *snapshot.registered_builtins.borrow_mut() = self.registered_builtins.borrow().clone();
        *snapshot.preview_badges.borrow_mut() = Some(
            AppId::ALL
                .iter()
                .map(|app| (*app, self.app_badge(*app)))
                .collect(),
        );
        snapshot
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{HostEnvironment, HostRequest, HostServices};
    use phone_domain::PhoneData;
    use phone_host::platform::DeviceApplication;
    use std::cell::Cell;

    #[test]
    fn visual_snapshot_preserves_observations_without_callbacks_or_source_mutation() {
        let saves = Rc::new(Cell::new(0));
        let requests = Rc::new(Cell::new(0));
        let saved = saves.clone();
        let requested = requests.clone();
        let vm = PhoneViewModel::new(
            PhoneData::default(),
            Rc::new(move |_| saved.set(saved.get() + 1)),
        );
        vm.set_services(HostServices {
            request: Rc::new(move |_| requested.set(requested.get() + 1)),
        });
        vm.set_viewport(320.0, 740.0);
        vm.set_clock(123_456);
        vm.set_utc_offset(3600);
        vm.set_environment(HostEnvironment {
            notification_permission: phone_host::HostPermission::Granted,
            ..Default::default()
        });
        vm.set_device_applications(vec![DeviceApplication {
            id: "org.example/.App".into(),
            title: "Example".into(),
        }])
        .unwrap();
        let original = vm.export_json().unwrap();
        let revision = vm.revision();
        let snapshot = vm.preview_snapshot(Route::Apps);
        assert_eq!(snapshot.route(), Route::Apps);
        assert_eq!(vm.route(), Route::Start);
        assert_eq!(snapshot.export_json().unwrap(), original);
        assert_eq!(snapshot.viewport(), vm.viewport());
        assert_eq!(snapshot.local_now(), vm.local_now());
        assert_eq!(snapshot.device_applications(), vm.device_applications());
        assert_eq!(snapshot.device_state(), Default::default());
        assert_eq!(
            snapshot.environment().notification_permission,
            phone_host::HostPermission::Unknown
        );
        assert!(snapshot.pending.borrow().is_empty());
        assert!(snapshot.history.borrow().is_empty());
        assert!(!snapshot.applying.get());
        for app in AppId::ALL {
            assert_eq!(snapshot.app_badge(app), vm.app_badge(app));
        }
        snapshot.request(HostRequest::StopCamera);
        snapshot.replace_data(PhoneData::default()).unwrap();
        snapshot.navigate(Route::App(AppId::Settings));
        assert_eq!(saves.get(), 0);
        assert_eq!(requests.get(), 0);
        assert_eq!(vm.revision(), revision);
        assert_eq!(vm.route(), Route::Start);
        assert_eq!(vm.export_json().unwrap(), original);
    }
}
