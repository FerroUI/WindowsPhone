pub mod alarms;
pub mod calculator;
pub mod calendar;
pub mod games;
pub mod mail;
pub mod media;
pub mod messaging;
pub mod notes;
pub mod people;
pub mod phone;
pub mod services;
pub mod settings;
pub mod store;
use crate::session::Session;
use ferroui_view::prelude::*;
use phone_domain::{AppId, Route};
use phone_state::PhoneViewModel;
use std::rc::Rc;

pub fn view(vm: Rc<PhoneViewModel>, ui: Rc<Session>) -> Element {
    match vm.route() {
        Route::Conversation(_) => messaging::view(vm, ui),
        Route::Contact(_) => people::view(vm, ui),
        Route::Mail(_) => mail::view(vm, ui),
        Route::Note(_) => notes::view(vm, ui),
        Route::Setting(_) => settings::view(vm, ui),
        Route::App(app) => match app {
            AppId::Phone => phone::view(vm, ui),
            AppId::Messaging => messaging::view(vm, ui),
            AppId::People => people::view(vm, ui),
            AppId::Outlook => mail::view(vm, ui),
            AppId::Calendar => calendar::view(vm, ui),
            AppId::OneNote => notes::view(vm, ui),
            AppId::Alarms => alarms::view(vm, ui),
            AppId::Calculator => calculator::view(vm, ui),
            AppId::Settings => settings::view(vm, ui),
            AppId::Store => store::view(vm, ui),
            AppId::Games => games::view(vm, ui),
            AppId::Photos | AppId::Camera | AppId::Music | AppId::Video => media::view(vm, ui, app),
            _ => services::view(vm, ui, app),
        },
        _ => crate::common::empty("Windows Phone", "Press Start to return to your tiles."),
    }
}
