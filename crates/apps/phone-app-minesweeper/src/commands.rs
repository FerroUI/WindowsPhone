use crate::{
    model::{Difficulty, Saved},
    state::ViewState,
};
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

pub fn select(context: &AppContext, index: usize) {
    let flag = context
        .session
        .state::<ViewState>()
        .flag_mode
        .get_untracked();
    let now = context.vm.now();
    change(context, |save| {
        if flag {
            save.flag(index)
        } else {
            save.reveal(index, now)
        }
    });
}

pub fn new_game(context: &AppContext, difficulty: Difficulty) {
    let mut save = context.storage().read::<Saved>().unwrap_or_default();
    if save.validate().is_err() {
        save = Saved::default();
    }
    let seed = save
        .game
        .seed
        .wrapping_mul(1_664_525)
        .wrapping_add(context.vm.now() as u32)
        .wrapping_add(1)
        .max(1);
    save.restart(difficulty, seed);
    match context.storage().write(&save) {
        Ok(()) => context.session.state::<ViewState>().flag_mode.set(false),
        Err(error) => context.vm.set_status(error),
    }
}
