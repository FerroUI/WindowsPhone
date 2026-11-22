//! Retained, declarative Windows Phone views shared by the native and web hosts.
mod app_list;
mod apps;
mod common;
mod dialogs;
mod gestures;
mod images;
mod navigation;
mod notifications;
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
        let ui = Rc::new(session::Session::new());
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

pub fn back(vm: &Rc<PhoneViewModel>) {
    if SESSION.with(|slot| {
        slot.borrow()
            .upgrade()
            .is_some_and(|ui| ui.locked.get_untracked())
    }) {
        return;
    }
    if !SESSION.with(|slot| slot.borrow().upgrade().is_some_and(|ui| ui.dismiss())) {
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
pub fn browser_navigated(url: String) {
    SESSION.with(|slot| {
        if let Some(ui) = slot.borrow().upgrade() {
            ui.browser_url.set(url.clone());
            ui.entry.set(url);
            ui.pivot.set(0);
            ui.menu.set(false);
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
            ui.locked.get_untracked()
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
