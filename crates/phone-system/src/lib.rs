//! Core system shell shared by native and browser hosts.
use crate::session::SystemSessionExt;
use phone_alarm_service as alarm;
mod app_list;

use phone_sdk::common;
mod dialogs;
mod gestures;
mod hardware;
use phone_sdk::images;
mod lockscreen;
mod navigation;
mod notifications;
mod previews;
pub(crate) mod session;
mod shell;
mod start;

use ferroui_base::Ref;
use ferroui_controls::Control;
use ferroui_reactive::ObservableExt;
use ferroui_reactiveui::IReactiveNotifyPropertyChanged;
use ferroui_view::prelude::*;
use phone_state::PhoneViewModel;
use std::{
    cell::RefCell,
    rc::{Rc, Weak},
};

ferroui_base::ferro_thread_local! {
    static MODEL: RefCell<Weak<PhoneViewModel>> = const { RefCell::new(Weak::new()) };
    static SESSION: RefCell<Weak<session::Session>> = const { RefCell::new(Weak::new()) };
}

pub fn create_view(vm: Rc<PhoneViewModel>, registry: Rc<phone_sdk::AppRegistry>) -> Ref<Control> {
    vm.set_registered_apps(
        registry
            .manifests()
            .into_iter()
            .filter(|m| m.builtin.is_none())
            .map(|m| m.id.to_string()),
    );
    ComponentHost::with_view(move || {
        let revision = signal(vm.revision());
        let ui = Rc::new(session::Session::new(vm.now(), registry.clone()));
        SESSION.with(|slot| *slot.borrow_mut() = Rc::downgrade(&ui));
        phone_sdk::set_current_session(&ui);
        MODEL.with(|slot| *slot.borrow_mut() = Rc::downgrade(&vm));
        let volume_ui = Rc::downgrade(&ui);
        phone_sdk::set_volume_handler(Rc::new(move |vm| {
            if let Some(ui) = volume_ui.upgrade() {
                ui.state::<hardware::HardwareState>().show(vm.now(), false);
            }
        }));
        images::initialize();
        let observed = vm.clone();
        let subscription = vm
            .changed()
            .subscribe_next(move |_| revision.set(observed.revision()));
        on_cleanup(move || subscription.dispose());
        let vm = vm.clone();
        rerender(move || {
            revision.get();
            shell::view(vm.clone(), ui.clone())
        })
    })
    .upcast()
}

/// Opt a real rendering host into native page previews. Disabled by default so
/// headless/stub hosts never enter bitmap rendering. May be set before create_view.
pub fn set_preview_capture_enabled(enabled: bool) {
    previews::set_enabled(enabled);
    if !enabled {
        SESSION.with(|slot| {
            if let Some(ui) = slot.borrow().upgrade() {
                ui.state::<crate::previews::PreviewCache>().clear();
            }
        });
    }
}

pub fn back(vm: &Rc<PhoneViewModel>) {
    if !SESSION.with(|slot| {
        slot.borrow().upgrade().is_some_and(|ui| {
            if ui.state::<phone_alarm_service::AlarmState>().back(vm) {
                return true;
            }
            if ui.locked.get_untracked()
                && !ui.state::<crate::hardware::HardwareState>().is_open()
                && !ui.notification_open.get_untracked()
            {
                return true;
            }
            ui.dismiss(vm)
        })
    }) {
        vm.back();
    }
}

pub fn register_image(key: String, bytes: Vec<u8>) -> Result<(), String> {
    images::register(key, bytes)
}

/// Supply an asynchronous source loader; completions use register_image or image_failed.
pub fn set_image_loader(loader: Rc<dyn Fn(String)>) {
    images::set_loader(loader);
}

pub fn image_failed(key: String) {
    images::failed(key);
}

pub fn clear_images() {
    images::clear();
}

pub fn host_cancel_input() {
    gestures::cancel();
}

pub fn show_media(title: String, source: String, playing: bool) {
    SESSION.with(|slot| {
        if let Some(ui) = slot.borrow().upgrade() {
            ui.media_title.set(title);
            ui.media_source.set(source);
            ui.media_playing.set(playing);
        }
    });
}
pub fn set_weather(place: String, temperature: f64, condition: String) {
    SESSION.with(|slot| {
        if let Some(ui) = slot.borrow().upgrade() {
            ui.registry()
                .host_weather(&ui, place, temperature, condition);
        }
    });
}
pub fn lock() {
    SESSION.with(|slot| {
        if let Some(ui) = slot.borrow().upgrade() {
            ui.locked.set(true);
            MODEL.with(|model| {
                if let Some(vm) = model.borrow().upgrade() {
                    vm.request(phone_state::HostRequest::StopCamera);
                }
            });
        }
    });
}
pub fn browser_navigated(vm: &PhoneViewModel, url: String) {
    SESSION.with(|slot| {
        if let Some(ui) = slot.borrow().upgrade() {
            ui.registry().host_navigation(vm, &ui, &url);
        }
    });
}
pub fn home(vm: &Rc<PhoneViewModel>) {
    if SESSION.with(|slot| {
        slot.borrow().upgrade().is_some_and(|ui| {
            ui.state::<phone_alarm_service::AlarmState>()
                .active
                .get_untracked()
                .is_some()
        })
    }) {
        return;
    }
    SESSION.with(|slot| {
        if let Some(ui) = slot.borrow().upgrade() {
            ui.clear_overlays();
            ui.locked.set(false);
        }
    });
    vm.home();
}
pub fn has_overlay() -> bool {
    SESSION.with(|slot| {
        slot.borrow().upgrade().is_some_and(|ui| {
            ui.state::<phone_alarm_service::AlarmState>()
                .active
                .get_untracked()
                .is_some()
                || ui
                    .state::<phone_alarm_service::AlarmState>()
                    .editor
                    .draft
                    .get_untracked()
                    .is_some()
                || ui.state::<crate::hardware::HardwareState>().is_open()
                || ui.locked.get_untracked()
                || ui.notification_open.get_untracked()
                || ui.switcher_open.get_untracked()
                || ui.alphabet.get_untracked()
                || ui.menu.get_untracked()
                || ui.dialog.get_untracked().is_some()
                || ui.edit_tile.get_untracked().is_some()
                || ui.folder.get_untracked().is_some()
        })
    })
}

/// A host clock tick updates visible transient panels and live tiles without serializing data.
pub fn host_tick(now: i64) {
    SESSION.with(|slot| {
        if let Some(ui) = slot.borrow().upgrade() {
            ui.state::<crate::hardware::HardwareState>().tick(now);
            ui.tick_live_tiles(now);
            MODEL.with(|model| {
                if let Some(vm) = model.borrow().upgrade() {
                    if !ui.locked.get_untracked() {
                        ui.registry().tick(&vm, &ui, now);
                    }
                }
            });
        }
    });
}

/// Foreground services use the same validated Rust models on both hosts.
pub fn alarm_tick(vm: &Rc<PhoneViewModel>) {
    SESSION.with(|slot| {
        if let Some(ui) = slot.borrow().upgrade() {
            ui.reconcile_document(&vm);
            ui.state::<phone_alarm_service::AlarmState>().tick(vm);
        }
    });
}
pub fn fire_alarm(vm: &Rc<PhoneViewModel>, id: phone_domain::Id) {
    SESSION.with(|slot| {
        if let Some(ui) = slot.borrow().upgrade() {
            ui.state::<phone_alarm_service::AlarmState>()
                .fire_alarm(vm, id);
        }
    });
}

/// Shared hardware-key contract for a desktop frame and browser-delivered keys.
pub fn hardware_key(vm: &Rc<PhoneViewModel>, key: &str) {
    use phone_domain::{AppId, Route};
    SESSION.with(|slot| {
        let Some(ui) = slot.borrow().upgrade() else {
            return;
        };
        let media = ui
            .state::<phone_alarm_service::AlarmState>()
            .active
            .get_untracked()
            .is_none()
            && (ui.media_playing.get_untracked()
                || matches!(vm.route().app(), Some(AppId::Music | AppId::Video)));
        match key {
            "volumeUp" => {
                hardware::adjust(vm, &ui.state::<crate::hardware::HardwareState>(), 1, media)
            }
            "volumeDown" => {
                hardware::adjust(vm, &ui.state::<crate::hardware::HardwareState>(), -1, media)
            }
            "volume" => ui
                .state::<crate::hardware::HardwareState>()
                .show(vm.now(), media),
            "volumeMute" => {
                let mut settings = vm.data().settings.clone();
                if media {
                    settings.media_muted = !settings.media_muted;
                } else {
                    settings.sounds = !settings.sounds;
                }
                vm.set_settings(settings);
                ui.state::<crate::hardware::HardwareState>()
                    .show(vm.now(), media);
            }
            "power" => {
                let locked = ui.locked.get_untracked() || vm.route() == Route::Lock;
                // Lock hides app overlays without discarding an unsaved form.
                ui.state::<crate::hardware::HardwareState>().close();
                ui.notification_open.set(false);
                ui.notification_peek.set(false);
                ui.locked.set(!locked);
                if locked && vm.route() == Route::Lock {
                    vm.back();
                }
            }
            "camera" => {
                if ui
                    .state::<phone_alarm_service::AlarmState>()
                    .active
                    .get_untracked()
                    .is_some()
                {
                    return;
                }
                ui.clear_overlays();
                ui.locked.set(false);
                vm.navigate(Route::App(AppId::Camera));
            }
            _ => {}
        }
    });
}

/// Registered app lifecycle notification from host focus/visibility changes.
pub fn host_visibility(visible: bool) {
    SESSION.with(|slot| {
        if let Some(ui) = slot.borrow().upgrade() {
            MODEL.with(|model| {
                if let Some(vm) = model.borrow().upgrade() {
                    ui.registry().lifecycle(
                        &vm,
                        &ui,
                        if visible {
                            phone_sdk::AppLifecycle::Resumed
                        } else {
                            phone_sdk::AppLifecycle::Suspended
                        },
                    );
                    if !visible {
                        vm.request(phone_state::HostRequest::StopCamera);
                    }
                }
            });
        }
    });
}
pub use phone_sdk::{set_host_surface_factory, HostSurface};

/// Launch a registered app by stable SDK identity, preserving normal Back behavior.
pub fn launch_app(vm: &Rc<PhoneViewModel>, id: &str, path: &str) -> bool {
    if path.len() > 2048 || path.chars().any(char::is_control) {
        return false;
    }
    SESSION.with(|slot| {
        let Some(ui) = slot.borrow().upgrade() else {
            return false;
        };
        let registry = ui.registry();
        let Some(app) = registry.by_id(id) else {
            return false;
        };
        if app.manifest.builtin.is_some() && !path.is_empty() {
            return false;
        }
        if app.manifest.builtin.map_or_else(
            || !vm.extension_enabled(id),
            |app| !vm.data().installed.contains(&app),
        ) {
            return false;
        }
        let route = app.manifest.route(path);
        ui.clear_overlays();
        vm.navigate(route.clone());
        vm.route() == route
    })
}

pub use phone_sdk::{complete_file, is_file_task_pending, PickedFile};
