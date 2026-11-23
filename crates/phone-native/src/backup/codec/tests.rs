use super::*;
use crate::asset_store::unique_key;

fn envelope(phone: &PhoneData, assets: serde_json::Value) -> Vec<u8> {
    serde_json::to_vec(&serde_json::json!({
        "format":"ferroui-windows-phone", "version":1, "phone":phone,
        "assets": assets, "missingAssets": [],
    }))
    .unwrap()
}

#[test]
fn browser_envelope_and_legacy_phone_document_are_readable() {
    let mut phone = PhoneData::default();
    phone.settings.lock_wallpaper = "media:lock-photo".into();
    let bytes = envelope(
        &phone,
        serde_json::json!([{
            "key":"media:lock-photo", "name":"Photo 11/23/2026", "mime":"image/jpeg", "kind":"image", "data":"cGhvdG8="
        }]),
    );
    let restored = decode(&bytes).unwrap();
    assert_eq!(restored.assets.len(), 1);
    assert_eq!(restored.assets[0].bytes, b"photo");
    assert_eq!(restored.missing, 0);
    assert_eq!(restored.phone.settings.lock_wallpaper, "media:lock-photo");
    let legacy = decode(phone.to_json().unwrap().as_bytes()).unwrap();
    assert!(legacy.assets.is_empty());
    assert_eq!(legacy.missing, 1);
}

#[test]
fn native_export_preserves_bytes_and_both_wallpapers() {
    let directory = std::env::temp_dir().join(unique_key().replace(':', "-"));
    let store = AssetStore::new(directory.clone());
    let start = store
        .put("Start.png", "image/png", Kind::Image, b"start pixels")
        .unwrap();
    let lock = store
        .put("Lock.jpg", "image/jpeg", Kind::Image, b"lock pixels")
        .unwrap();
    let mut phone = PhoneData::default();
    phone.settings.wallpaper = start.clone();
    phone.settings.lock_wallpaper = lock.clone();
    let (bytes, missing) = export(&phone, &store).unwrap();
    assert_eq!(missing, 0);
    let restored = decode(&bytes).unwrap();
    assert_eq!(restored.assets.len(), 2);
    assert!(restored
        .assets
        .iter()
        .any(|asset| asset.key == start && asset.bytes == b"start pixels"));
    assert!(restored
        .assets
        .iter()
        .any(|asset| asset.key == lock && asset.bytes == b"lock pixels"));
    std::fs::remove_dir_all(directory).unwrap();
}

#[test]
fn validates_every_asset_before_restore_can_mutate_state() {
    assert!(decode(b"{}").is_err());
    assert!(
        decode(br#"{"format":"ferroui-windows-phone","version":1,"phone":{},"assets":[]}"#)
            .is_err()
    );
    let mut phone = PhoneData::default();
    phone.settings.wallpaper = "media:photo".into();
    let item = serde_json::json!({"key":"media:photo","name":"a.jpg","mime":"image/jpeg","kind":"image","data":"cGhvdG8="});
    assert!(decode(&envelope(
        &phone,
        serde_json::json!([item.clone(), item.clone()])
    ))
    .is_err());
    for (field, value) in [
        ("key", "media:../../outside"),
        ("key", "media:unreferenced"),
        ("kind", "audio"),
        ("data", "Zh=="),
        ("data", ""),
        ("mime", "audio/mpeg"),
    ] {
        let mut bad = item.clone();
        bad[field] = value.into();
        assert!(
            decode(&envelope(&phone, serde_json::json!([bad]))).is_err(),
            "{field}={value}"
        );
    }
    let mut future: serde_json::Value =
        serde_json::from_slice(&envelope(&phone, serde_json::json!([]))).unwrap();
    future["version"] = 2.into();
    assert!(decode(&serde_json::to_vec(&future).unwrap()).is_err());
    future["format"] = "another-app".into();
    assert!(decode(&serde_json::to_vec(&future).unwrap()).is_err());
}

#[test]
fn restore_remaps_all_media_references_and_counts_missing_payloads() {
    let mut phone = PhoneData::default();
    phone.photos.push(phone_domain::Photo {
        id: 1,
        name: "photo".into(),
        source: "media:photo".into(),
        favorite: false,
    });
    phone.tracks.push(phone_domain::MediaTrack {
        id: 2,
        title: "song".into(),
        source: "media:song".into(),
        media_kind: "audio".into(),
        artist: String::new(),
        duration_seconds: 0.0,
    });
    phone.settings.wallpaper = "media:photo".into();
    phone.settings.lock_wallpaper = "media:photo".into();
    let restored = decode(&envelope(&phone, serde_json::json!([]))).unwrap();
    assert_eq!(restored.missing, 2);
    let keys = BTreeMap::from([
        ("media:photo".into(), "media:fresh-photo".into()),
        ("media:song".into(), "media:fresh-song".into()),
    ]);
    remap(&mut phone, &keys);
    assert_eq!(phone.photos[0].source, "media:fresh-photo");
    assert_eq!(phone.tracks[0].source, "media:fresh-song");
    assert_eq!(phone.settings.wallpaper, "media:fresh-photo");
    assert_eq!(phone.settings.lock_wallpaper, "media:fresh-photo");
}
