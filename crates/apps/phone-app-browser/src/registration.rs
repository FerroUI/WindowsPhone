use phone_sdk::{session::Session, AppRegistration};
use phone_state::PhoneViewModel;
use std::rc::Rc;

pub fn registration() -> AppRegistration {
    AppRegistration::builtin(phone_domain::AppId::InternetExplorer, crate::view)
        .with_back(|vm, ui| crate::browser_back(vm, ui))
        .with_host_navigation(crate::browser_navigated)
        .with_route_changed(route_changed)
        .with_reset(|ui| {
            let state = ui.state::<crate::BrowserSession>();
            state.url.set(String::new());
            state.view.set(crate::session::BrowserView::Page);
        })
}

fn route_changed(
    _: &Rc<PhoneViewModel>,
    ui: &Rc<Session>,
    _: &phone_domain::Route,
    next: &phone_domain::Route,
) {
    if *next == phone_domain::Route::App(phone_domain::AppId::InternetExplorer) {
        ui.entry
            .set(ui.state::<crate::BrowserSession>().url.get_untracked());
    }
}
