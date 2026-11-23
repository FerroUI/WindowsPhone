use crate::collections::Collections;
use phone_domain::{AppId, Id, MediaTrack};
use phone_sdk::{media::RepeatMode, AppStorage};
use phone_state::PhoneViewModel;
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, rc::Rc};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Library {
    pub version: u32,
    pub playlists: Collections,
    pub albums: BTreeMap<Id, String>,
    pub repeat: RepeatMode,
    pub shuffle: bool,
}
impl Default for Library {
    fn default() -> Self {
        Self {
            version: 1,
            playlists: Collections::default(),
            albums: BTreeMap::new(),
            repeat: RepeatMode::Off,
            shuffle: false,
        }
    }
}
impl Library {
    pub fn validate(&self) -> Result<(), String> {
        if self.version != 1 {
            return Err("This media library was saved by an unsupported app version.".into());
        }
        self.playlists.validate()?;
        if self.albums.len() > 4096 {
            return Err("The media library has too many album labels.".into());
        }
        for (id, album) in &self.albums {
            if *id == 0 {
                return Err("An album label has an invalid item identifier.".into());
            }
            crate::collections::valid_name(album)?;
        }
        Ok(())
    }
    pub fn album(&self, id: Id) -> &str {
        self.albums
            .get(&id)
            .map(String::as_str)
            .unwrap_or("unknown album")
    }
    pub fn tracks<'a>(&self, playlist: Id, tracks: &'a [MediaTrack]) -> Vec<&'a MediaTrack> {
        let by_id = tracks
            .iter()
            .map(|track| (track.id, track))
            .collect::<std::collections::HashMap<_, _>>();
        self.playlists
            .get(playlist)
            .map(|p| {
                p.items
                    .iter()
                    .filter_map(|id| by_id.get(id).copied())
                    .collect()
            })
            .unwrap_or_default()
    }
}
pub fn storage(vm: &Rc<PhoneViewModel>, app: AppId) -> AppStorage {
    AppStorage::new(
        vm.clone(),
        if app == AppId::Video {
            "phone.video"
        } else {
            "phone.music"
        },
    )
}
pub fn read(vm: &Rc<PhoneViewModel>, app: AppId) -> Result<Library, String> {
    let value = storage(vm, app).read::<Library>()?;
    value.validate()?;
    Ok(value)
}
pub fn change(
    vm: &Rc<PhoneViewModel>,
    app: AppId,
    apply: impl FnOnce(&mut Library) -> Result<(), String>,
) -> Result<(), String> {
    let mut value = read(vm, app)?;
    let tracks = vm
        .data()
        .tracks
        .iter()
        .filter(|track| (track.media_kind == "video") == (app == AppId::Video))
        .map(|track| track.id)
        .collect::<std::collections::BTreeSet<_>>();
    value.playlists.retain_items(|id| tracks.contains(&id));
    value.albums.retain(|id, _| tracks.contains(id));
    apply(&mut value)?;
    value.validate()?;
    storage(vm, app).write(&value)
}

#[cfg(test)]
mod tests {
    use super::*;
    use phone_domain::PhoneData;
    use std::collections::BTreeSet;
    #[test]
    fn playlists_roundtrip_by_track_identity_and_mutations_preserve_the_other_library() {
        let mut data = PhoneData::default();
        data.tracks = vec![
            MediaTrack {
                id: 1,
                title: "Song".into(),
                source: "media:old".into(),
                media_kind: "audio".into(),
                ..Default::default()
            },
            MediaTrack {
                id: 2,
                title: "Clip".into(),
                source: "media:video".into(),
                media_kind: "video".into(),
                ..Default::default()
            },
        ];
        let vm = PhoneViewModel::new(data, Rc::new(|_| {}));
        change(&vm, AppId::Music, |library| {
            let id = library.playlists.create("Road trip")?;
            library
                .playlists
                .replace_items(id, &BTreeSet::from([1]), &[1])?;
            library.albums.insert(1, "Album".into());
            Ok(())
        })
        .unwrap();
        change(&vm, AppId::Video, |library| {
            library.playlists.create("Films")?;
            Ok(())
        })
        .unwrap();
        let music = vm.app_data("phone.music").unwrap();
        let mut restored = PhoneData::from_json(&vm.export_json().unwrap()).unwrap();
        restored.tracks[0].source = "media:remapped".into();
        let vm = PhoneViewModel::new(restored, Rc::new(|_| {}));
        let library = read(&vm, AppId::Music).unwrap();
        assert_eq!(
            library.tracks(1, &vm.data().tracks)[0].source,
            "media:remapped"
        );
        change(&vm, AppId::Video, |library| {
            library.playlists.remove(1);
            Ok(())
        })
        .unwrap();
        assert_eq!(vm.app_data("phone.music"), Some(music));
        vm.delete_track(1);
        change(&vm, AppId::Music, |_| Ok(())).unwrap();
        assert!(read(&vm, AppId::Music)
            .unwrap()
            .playlists
            .get(1)
            .unwrap()
            .items
            .is_empty());
    }
    #[test]
    fn unsupported_library_versions_are_preserved_and_rejected() {
        let vm = PhoneViewModel::new(PhoneData::default(), Rc::new(|_| {}));
        let value = serde_json::json!({"version":999});
        vm.set_app_data("phone.music", Some(value.clone())).unwrap();
        assert!(change(&vm, AppId::Music, |library| {
            library.playlists.create("New")?;
            Ok(())
        })
        .is_err());
        assert_eq!(vm.app_data("phone.music"), Some(value));
    }
}
