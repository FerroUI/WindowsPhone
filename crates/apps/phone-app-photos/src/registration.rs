use phone_sdk::{session::Session, AppRegistration};
use phone_state::PhoneViewModel;
use std::rc::Rc;

pub fn registration() -> AppRegistration {
    AppRegistration::builtin(phone_domain::AppId::Photos, crate::view)
        .with_pivot_count(|ui| {
            if ui.selected.get_untracked().is_none() {
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
        .with_reset(|ui| ui.state::<crate::PhotosSession>().zoom.set(1.0))
}

fn swipe(vm: &Rc<PhoneViewModel>, ui: &Rc<Session>, delta: f64) -> bool {
    let Some(id) = ui.selected.get_untracked() else {
        return false;
    };
    if ui.state::<crate::PhotosSession>().zoom.get_untracked() > 1.0 {
        return false;
    }
    let data = vm.data();
    let photos = &data.photos;
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
