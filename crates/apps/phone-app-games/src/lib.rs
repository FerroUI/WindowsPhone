//! Games is a registry-driven collection hub; game implementations live in their own crates.
mod collection;

use ferroui_base::layout::Orientation;
use ferroui_view::prelude::*;
use phone_controls::{action, app_bar, pivot, Icon};
use phone_domain::{AppId, Route};
use phone_sdk::{common::*, session::Session, AppCategory, AppRegistration};
use phone_state::PhoneViewModel;
use std::rc::Rc;

pub fn view(vm: Rc<PhoneViewModel>, ui: Rc<Session>) -> Element {
    let data = vm.data();
    let games: Vec<_> = ui
        .app_manifests()
        .into_iter()
        .filter(|app| {
            app.listed
                && app.category == AppCategory::Game
                && app.builtin.map_or_else(
                    || vm.extension_enabled(app.id),
                    |id| data.installed.contains(&id),
                )
        })
        .collect();
    drop(data);
    let mut tabs = stack_panel()
        .orientation(Orientation::Horizontal)
        .margin((20.0, 0.0));
    for (index, label) in ["collection", "player"].iter().enumerate() {
        let state = ui.clone();
        tabs = tabs.child(pivot(label, ui.pivot.get() == index, move || {
            state.pivot.set(index)
        }));
    }
    let content = if ui.pivot.get() == 0 {
        collection::view(vm.clone(), &games)
    } else {
        rsx! {StackPanel {margin:20.0, spacing:22.0,
            {text("local player", 42.0)}
            {text(&format!("{} games", games.len()), 30.0)}
            {muted("Resume your boards and see your scores inside each game. Progress is stored on this device and included in phone backups.", 20.0)}
            {muted("Xbox account, friends and online achievements are not connected.", 17.0)}
        }}
    };
    let store = vm.clone();
    page(
        "games",
        scroll(rsx! {StackPanel {spacing:20.0, {tabs} {content}}}),
        app_bar(
            vec![action(Icon::Store, "games store", move || {
                store.navigate(Route::App(AppId::Store))
            })],
            false,
            vec![],
        ),
    )
}

pub fn registration() -> AppRegistration {
    AppRegistration::builtin(AppId::Games, view).with_pivot_count(|_| 2)
}
