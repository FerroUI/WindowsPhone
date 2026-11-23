//! Independent podcast library, RSS/Atom import, streaming and playback progress.
mod commands;
mod feed;
mod model;
mod state;
mod views;
use phone_sdk::prelude::*;
pub const APP_ID: &str = "org.ferroui.podcasts";
pub fn registration() -> AppRegistration {
    AppRegistration::new(
        AppManifest::new(APP_ID, "Podcasts", Icon::Headphones)
            .category(AppCategory::Media)
            .accent("#AA00FF")
            .capabilities(&[
                Capability::Network,
                Capability::Files,
                Capability::Media,
                Capability::Sharing,
            ]),
        views::view,
    )
    .with_back(|_, ui| {
        let state = ui.state::<state::State>();
        if state.episode.get_untracked().is_some() {
            state.episode.set(None);
            true
        } else {
            false
        }
    })
    .with_reset(|ui| ui.state::<state::State>().reset())
    .with_route_changed(|_, ui, old, new| {
        if old != new {
            ui.state::<state::State>().episode.set(None);
        }
    })
    .with_media_event(commands::media_event)
}
