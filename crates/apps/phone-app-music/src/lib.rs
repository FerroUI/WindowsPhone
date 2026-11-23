//! Music application entry point over the shared retained media collection component.
use ferroui_view::prelude::Element;
use phone_sdk::session::Session;
use phone_state::PhoneViewModel;
use std::rc::Rc;
pub fn view(vm: Rc<PhoneViewModel>, ui: Rc<Session>) -> Element {
    phone_media_support::view(vm, ui, phone_domain::AppId::Music)
}

mod registration;
pub use registration::registration;
