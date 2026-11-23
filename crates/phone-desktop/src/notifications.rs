//! Dispatcher bridge for OS notification observation and explicit permission.
use ferroui_base::{threading::Dispatcher, Ref};
use ferroui_controls::Window;
use phone_host::{native::notifications, HostPermission};
use phone_state::PhoneViewModel;
use std::{cell::Cell, rc::Rc};

thread_local! {
    static OBSERVING: Cell<bool> = const { Cell::new(false) };
    static REQUEST_PENDING: Cell<bool> = const { Cell::new(false) };
}

pub fn refresh(vm: &Rc<PhoneViewModel>, request: bool) {
    if OBSERVING.with(|value| value.replace(true)) {
        if request {
            REQUEST_PENDING.with(|value| value.set(true));
        }
        return;
    }
    let weak = Rc::downgrade(vm);
    drop(Dispatcher::current_dispatcher().spawn_local(async move {
        let result =
            crate::background::run_system(move || notifications::permission(request)).await;
        OBSERVING.with(|value| value.set(false));
        let request_pending = REQUEST_PENDING.with(|value| value.replace(false));
        let Some(vm) = weak.upgrade() else {
            return;
        };
        let mut environment = vm.environment();
        environment.notification_permission = match result {
            Ok(value) => value,
            Err(error) => {
                if request {
                    vm.set_status(error);
                }
                HostPermission::Unknown
            }
        };
        vm.set_environment(environment);
        // An explicit click during a passive refresh must still request consent.
        if request_pending && !request {
            refresh(&vm, true);
        }
    }));
}

pub fn window_handle(window: &Ref<Window>) -> Option<isize> {
    window
        .try_get_platform_handle()
        .filter(|handle| handle.handle_descriptor() == Some("HWND"))
        .map(|handle| handle.handle())
}

pub fn show(vm: &Rc<PhoneViewModel>, window: &Ref<Window>, title: String, body: String) {
    if vm.environment().notification_permission != HostPermission::Granted {
        return;
    }
    let handle = window_handle(window);
    let weak = Rc::downgrade(vm);
    drop(Dispatcher::current_dispatcher().spawn_local(async move {
        if let Err(error) =
            crate::background::run_system(move || notifications::show(handle, &title, &body)).await
        {
            if let Some(vm) = weak.upgrade() {
                vm.set_status(format!("Device notification: {error}"));
            }
        }
    }));
}
