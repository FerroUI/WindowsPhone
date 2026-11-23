use phone_domain::{MediaTrack, PhoneData};
use phone_state::PhoneViewModel;
use std::{cell::Cell, rc::Rc};
fn track(source: &str) -> MediaTrack {
    MediaTrack {
        title: "Episode".into(),
        source: source.into(),
        artist: "Podcast".into(),
        media_kind: "audio".into(),
        ..Default::default()
    }
}
#[test]
fn media_import_commits_once_deduplicates_and_rejects_invalid_batches_atomically() {
    let writes = Rc::new(Cell::new(0));
    let observed = writes.clone();
    let vm = PhoneViewModel::new(
        PhoneData::default(),
        Rc::new(move |_| observed.set(observed.get() + 1)),
    );
    let ids = vm
        .import_media_tracks(vec![
            track("https://example.org/a.mp3"),
            track("https://example.org/b.mp3"),
            track("https://example.org/a.mp3"),
        ])
        .unwrap();
    assert_eq!(writes.get(), 1);
    assert_eq!(ids[0], ids[2]);
    assert_ne!(ids[0], ids[1]);
    assert_eq!(vm.data().tracks.len(), 2);
    let before = vm.data().clone();
    assert!(vm
        .import_media_tracks(vec![
            track("https://example.org/c.mp3"),
            track("file:///private/file")
        ])
        .is_err());
    assert_eq!(*vm.data(), before);
    assert_eq!(writes.get(), 1);
    vm.import_media_tracks(vec![track("https://example.org/a.mp3")])
        .unwrap();
    assert_eq!(writes.get(), 1);
}
