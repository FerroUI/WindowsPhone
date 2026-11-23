use crate::{HostRequest, PhoneViewModel};
use phone_domain::{Action, Id, Route, TileAppearance, TileNotification, TileTarget};

impl PhoneViewModel {
    pub fn set_tile_appearance(&self, id: Id, appearance: TileAppearance) -> Result<(), String> {
        appearance.validate()?;
        let data = self.data();
        data.tile(id).ok_or("This tile is no longer pinned.")?;
        if let phone_domain::TileBackground::Photo(photo) = appearance.background {
            if !data.photos.iter().any(|item| item.id == photo) {
                return Err("Choose an existing photo.".into());
            }
        }
        drop(data);
        self.dispatch(Action::SetTileAppearance { id, appearance });
        Ok(())
    }
    pub fn publish_tile_notification(
        &self,
        id: Id,
        notification: TileNotification,
    ) -> Result<(), String> {
        notification.validate_publication(self.now())?;
        let data = self.data();
        let tile = data.tile(id).ok_or("This tile is no longer pinned.")?;
        if tile.is_folder() {
            return Err("Publish to an app tile inside the folder.".into());
        }
        if notification
            .template
            .photo_ids()
            .iter()
            .any(|id| !data.photos.iter().any(|photo| photo.id == *id))
        {
            return Err("Choose an existing tile photo.".into());
        }
        drop(data);
        self.dispatch(Action::PublishTileNotification { id, notification });
        Ok(())
    }
    pub fn clear_tile_notifications(&self, id: Id, tag: Option<&str>) -> Result<(), String> {
        if let Some(tag) = tag {
            phone_domain::validate_tile_tag(tag)?;
        }
        self.data()
            .tile(id)
            .ok_or("This tile is no longer pinned.")?;
        self.dispatch(Action::ClearTileNotifications {
            id,
            tag: tag.map(str::to_owned),
        });
        Ok(())
    }
    pub fn dissolve_folder(&self, id: Id) {
        self.dispatch(Action::DissolveFolder(id));
    }
    pub fn move_into_folder(&self, tile: Id, folder: Id) {
        self.dispatch(Action::MoveIntoFolder { tile, folder });
    }
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
