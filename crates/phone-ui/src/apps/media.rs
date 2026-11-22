mod camera;
mod photo_grid;
mod photos;
mod player;

use crate::session::Session;
use ferroui_view::prelude::*;
use phone_domain::AppId;
use phone_state::PhoneViewModel;
use std::rc::Rc;

pub fn view(vm: Rc<PhoneViewModel>, ui: Rc<Session>, app: AppId) -> Element {
    match app {
        AppId::Photos => photos::view(vm, ui),
        AppId::Camera => camera::view(vm, ui),
        AppId::Music | AppId::Video => player::view(vm, ui, app),
        _ => unreachable!("media route"),
    }
}
