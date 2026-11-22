//! Retained, declarative Windows Phone views shared by the native and web hosts.
mod app_list;
mod apps;
mod common;
mod dialogs;
mod gestures;
mod hardware;
mod images;
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
    static SESSION: RefCell<Weak<session::Session>> = const { RefCell::new(Weak::new()) };
}

pub fn create_view(vm: Rc<PhoneViewModel>) -> Ref<Control> {
    ComponentHost::with_view(move || {
        let revision = signal(vm.revision());
        let ui = Rc::new(session::Session::new(vm.now()));
        SESSION.with(|slot| *slot.borrow_mut() = Rc::downgrade(&ui));
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
                ui.previews.clear();
            }
        });
    }
}

pub fn back(vm: &Rc<PhoneViewModel>) {
    if !SESSION.with(|slot| {
        slot.borrow().upgrade().is_some_and(|ui| {
            if ui.locked.get_untracked()
                && !ui.hardware.is_open()
                && !ui.notification_open.get_untracked()
            {
                return true;
            }
            ui.dismiss() || apps::services::browser_back(vm, &ui)
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
            ui.weather.set(Some((place, temperature, condition)));
        }
    });
}
pub fn lock() {
    SESSION.with(|slot| {
        if let Some(ui) = slot.borrow().upgrade() {
            ui.locked.set(true);
        }
    });
}
pub fn browser_navigated(vm: &PhoneViewModel, url: String) {
    SESSION.with(|slot| {
        if let Some(ui) = slot.borrow().upgrade() {
            apps::services::browser_navigated(vm, &ui, &url);
        }
    });
}
pub fn home(vm: &Rc<PhoneViewModel>) {
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
            ui.hardware.is_open()
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
            ui.hardware.tick(now);
            ui.tick_live_tiles(now);
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
        let media = ui.media_playing.get_untracked()
            || matches!(vm.route().app(), Some(AppId::Music | AppId::Video));
        match key {
            "volumeUp" => hardware::adjust(vm, &ui.hardware, 1, media),
            "volumeDown" => hardware::adjust(vm, &ui.hardware, -1, media),
            "volume" => ui.hardware.show(vm.now(), media),
            "volumeMute" => {
                let mut settings = vm.data().settings.clone();
                if media {
                    settings.media_muted = !settings.media_muted;
                } else {
                    settings.sounds = !settings.sounds;
                }
                vm.set_settings(settings);
                ui.hardware.show(vm.now(), media);
            }
            "power" => {
                let locked = ui.locked.get_untracked() || vm.route() == Route::Lock;
                // Lock hides app overlays without discarding an unsaved form.
                ui.hardware.close();
                ui.notification_open.set(false);
                ui.notification_peek.set(false);
                ui.locked.set(!locked);
                if locked && vm.route() == Route::Lock {
                    vm.back();
                }
            }
            "camera" => {
                ui.clear_overlays();
                ui.locked.set(false);
                vm.navigate(Route::App(AppId::Camera));
            }
            _ => {}
        }
    });
}
