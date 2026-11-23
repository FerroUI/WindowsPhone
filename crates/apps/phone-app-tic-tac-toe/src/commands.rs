use crate::model::{Mode, Saved};
use phone_sdk::AppContext;

pub fn change(context: &AppContext, apply: impl FnOnce(&mut Saved) -> Result<bool, String>) {
    let storage = context.storage();
    let result = storage.read::<Saved>().and_then(|mut save| {
        save.validate()?;
        if apply(&mut save)? {
            storage.write(&save)?;
        }
        Ok(())
    });
    if let Err(error) = result {
        context.vm.set_status(error);
    }
}

pub fn new_game(context: &AppContext, mode: Mode) {
    let mut save = context.storage().read::<Saved>().unwrap_or_default();
    if save.validate().is_err() {
        save = Saved::default();
    }
    save.restart(mode);
    if let Err(error) = context.storage().write(&save) {
        context.vm.set_status(error);
    }
}
