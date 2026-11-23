//! Explicit position acquisition and current weather; no background tracking.
use ferroui_base::threading::Dispatcher;
use phone_host::native::{location, open_external, weather};
use phone_state::PhoneViewModel;
use std::{cell::Cell, rc::Rc};

thread_local! { static ACTIVE: Cell<bool> = const { Cell::new(false) }; }

pub fn start(vm: &Rc<PhoneViewModel>, current_weather: bool) {
    if ACTIVE.with(|active| active.replace(true)) {
        vm.set_status("A location request is already in progress.".into());
        return;
    }
    let weak = Rc::downgrade(vm);
    let epoch = vm.document_epoch();
    drop(Dispatcher::current_dispatcher().spawn_local(async move {
        let result = async {
            let position = location::locate().await?;
            let Some(vm) = weak.upgrade().filter(|vm| vm.document_epoch() == epoch) else {
                return Ok(());
            };
            if current_weather {
                let reading =
                    crate::background::run_system(move || weather::current(position)).await?;
                if vm.document_epoch() == epoch {
                    phone_ui::set_weather(
                        "current location".into(),
                        reading.temperature_celsius,
                        reading.condition.into(),
                    );
                }
            } else {
                let url = position.map_url()?;
                crate::background::run_system(move || open_external(std::ffi::OsStr::new(&url)))
                    .await?;
            }
            Ok::<_, String>(())
        }
        .await;
        ACTIVE.with(|active| active.set(false));
        if let Err(error) = result {
            if let Some(vm) = weak.upgrade().filter(|vm| vm.document_epoch() == epoch) {
                vm.set_status(error);
            }
        }
    }));
}
