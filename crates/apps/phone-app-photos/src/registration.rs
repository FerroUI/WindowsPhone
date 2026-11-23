use phone_sdk::{session::Session, AppRegistration};
use phone_state::PhoneViewModel;
use std::rc::Rc;

pub fn registration() -> AppRegistration {
    AppRegistration::builtin(phone_domain::AppId::Photos, crate::view)
        .with_pivot_count(|ui| {
            if ui.selected.get_untracked().is_none()
                && !ui.state::<crate::PhotosSession>().selecting.get_untracked()
                && (ui.pivot.get_untracked() != 1
                    || ui
                        .state::<crate::PhotosSession>()
                        .album
                        .get_untracked()
                        .is_none())
            {
                3
            } else {
                0
            }
        })
        .with_can_swipe(|ui| {
            ui.selected.get_untracked().is_some()
                && ui.state::<crate::PhotosSession>().zoom.get_untracked() <= 1.0
        })
        .with_swipe(swipe)
        .with_back(|_, ui| {
            let state = ui.state::<crate::PhotosSession>();
            if state.choosing_album.get_untracked() {
                state.choosing_album.set(false);
                return true;
            }
            if state.selecting.get_untracked() {
                state.selecting.set(false);
                state.selection.set(Default::default());
                return true;
            }
            if ui.selected.get_untracked().is_none()
                && ui.pivot.get_untracked() == 1
                && state.album.get_untracked().is_some()
            {
                state.album.set(None);
                return true;
            }
            false
        })
        .with_reset(|ui| {
            let state = ui.state::<crate::PhotosSession>();
            state.zoom.set(1.0);
            state.album.set(None);
            state.selecting.set(false);
            state.selection.set(Default::default());
            state.choosing_album.set(false);
        })
}

fn swipe(vm: &Rc<PhoneViewModel>, ui: &Rc<Session>, delta: f64) -> bool {
    let Some(id) = ui.selected.get_untracked() else {
        return false;
    };
    if ui.state::<crate::PhotosSession>().zoom.get_untracked() > 1.0 {
        return false;
    }
    let photos = crate::albums::visible(vm, ui);
    let Some(index) = photos.iter().position(|photo| photo.id == id) else {
        return false;
    };
    let next = if delta < 0.0 {
        (index + 1).min(photos.len().saturating_sub(1))
    } else {
        index.saturating_sub(1)
    };
    if let Some(photo) = photos.get(next) {
        ui.selected.set(Some(photo.id));
    }
    true
}
