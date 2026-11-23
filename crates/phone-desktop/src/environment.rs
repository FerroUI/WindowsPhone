use phone_host::{HostEnvironment, HostPermission, HostPlatform, native::PowerMonitor};
use phone_state::PhoneViewModel;

pub struct NativeEnvironment {
    power: Option<PowerMonitor>,
}

impl NativeEnvironment {
    pub fn new(vm: &PhoneViewModel) -> Self {
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
            notification_permission: HostPermission::Unsupported,
            storage_persisted: Some(true),
            persistent_storage_supported: Some(false),
            media_session_supported: Some(false),
            ..Default::default()
        });
        Self {
            power: PowerMonitor::start().ok(),
        }
    }

    pub fn tick(&self, vm: &PhoneViewModel) {
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
