//! An independently registered phone SDK game with isolated, resumable storage.
mod commands;
mod logic;
mod model;
mod view;

use phone_controls::Icon;
use phone_sdk::{AppCategory, AppManifest, AppRegistration};

pub const APP_ID: &str = "org.ferroui.tic-tac-toe";

pub fn registration() -> AppRegistration {
    AppRegistration::new(
        AppManifest::new(APP_ID, "Tic tac toe", Icon::Games)
            .category(AppCategory::Game)
            .accent("#108A15"),
        view::view,
    )
}
