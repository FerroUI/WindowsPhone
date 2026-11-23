use crate::{HostRequest, PhoneViewModel};
use phone_domain::{Action, Id, Route, TileTarget};

impl PhoneViewModel {
    pub fn pin_contact(&self, id: Id) {
        self.dispatch(Action::PinTarget(TileTarget::Contact(id)));
    }
    pub fn pin_note(&self, id: Id) {
        self.dispatch(Action::PinTarget(TileTarget::Note(id)));
    }
    pub fn pin_document(&self, id: Id) {
        self.dispatch(Action::PinTarget(TileTarget::Document(id)));
    }
    pub fn pin_bookmark(&self, id: Id) {
        self.dispatch(Action::PinTarget(TileTarget::Bookmark(id)));
    }
    pub fn extract_folder_tile(&self, folder: Id, tile: Id) {
        self.dispatch(Action::ExtractFolderTile { folder, tile });
    }
    pub fn move_folder_tile(&self, folder: Id, tile: Id, before: Option<Id>) {
        self.dispatch(Action::MoveFolderTile {
            folder,
            tile,
            before,
        });
    }
    pub fn move_tile_relative(&self, id: Id, target: Id, after: bool) {
        self.dispatch(Action::MoveTileRelative { id, target, after });
    }
    pub fn open_tile(&self, id: Id) {
        let tile = self.data().tile(id).cloned();
        let Some(tile) = tile else {
            return;
        };
        if tile.is_folder() {
            self.navigate(Route::Folder(id));
            return;
        }
        match tile.target {
            Some(TileTarget::AppLink(id)) => {
                let link = self
                    .data()
                    .app_links
                    .iter()
                    .find(|link| link.id == id)
                    .cloned();
                if let Some(link) = link {
                    if self.extension_enabled(&link.app) {
                        self.navigate(Route::Extension {
                            app: link.app,
                            path: link.path,
                        });
                    } else {
                        self.set_status(
                            "This app is unavailable or disabled in this build.".into(),
                        );
                    }
                }
            }
            Some(TileTarget::Contact(id)) => self.navigate(Route::Contact(id)),
            Some(TileTarget::Note(id)) => self.navigate(Route::Note(id)),
            Some(TileTarget::Document(id)) => self.navigate(Route::Document(id)),
            Some(TileTarget::Bookmark(id)) => {
                let url = self
                    .data()
                    .bookmarks
                    .iter()
                    .find(|bookmark| bookmark.id == id)
                    .map(|bookmark| bookmark.url.clone());
                if let Some(url) = url {
                    self.navigate(Route::App(tile.app));
                    self.request(HostRequest::Browse { url });
                }
            }
            None => self.resume_app(tile.app),
        }
    }
}
