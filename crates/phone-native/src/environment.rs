use phone_host::{native::PowerMonitor, HostEnvironment, HostPermission, HostPlatform};
use phone_state::PhoneViewModel;
use std::{
    cell::Cell,
    rc::{Rc, Weak},
    time::{Duration, Instant},
};

pub struct NativeEnvironment {
    power: Option<PowerMonitor>,
    vm: Weak<PhoneViewModel>,
    last_notification_refresh: Cell<Instant>,
}

impl NativeEnvironment {
    pub fn new(vm: &Rc<PhoneViewModel>) -> Self {
        #[cfg(target_os = "windows")]
        let platform = HostPlatform::Windows;
        #[cfg(target_os = "macos")]
        let platform = HostPlatform::Macos;
        #[cfg(target_os = "linux")]
        let platform = HostPlatform::Linux;
        #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
        let platform = HostPlatform::Unknown;
        vm.set_environment(HostEnvironment {
            platform,
            notification_permission: HostPermission::Unknown,
            storage_persisted: Some(true),
            persistent_storage_supported: Some(false),
            // Native registration is observed when a player first opens.
            media_session_supported: None,
            ..Default::default()
        });
        crate::notifications::refresh(vm, false);
        Self {
            power: PowerMonitor::start().ok(),
            vm: Rc::downgrade(vm),
            last_notification_refresh: Cell::new(Instant::now()),
        }
    }

    pub fn tick(&self, vm: &PhoneViewModel) {
        if self.last_notification_refresh.get().elapsed() >= Duration::from_secs(30) {
            self.last_notification_refresh.set(Instant::now());
            if let Some(vm) = self.vm.upgrade() {
                crate::notifications::refresh(&vm, false);
            }
        }
        if let Some(value) = self.power.as_ref().and_then(PowerMonitor::take_latest) {
            let mut environment = vm.environment();
            environment.battery_percent = value.battery_percent;
            environment.charging = value.charging;
            environment.external_power = value.external_power;
            environment.power_saver = value.power_saver;
            vm.set_environment(environment);
        }
    }
}
