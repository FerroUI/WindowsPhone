use super::*;

pub struct TestDirectory(PathBuf);
impl TestDirectory {
    pub fn new() -> Self {
        let path = std::env::temp_dir().join(unique_key().replace(':', "-"));
        fs::create_dir_all(&path).unwrap();
        Self(path)
    }
    pub fn store(&self) -> AssetStore {
        AssetStore::new(self.0.clone())
    }
}
impl Drop for TestDirectory {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn asset_roundtrip_is_immutable_and_never_uses_display_name_as_path() {
    let directory = TestDirectory::new();
    let store = directory.store();
    let first = store
        .put(
            "Photo 11/23/2026",
            "image/jpeg",
            Kind::Image,
            b"photo bytes",
        )
        .unwrap();
    let second = store
        .put(
            "Photo 11/23/2026",
            "image/jpeg",
            Kind::Image,
            b"other bytes",
        )
        .unwrap();
    assert_ne!(first, second);
    let asset = store.get(&first).unwrap();
    assert_eq!(asset.metadata.name, "Photo 11/23/2026");
    assert_eq!(asset.bytes, b"photo bytes");
    assert_eq!(asset.metadata.extension, "jpg");
    assert!(store.playable(&first).is_err());
    store.remove(&first).unwrap();
    store.remove(&first).unwrap();
    assert!(store.get(&first).is_err());
    assert_eq!(store.get(&second).unwrap().bytes, b"other bytes");
}

#[test]
fn invalid_media_identifiers_cannot_escape_the_store() {
    let directory = TestDirectory::new();
    let store = directory.store();
    for key in [
        "",
        "file:///etc/passwd",
        "media:",
        "media:../secret",
        "media:a/b",
        "media:a\\b",
        "media:-a",
        "media:a.json",
        "media:a%2fb",
        "media:a:b",
        "media:é",
    ] {
        assert!(store.get(key).is_err(), "{key}");
        assert!(store.playable(key).is_err(), "{key}");
    }
    assert!(key_suffix(&format!("media:{}", "a".repeat(129))).is_err());
    assert!(key_suffix("media:04b28468-6107-437e-98ce-053c983f75c3").is_ok());
}

#[test]
fn validates_kind_sizes_extensions_and_corrupted_manifests() {
    let directory = TestDirectory::new();
    let store = directory.store();
    assert!(store.put("empty.mp3", "", Kind::Audio, b"").is_err());
    assert!(validate_asset("large.png", "", Kind::Image, IMAGE_LIMIT + 1).is_err());
    assert!(validate_asset("movie.exe", "", Kind::Video, 1).is_err());
    assert!(validate_asset("photo.png", "audio/mpeg", Kind::Image, 1).is_err());
    let key = store.put("Song.MP3", "", Kind::Audio, b"music").unwrap();
    let (path, metadata) = store.playable(&key).unwrap();
    assert!(path.starts_with(&directory.0));
    assert_eq!(path.extension().unwrap(), "mp3");
    let mut json = serde_json::to_value(metadata).unwrap();
    json["extension"] = "../outside".into();
    fs::write(
        store.metadata_path(&key).unwrap(),
        serde_json::to_vec(&json).unwrap(),
    )
    .unwrap();
    assert!(store.get(&key).is_err());
}

#[cfg(unix)]
#[test]
fn refuses_symlink_media_files() {
    let directory = TestDirectory::new();
    let store = directory.store();
    let key = store.put("audio.wav", "", Kind::Audio, b"audio").unwrap();
    let (path, _) = store.playable(&key).unwrap();
    fs::remove_file(&path).unwrap();
    let elsewhere = directory.0.join("outside");
    fs::write(&elsewhere, b"secret").unwrap();
    std::os::unix::fs::symlink(&elsewhere, &path).unwrap();
    assert!(store.get(&key).is_err());
    assert!(store.playable(&key).is_err());
}
