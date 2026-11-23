//! Office document library and dispatch to independently compiled editors.
mod hub;
use phone_office_support::*;
pub fn view(vm: Rc<PhoneViewModel>, ui: Rc<Session>) -> Element {
    if let Route::Document(id) = vm.route() {
        let document = vm.data().document(id).cloned();
        if let Some(document) = document {
            return match document.kind() {
                phone_domain::DocumentKind::Word => phone_app_word::view(vm, ui, document),
                phone_domain::DocumentKind::Excel => phone_app_excel::view(vm, ui, document),
                phone_domain::DocumentKind::PowerPoint => {
                    phone_app_powerpoint::view(vm, ui, document)
                }
            };
        }
    }
    hub::view(vm, ui)
}

mod registration;
pub use registration::registration;
