use crate::{
    model::{self, Library},
    state::{LibraryPage, LibrarySession},
};
use phone_domain::{Action, AppId, Id, MediaTrack};
use phone_sdk::{
    media::{self, PlaybackSession},
    session::Session,
};
use phone_state::{HostRequest, PhoneViewModel};
use std::{collections::BTreeSet, rc::Rc};

pub fn visible(library: &Library, page: &LibraryPage, tracks: &[MediaTrack]) -> Vec<MediaTrack> {
    match page {
        LibraryPage::Playlist(id) => library.tracks(*id, tracks).into_iter().cloned().collect(),
        LibraryPage::Artist(artist) => tracks
            .iter()
            .filter(|track| artist_name(track) == artist)
            .cloned()
            .collect(),
        LibraryPage::Album(album) => tracks
            .iter()
            .filter(|track| library.album(track.id) == album)
            .cloned()
            .collect(),
        _ => tracks.to_vec(),
    }
}
pub fn artist_name(track: &MediaTrack) -> &str {
    if track.artist.trim().is_empty() {
        "unknown artist"
    } else {
        &track.artist
    }
}
pub fn play(vm: &Rc<PhoneViewModel>, ui: &Rc<Session>, app: AppId, id: Id) {
    let result = (|| {
        let library = model::read(vm, app)?;
        let tracks = vm
            .data()
            .tracks
            .iter()
            .filter(|track| (track.media_kind == "video") == (app == AppId::Video))
            .cloned()
            .collect::<Vec<_>>();
        let page = ui.state::<LibrarySession>().page(app).get_untracked();
        let tracks = visible(&library, &page, &tracks);
        let selected = tracks
            .iter()
            .find(|track| track.id == id)
            .ok_or("This media item is no longer in your collection.")?;
        let mut seen = std::collections::HashSet::new();
        media::start(
            vm,
            ui,
            tracks
                .iter()
                .map(|track| track.source.clone())
                .filter(|source| seen.insert(source.clone()))
                .collect(),
            &selected.source,
            library.repeat,
            library.shuffle,
        )?;
        ui.pivot.set(1);
        ui.menu.set(false);
        Ok::<_, String>(())
    })();
    if let Err(error) = result {
        vm.set_status(error);
    }
}
pub fn create_playlist(vm: &Rc<PhoneViewModel>, ui: &Rc<Session>, app: AppId) {
    let model = vm.clone();
    let session = ui.clone();
    let epoch = vm.document_epoch();
    ui.form(
        "new playlist",
        vec![("name", String::new())],
        Rc::new(move |fields| {
            if model.document_epoch() != epoch {
                return;
            }
            let mut created = None;
            let result = model::change(&model, app, |library| {
                created = Some(
                    library
                        .playlists
                        .create(fields.first().map(String::as_str).unwrap_or_default())?,
                );
                Ok(())
            });
            match result {
                Ok(()) => {
                    session.dialog.set(None);
                    session.pivot.set(0);
                    if let Some(id) = created {
                        begin_selection(&model, &session, app, id);
                    }
                }
                Err(error) => session.form_error.set(error),
            }
        }),
    );
}
pub fn rename_playlist(vm: &Rc<PhoneViewModel>, ui: &Rc<Session>, app: AppId, id: Id) {
    let Ok(library) = model::read(vm, app) else {
        return;
    };
    let Some(group) = library.playlists.get(id) else {
        return;
    };
    let model = vm.clone();
    let state = ui.clone();
    let epoch = vm.document_epoch();
    ui.form(
        "rename playlist",
        vec![("name", group.name.clone())],
        Rc::new(move |fields| {
            if model.document_epoch() != epoch {
                return;
            }
            match model::change(&model, app, |library| {
                library
                    .playlists
                    .rename(id, fields.first().map(String::as_str).unwrap_or_default())
            }) {
                Ok(()) => state.dialog.set(None),
                Err(error) => state.form_error.set(error),
            }
        }),
    );
}
pub fn delete_playlist(vm: &Rc<PhoneViewModel>, ui: &Rc<Session>, app: AppId, id: Id) {
    let model = vm.clone();
    let state = ui.clone();
    let epoch = vm.document_epoch();
    ui.confirm(
        "Delete this playlist? Your media files will be kept.",
        Rc::new(move || {
            if model.document_epoch() != epoch {
                return;
            }
            match model::change(&model, app, |library| {
                library.playlists.remove(id);
                Ok(())
            }) {
                Ok(()) => {
                    state.dialog.set(None);
                    state
                        .state::<LibrarySession>()
                        .page(app)
                        .set(LibraryPage::Playlists);
                }
                Err(error) => state.form_error.set(error),
            }
        }),
    );
}
pub fn begin_selection(vm: &Rc<PhoneViewModel>, ui: &Rc<Session>, app: AppId, id: Id) {
    if let Ok(library) = model::read(vm, app) {
        if let Some(group) = library.playlists.get(id) {
            let available = vm
                .data()
                .tracks
                .iter()
                .map(|track| track.id)
                .collect::<BTreeSet<_>>();
            let state = ui.state::<LibrarySession>();
            state.selection.set(
                group
                    .items
                    .iter()
                    .copied()
                    .filter(|id| available.contains(id))
                    .collect(),
            );
            state.page(app).set(LibraryPage::Select(id));
            ui.query.set(String::new());
        }
    }
}
pub fn save_selection(vm: &Rc<PhoneViewModel>, ui: &Rc<Session>, app: AppId) {
    let state = ui.state::<LibrarySession>();
    let LibraryPage::Select(id) = state.page(app).get_untracked() else {
        return;
    };
    let selected = state.selection.get_untracked();
    let available = vm
        .data()
        .tracks
        .iter()
        .filter(|track| (track.media_kind == "video") == (app == AppId::Video))
        .map(|track| track.id)
        .collect::<Vec<_>>();
    match model::change(vm, app, |library| {
        library.playlists.replace_items(id, &selected, &available)
    }) {
        Ok(()) => {
            state.page(app).set(LibraryPage::Playlist(id));
            state.selection.set(Default::default());
            ui.query.set(String::new());
        }
        Err(error) => vm.set_status(error),
    }
}
pub fn edit_track(vm: &Rc<PhoneViewModel>, ui: &Rc<Session>, id: Id) {
    let Some(track) = vm
        .data()
        .tracks
        .iter()
        .find(|track| track.id == id)
        .cloned()
    else {
        return;
    };
    let model = vm.clone();
    let state = ui.clone();
    let epoch = vm.document_epoch();
    ui.form(
        "edit song info",
        vec![("title", track.title), ("artist", track.artist)],
        Rc::new(move |fields| {
            if model.document_epoch() != epoch {
                return;
            }
            let title = fields.first().map(|s| s.trim()).unwrap_or_default();
            let artist = fields.get(1).map(|s| s.trim()).unwrap_or_default();
            if title.is_empty()
                || title.chars().count() > 200
                || artist.chars().count() > 200
                || title.chars().any(char::is_control)
                || artist.chars().any(char::is_control)
            {
                state
                    .form_error
                    .set("Enter a title and artist of at most 200 printable characters.".into());
                return;
            }
            let Some(mut track) = model
                .data()
                .tracks
                .iter()
                .find(|track| track.id == id)
                .cloned()
            else {
                state
                    .form_error
                    .set("This media item no longer exists.".into());
                return;
            };
            track.title = title.into();
            track.artist = artist.into();
            model.dispatch(Action::SaveTrack(track));
            state.dialog.set(None);
        }),
    );
}
pub fn edit_album(vm: &Rc<PhoneViewModel>, ui: &Rc<Session>, app: AppId, id: Id) {
    let Ok(library) = model::read(vm, app) else {
        return;
    };
    let name = library.albums.get(&id).cloned().unwrap_or_default();
    let model = vm.clone();
    let state = ui.clone();
    let epoch = vm.document_epoch();
    ui.form(
        "set album",
        vec![("album", name)],
        Rc::new(move |fields| {
            if model.document_epoch() != epoch {
                return;
            }
            let album = fields.first().map(|s| s.trim()).unwrap_or_default();
            if !model.data().tracks.iter().any(|track| track.id == id) {
                state
                    .form_error
                    .set("This media item no longer exists.".into());
                return;
            }
            match model::change(&model, app, |library| {
                if album.is_empty() {
                    library.albums.remove(&id);
                } else {
                    library
                        .albums
                        .insert(id, crate::collections::valid_name(album)?);
                }
                Ok(())
            }) {
                Ok(()) => state.dialog.set(None),
                Err(error) => state.form_error.set(error),
            }
        }),
    );
}
pub fn remove(vm: &Rc<PhoneViewModel>, ui: &Rc<Session>, app: AppId, id: Id) {
    let model = vm.clone();
    let state = ui.clone();
    let epoch = vm.document_epoch();
    ui.confirm(
        "Delete this media item from your phone?",
        Rc::new(move || {
            if model.document_epoch() != epoch {
                return;
            }
            let Some(track) = model
                .data()
                .tracks
                .iter()
                .find(|track| track.id == id)
                .cloned()
            else {
                state.dialog.set(None);
                return;
            };
            if let Err(error) = model::change(&model, app, |library| {
                library.playlists.retain_items(|item| item != id);
                library.albums.remove(&id);
                Ok(())
            }) {
                state.form_error.set(error);
                return;
            }
            if state.media_source.get_untracked() == track.source {
                media::flush(&model, &state);
                if model.document_epoch() != epoch {
                    return;
                }
                model.request(HostRequest::MediaControl {
                    action: "stop".into(),
                });
                media::publish_state(&model, &state, String::new(), String::new(), false);
            }
            state.state::<PlaybackSession>().queue.update(|queue| {
                queue.retain(|source| source != track.source);
            });
            model.dispatch(Action::DeleteTrack(id));
            if (track.source.starts_with("media:")
                || phone_sdk::host::http::validate_url(&track.source).is_ok())
                && !model
                    .data()
                    .tracks
                    .iter()
                    .any(|other| other.source == track.source)
            {
                model.request(HostRequest::DeleteMedia {
                    source: track.source,
                });
            }
            state.dialog.set(None);
            state.menu.set(false);
        }),
    );
}
