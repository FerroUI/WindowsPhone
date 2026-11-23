use ferroui_view::prelude::*;
use phone_domain::{AppId, Id};
use std::collections::BTreeSet;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum LibraryPage {
    #[default]
    Songs,
    Artists,
    Albums,
    Playlists,
    Artist(String),
    Album(String),
    Playlist(Id),
    Select(Id),
}
pub struct LibrarySession {
    pub music: Signal<LibraryPage>,
    pub video: Signal<LibraryPage>,
    pub selection: Signal<BTreeSet<Id>>,
    pub queue_open: Signal<bool>,
}
impl Default for LibrarySession {
    fn default() -> Self {
        Self {
            music: signal(LibraryPage::Songs),
            video: signal(LibraryPage::Songs),
            selection: signal(BTreeSet::new()),
            queue_open: signal(false),
        }
    }
}
impl LibrarySession {
    pub fn page(&self, app: AppId) -> Signal<LibraryPage> {
        if app == AppId::Video {
            self.video
        } else {
            self.music
        }
    }
}
