//! Portable backup envelope shared with the web edition; no filesystem effects.
use super::encoding;
use crate::asset_store::{AssetStore, Kind, key_suffix, validate_asset};
use phone_domain::PhoneData;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

pub const ENVELOPE_LIMIT: usize = 256 * 1024 * 1024;
const ASSET_TOTAL_LIMIT: usize = 192 * 1024 * 1024;
const ASSET_COUNT_LIMIT: usize = 10_000;

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct Envelope {
    format: String,
    version: u32,
    #[serde(default)]
    exported_at: String,
    phone: Box<serde_json::value::RawValue>,
    #[serde(default)]
    missing_assets: Vec<String>,
    #[serde(default)]
    assets: Vec<EncodedAsset>,
}

#[derive(Deserialize, Serialize)]
struct EncodedAsset {
    key: String,
    name: String,
    #[serde(default)]
    mime: String,
    kind: Kind,
    data: String,
}

pub struct RestoredAsset {
    pub key: String,
    pub name: String,
    pub mime: String,
    pub kind: Kind,
    pub bytes: Vec<u8>,
}

pub struct Restored {
    pub phone: PhoneData,
    pub assets: Vec<RestoredAsset>,
    pub missing: usize,
}

pub fn sources(phone: &PhoneData) -> Result<BTreeMap<String, Kind>, String> {
    let mut sources = BTreeMap::new();
    let mut add = |source: &str, kind| -> Result<(), String> {
        if source.starts_with("media:") {
            key_suffix(source)?;
            if sources
                .insert(source.to_owned(), kind)
                .is_some_and(|previous| previous != kind)
            {
                return Err("A media file is referenced with incompatible media kinds.".into());
            }
        }
        Ok(())
    };
    for photo in &phone.photos {
        add(&photo.source, Kind::Image)?;
    }
    for track in &phone.tracks {
        add(
            &track.source,
            if track.media_kind == "video" {
                Kind::Video
            } else {
                Kind::Audio
            },
        )?;
    }
    add(&phone.settings.wallpaper, Kind::Image)?;
    add(&phone.settings.lock_wallpaper, Kind::Image)?;
    if sources.len() > ASSET_COUNT_LIMIT {
        return Err("The backup contains too many media files.".into());
    }
    Ok(sources)
}

pub fn export(phone: &PhoneData, store: &AssetStore) -> Result<(Vec<u8>, usize), String> {
    let phone_json = phone.to_json()?;
    PhoneData::from_json(&phone_json)?;
    let mut assets = Vec::new();
    let mut missing = Vec::new();
    let mut total = 0usize;
    for (key, kind) in sources(phone)? {
        let asset = match store.get(&key) {
            Ok(asset) if asset.metadata.kind == kind => asset,
            _ => {
                missing.push(key);
                continue;
            }
        };
        total = total
            .checked_add(asset.bytes.len())
            .ok_or("Backup media is too large.")?;
        if total > ASSET_TOTAL_LIMIT {
            return Err("Backup media exceeds the 192 MB portable backup limit.".into());
        }
        assets.push(EncodedAsset {
            key,
            name: asset.metadata.name,
            mime: asset.metadata.mime,
            kind,
            data: encoding::encode(&asset.bytes),
        });
    }
    let missing_count = missing.len();
    let envelope = Envelope {
        format: "ferroui-windows-phone".into(),
        version: 1,
        exported_at: chrono::Utc::now().to_rfc3339(),
        phone: serde_json::value::RawValue::from_string(phone_json)
            .map_err(|error| error.to_string())?,
        missing_assets: missing,
        assets,
    };
    let bytes = serde_json::to_vec(&envelope).map_err(|error| error.to_string())?;
    if bytes.len() > ENVELOPE_LIMIT {
        return Err("The backup exceeds the 256 MB portable backup limit.".into());
    }
    Ok((bytes, missing_count))
}

pub fn decode(bytes: &[u8]) -> Result<Restored, String> {
    if bytes.len() > ENVELOPE_LIMIT {
        return Err("The backup exceeds the 256 MB import limit.".into());
    }
    #[derive(Deserialize)]
    struct Probe {
        format: Option<String>,
    }
    let probe: Probe =
        serde_json::from_slice(bytes).map_err(|error| format!("Invalid backup: {error}"))?;
    if probe.format.is_none() {
        let text = std::str::from_utf8(bytes).map_err(|_| "Backup must be UTF-8 JSON.")?;
        let phone = decode_phone(text)?;
        let missing = sources(&phone)?.len();
        return Ok(Restored {
            phone,
            assets: Vec::new(),
            missing,
        });
    }
    if probe.format.as_deref() != Some("ferroui-windows-phone") {
        return Err("This is not a supported Windows Phone backup.".into());
    }
    let envelope: Envelope =
        serde_json::from_slice(bytes).map_err(|error| format!("Invalid media backup: {error}"))?;
    if envelope.version != 1 {
        return Err("This backup version is not supported.".into());
    }
    if envelope.assets.len() > ASSET_COUNT_LIMIT
        || envelope.missing_assets.len() > ASSET_COUNT_LIMIT
    {
        return Err("Invalid media manifest: too many entries.".into());
    }
    let phone = decode_phone(envelope.phone.get())?;
    let sources = sources(&phone)?;
    let mut seen = BTreeSet::new();
    let mut assets = Vec::with_capacity(envelope.assets.len());
    let mut total = 0usize;
    for asset in envelope.assets {
        key_suffix(&asset.key)?;
        if !seen.insert(asset.key.clone()) {
            return Err("Duplicate media identifier in backup.".into());
        }
        if sources.get(&asset.key) != Some(&asset.kind) {
            return Err("Backup media is unreferenced or has an incompatible kind.".into());
        }
        let bytes = encoding::decode(&asset.data, asset.kind.limit())?;
        validate_asset(&asset.name, &asset.mime, asset.kind, bytes.len())?;
        total = total
            .checked_add(bytes.len())
            .ok_or("Backup media is too large.")?;
        if total > ASSET_TOTAL_LIMIT {
            return Err("Backup media exceeds the 192 MB decoded limit.".into());
        }
        assets.push(RestoredAsset {
            key: asset.key,
            name: asset.name,
            mime: asset.mime,
            kind: asset.kind,
            bytes,
        });
    }
    let missing = sources.keys().filter(|key| !seen.contains(*key)).count();
    Ok(Restored {
        phone,
        assets,
        missing,
    })
}

fn decode_phone(json: &str) -> Result<PhoneData, String> {
    #[derive(Deserialize)]
    struct Version {
        version: u32,
    }
    let version: Version = serde_json::from_str(json)
        .map_err(|_| "This backup is missing a phone document version.")?;
    if version.version != phone_domain::DATA_VERSION {
        return Err("This phone document version is not supported.".into());
    }
    PhoneData::from_json(json)
}

pub fn remap(phone: &mut PhoneData, keys: &BTreeMap<String, String>) {
    let replace = |source: &mut String| {
        if let Some(key) = keys.get(source) {
            *source = key.clone();
        }
    };
    for photo in &mut phone.photos {
        replace(&mut photo.source);
    }
    for track in &mut phone.tracks {
        replace(&mut track.source);
    }
    replace(&mut phone.settings.wallpaper);
    replace(&mut phone.settings.lock_wallpaper);
}

#[cfg(test)]
mod tests;
