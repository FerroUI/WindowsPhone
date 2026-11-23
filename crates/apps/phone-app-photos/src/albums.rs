use crate::PhotosSession;
use ferroui_view::prelude::*;
use phone_domain::{Action, Id, Photo};
use phone_media_support::collections::Collections;
use phone_sdk::{common::*, session::Session, AppStorage};
use phone_state::PhoneViewModel;
use serde::{Deserialize, Serialize};
use std::{collections::BTreeSet, rc::Rc};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Albums {
    pub version: u32,
    pub albums: Collections,
}
impl Default for Albums {
    fn default() -> Self {
        Self {
            version: 1,
            albums: Collections::default(),
        }
    }
}
pub fn read(vm: &PhoneViewModel) -> Result<Albums, String> {
    let value = vm
        .app_data("phone.photos")
        .map(serde_json::from_value::<Albums>)
        .transpose()
        .map_err(|e| format!("Invalid photo albums: {e}"))?
        .unwrap_or_default();
    if value.version != 1 {
        return Err("These photo albums were saved by an unsupported version.".into());
    }
    value.albums.validate()?;
    Ok(value)
}
fn change(
    vm: &Rc<PhoneViewModel>,
    apply: impl FnOnce(&mut Albums) -> Result<(), String>,
) -> Result<(), String> {
    let mut value = read(vm)?;
    let photos = vm
        .data()
        .photos
        .iter()
        .map(|p| p.id)
        .collect::<BTreeSet<_>>();
    value.albums.retain_items(|id| photos.contains(&id));
    apply(&mut value)?;
    value.albums.validate()?;
    AppStorage::new(vm.clone(), "phone.photos").write(&value)
}
pub fn visible(vm: &PhoneViewModel, ui: &Session) -> Vec<Photo> {
    let photos = vm.data().photos.clone();
    match ui.pivot.get_untracked() {
        2 => photos.into_iter().filter(|photo| photo.favorite).collect(),
        1 => {
            let Some(id) = ui.state::<PhotosSession>().album.get_untracked() else {
                return photos;
            };
            let Ok(value) = read(vm) else {
                return Vec::new();
            };
            value
                .albums
                .get(id)
                .map(|album| {
                    album
                        .items
                        .iter()
                        .filter_map(|id| photos.iter().find(|photo| photo.id == *id).cloned())
                        .collect()
                })
                .unwrap_or_default()
        }
        _ => photos,
    }
}
pub fn list(vm: Rc<PhoneViewModel>, ui: Rc<Session>) -> Element {
    let value = match read(&vm) {
        Ok(value) => value,
        Err(error) => return scroll(empty("can't read albums", &error)),
    };
    let mut items = stack_panel();
    let all = ui.clone();
    items = items.child(row(
        "Camera Roll",
        &format!("{} photos", vm.data().photos.len()),
        move || {
            all.pivot.set(0);
            all.state::<PhotosSession>().album.set(None);
        },
    ));
    for album in value.albums.groups {
        let count = album
            .items
            .iter()
            .filter(|id| vm.data().photos.iter().any(|photo| photo.id == **id))
            .count();
        let state = ui.clone();
        let id = album.id;
        items = items.child(row(&album.name, &format!("{count} photos"), move || {
            state.state::<PhotosSession>().album.set(Some(id));
            state.selected.set(None);
        }));
    }
    scroll(items.into())
}
pub fn create(vm: &Rc<PhoneViewModel>, ui: &Rc<Session>, initial: Option<Id>) {
    let model = vm.clone();
    let state = ui.clone();
    let epoch = vm.document_epoch();
    ui.form(
        "new album",
        vec![("name", String::new())],
        Rc::new(move |fields| {
            if model.document_epoch() != epoch {
                return;
            }
            let mut created = None;
            let available = model
                .data()
                .photos
                .iter()
                .map(|photo| photo.id)
                .collect::<Vec<_>>();
            let result = change(&model, |value| {
                let id = value
                    .albums
                    .create(fields.first().map(String::as_str).unwrap_or_default())?;
                if let Some(photo) = initial {
                    value
                        .albums
                        .replace_items(id, &BTreeSet::from([photo]), &available)?;
                }
                created = Some(id);
                Ok(())
            });
            match result {
                Ok(()) => {
                    state.dialog.set(None);
                    state.state::<PhotosSession>().choosing_album.set(false);
                    if initial.is_none() {
                        state.pivot.set(1);
                        state.state::<PhotosSession>().album.set(created);
                        if let Some(id) = created {
                            select(&model, &state, id);
                        }
                    }
                }
                Err(error) => state.form_error.set(error),
            }
        }),
    );
}
pub fn rename(vm: &Rc<PhoneViewModel>, ui: &Rc<Session>, id: Id) {
    let Ok(value) = read(vm) else { return };
    let Some(album) = value.albums.get(id) else {
        return;
    };
    let model = vm.clone();
    let state = ui.clone();
    let epoch = vm.document_epoch();
    ui.form(
        "rename album",
        vec![("name", album.name.clone())],
        Rc::new(move |fields| {
            if model.document_epoch() != epoch {
                return;
            }
            match change(&model, |value| {
                value
                    .albums
                    .rename(id, fields.first().map(String::as_str).unwrap_or_default())
            }) {
                Ok(()) => state.dialog.set(None),
                Err(error) => state.form_error.set(error),
            }
        }),
    );
}
pub fn delete(vm: &Rc<PhoneViewModel>, ui: &Rc<Session>, id: Id) {
    let model = vm.clone();
    let state = ui.clone();
    let epoch = vm.document_epoch();
    ui.confirm(
        "Delete this album? Your photos will be kept.",
        Rc::new(move || {
            if model.document_epoch() != epoch {
                return;
            }
            match change(&model, |value| {
                value.albums.remove(id);
                Ok(())
            }) {
                Ok(()) => {
                    state.dialog.set(None);
                    state.state::<PhotosSession>().album.set(None);
                }
                Err(error) => state.form_error.set(error),
            }
        }),
    );
}
pub fn select(vm: &Rc<PhoneViewModel>, ui: &Rc<Session>, id: Id) {
    if let Ok(value) = read(vm) {
        if let Some(album) = value.albums.get(id) {
            let available = vm
                .data()
                .photos
                .iter()
                .map(|photo| photo.id)
                .collect::<BTreeSet<_>>();
            let state = ui.state::<PhotosSession>();
            state.selection.set(
                album
                    .items
                    .iter()
                    .copied()
                    .filter(|id| available.contains(id))
                    .collect(),
            );
            state.selecting.set(true);
        }
    }
}
pub fn save_selection(vm: &Rc<PhoneViewModel>, ui: &Rc<Session>) {
    let state = ui.state::<PhotosSession>();
    let Some(id) = state.album.get_untracked() else {
        return;
    };
    let selection = state.selection.get_untracked();
    let available = vm
        .data()
        .photos
        .iter()
        .map(|photo| photo.id)
        .collect::<Vec<_>>();
    match change(vm, |value| {
        value.albums.replace_items(id, &selection, &available)
    }) {
        Ok(()) => {
            state.selecting.set(false);
            state.selection.set(Default::default());
        }
        Err(error) => vm.set_status(error),
    }
}
pub fn remove_photo(vm: &Rc<PhoneViewModel>, album: Id, photo: Id) {
    if let Err(error) = change(vm, |value| {
        value.albums.remove_item(album, photo);
        Ok(())
    }) {
        vm.set_status(error);
    }
}
pub fn forget_photo(vm: &Rc<PhoneViewModel>, photo: Id) -> Result<(), String> {
    change(vm, |value| {
        value.albums.retain_items(|id| id != photo);
        Ok(())
    })
}
pub fn chooser(vm: Rc<PhoneViewModel>, ui: Rc<Session>, photo: Id) -> Element {
    let value = match read(&vm) {
        Ok(value) => value,
        Err(error) => return scroll(empty("can't read albums", &error)),
    };
    let create_vm = vm.clone();
    let create_ui = ui.clone();
    let mut body = stack_panel().child(row(
        "new album",
        "Create an album for this photo.",
        move || create(&create_vm, &create_ui, Some(photo)),
    ));
    for album in value.albums.groups {
        let model = vm.clone();
        let state = ui.clone();
        let id = album.id;
        let contains = album.items.contains(&photo);
        body = body.child(row(
            &album.name,
            if contains {
                "already in album"
            } else {
                "add photo"
            },
            move || {
                let available = model.data().photos.iter().map(|p| p.id).collect::<Vec<_>>();
                let result = change(&model, |value| {
                    let mut selected = value
                        .albums
                        .get(id)
                        .ok_or("This album no longer exists.")?
                        .items
                        .iter()
                        .copied()
                        .collect::<BTreeSet<_>>();
                    selected.insert(photo);
                    value.albums.replace_items(id, &selected, &available)
                });
                match result {
                    Ok(()) => state.state::<PhotosSession>().choosing_album.set(false),
                    Err(error) => model.set_status(error),
                }
            },
        ));
    }
    page("add to album", scroll(body.into()), Element::Empty)
}
pub fn rename_photo(vm: &Rc<PhoneViewModel>, ui: &Rc<Session>, id: Id) {
    let Some(photo) = vm.data().photos.iter().find(|p| p.id == id).cloned() else {
        return;
    };
    let model = vm.clone();
    let state = ui.clone();
    let epoch = vm.document_epoch();
    ui.form(
        "rename photo",
        vec![("name", photo.name)],
        Rc::new(move |fields| {
            if model.document_epoch() != epoch {
                return;
            }
            let name = match phone_media_support::collections::valid_name(
                fields.first().map(String::as_str).unwrap_or_default(),
            ) {
                Ok(name) => name,
                Err(error) => {
                    state.form_error.set(error);
                    return;
                }
            };
            let Some(mut photo) = model.data().photos.iter().find(|p| p.id == id).cloned() else {
                state.form_error.set("This photo no longer exists.".into());
                return;
            };
            photo.name = name;
            model.dispatch(Action::SavePhoto(photo));
            state.dialog.set(None);
        }),
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn albums_keep_ids_across_asset_remapping_and_never_remove_the_photo() {
        let mut data = phone_domain::PhoneData::default();
        data.photos.push(Photo {
            id: 1,
            name: "One".into(),
            source: "forest".into(),
            favorite: false,
        });
        let vm = PhoneViewModel::new(data, Rc::new(|_| {}));
        change(&vm, |value| {
            let album = value.albums.create("Family")?;
            value
                .albums
                .replace_items(album, &BTreeSet::from([1]), &[1])
        })
        .unwrap();
        let mut restored = phone_domain::PhoneData::from_json(&vm.export_json().unwrap()).unwrap();
        restored.photos[0].source = "coast".into();
        let vm = PhoneViewModel::new(restored, Rc::new(|_| {}));
        assert_eq!(read(&vm).unwrap().albums.get(1).unwrap().items, vec![1]);
        remove_photo(&vm, 1, 1);
        assert!(read(&vm).unwrap().albums.get(1).unwrap().items.is_empty());
        assert_eq!(vm.data().photos.len(), 1);
    }
    #[test]
    fn corrupt_album_schema_is_never_overwritten_by_a_mutation() {
        let vm = PhoneViewModel::new(phone_domain::PhoneData::default(), Rc::new(|_| {}));
        let data = serde_json::json!({"version":99});
        vm.set_app_data("phone.photos", Some(data.clone())).unwrap();
        assert!(forget_photo(&vm, 1).is_err());
        assert_eq!(vm.app_data("phone.photos"), Some(data));
    }
}
