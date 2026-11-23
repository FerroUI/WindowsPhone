//! Standalone SDK game: its model and saved progress are owned by this crate.
mod commands;
mod model;
mod state;
mod view;

use phone_controls::Icon;
use phone_sdk::{AppCategory, AppManifest, AppRegistration};

pub const APP_ID: &str = "org.ferroui.minesweeper";

pub fn registration() -> AppRegistration {
    AppRegistration::new(
        AppManifest::new(APP_ID, "Minesweeper", Icon::Games)
            .category(AppCategory::Game)
            .accent("#007C9D"),
        view::view,
    )
    .with_reset(|session| {
        let state = session.state::<state::ViewState>();
        state.flag_mode.set(false);
        state.clock.set(0);
    })
    .with_tick(|_vm, session, now| {
        let state = session.state::<state::ViewState>();
        if state.clock.get_untracked() != now {
            state.clock.set(now);
        }
    })
}
