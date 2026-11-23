pub mod model;
mod pages;
pub mod view;
use model::Library;
use phone_sdk::AppStorage;
use phone_state::PhoneViewModel;
use std::rc::Rc;
pub const APP_ID: &str = "phone.onenote";

pub fn read(vm: &Rc<PhoneViewModel>) -> Result<Library, String> {
    let mut library = AppStorage::new(vm.clone(), APP_ID).read::<Library>()?;
    library.validate()?;
    library.prune(vm.data().notes.iter().map(|note| note.id));
    Ok(library)
}
pub fn change<T>(
    vm: &Rc<PhoneViewModel>,
    change: impl FnOnce(&mut Library) -> Result<T, String>,
) -> Result<T, String> {
    let mut library = read(vm)?;
    let result = change(&mut library)?;
    library.validate()?;
    AppStorage::new(vm.clone(), APP_ID).write(&library)?;
    Ok(result)
}
