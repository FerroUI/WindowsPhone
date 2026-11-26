use super::*;
use phone_domain::PhoneData;
use phone_host::HostPlatform;

struct NetworkProvider {
    owns: bool,
    active: Cell<bool>,
    online: Cell<Option<bool>>,
}

impl PlatformPlugin for NetworkProvider {
    fn id(&self) -> &'static str {
        "test.runtime.network-observation"
    }

    fn state(&self) -> PlatformState {
        PlatformState {
            platform: HostPlatform::Ios,
            ..Default::default()
        }
    }

    fn environment(&self) -> HostEnvironment {
        HostEnvironment {
            platform: HostPlatform::Ios,
            online: self.online.get(),
            ..Default::default()
        }
    }

    fn owns_network_observation(&self) -> bool {
        self.owns
    }

    fn dispatch(&self, _: PlatformRequest) -> PlatformTask {
        ready(Err("This observer does not issue native actions.".into()))
    }

    fn lifecycle(&self, event: HostLifecycle) {
        let active = event == HostLifecycle::Resumed;
        if self.active.replace(active) != active {
            self.online.set(None);
        }
    }
}

fn fixture(owns: bool) -> (Rc<PhoneRuntime>, Rc<NetworkProvider>) {
    let vm = PhoneViewModel::new(PhoneData::demo(), Rc::new(|_| {}));
    vm.set_environment(HostEnvironment {
        platform: HostPlatform::Ios,
        battery_percent: Some(73),
        storage_usage_bytes: Some(4096),
        online: Some(true),
        ..Default::default()
    });
    let provider = Rc::new(NetworkProvider {
        owns,
        active: Cell::new(true),
        online: Cell::new(None),
    });
    let runtime = PhoneRuntime::attach(vm, provider.clone(), ServiceRegistry::default()).unwrap();
    (runtime, provider)
}

#[test]
fn network_owner_retires_stale_vm_state_and_waits_for_a_fresh_foreground_observation() {
    let (runtime, provider) = fixture(true);
    assert_eq!(runtime.vm.environment().online, None);
    assert_eq!(runtime.vm.environment().battery_percent, Some(73));
    assert_eq!(runtime.vm.environment().storage_usage_bytes, Some(4096));

    provider.online.set(Some(true));
    runtime.observe();
    assert_eq!(runtime.vm.environment().online, Some(true));
    provider.online.set(None);
    runtime.observe();
    assert_eq!(runtime.vm.environment().online, None);
    provider.online.set(Some(false));
    runtime.observe();
    assert_eq!(runtime.vm.environment().online, Some(false));

    runtime.lifecycle(HostLifecycle::Suspended);
    assert_eq!(runtime.vm.environment().online, None);
    provider.online.set(Some(true));
    runtime.observe();
    assert_eq!(
        runtime.vm.environment().online,
        None,
        "background observations cannot repopulate the VM"
    );
    runtime.lifecycle(HostLifecycle::Resumed);
    assert_eq!(
        runtime.vm.environment().online,
        None,
        "resuming does not assert the previous path is usable"
    );
    provider.online.set(Some(true));
    runtime.observe();
    assert_eq!(runtime.vm.environment().online, Some(true));
    assert_eq!(runtime.vm.environment().battery_percent, Some(73));
    assert_eq!(runtime.vm.environment().storage_usage_bytes, Some(4096));
    runtime.lifecycle(HostLifecycle::Closing);
}

#[test]
fn providers_without_network_ownership_keep_the_optional_measurement_merge_policy() {
    let (runtime, provider) = fixture(false);
    assert_eq!(runtime.vm.environment().online, Some(true));
    runtime.lifecycle(HostLifecycle::Suspended);
    assert_eq!(runtime.vm.environment().online, Some(true));
    runtime.lifecycle(HostLifecycle::Resumed);
    assert_eq!(runtime.vm.environment().online, Some(true));
    provider.online.set(Some(false));
    runtime.observe();
    assert_eq!(runtime.vm.environment().online, Some(false));
    provider.online.set(None);
    runtime.observe();
    assert_eq!(runtime.vm.environment().online, Some(false));
    assert_eq!(runtime.vm.environment().battery_percent, Some(73));
    assert_eq!(runtime.vm.environment().storage_usage_bytes, Some(4096));
    runtime.lifecycle(HostLifecycle::Closing);
}
