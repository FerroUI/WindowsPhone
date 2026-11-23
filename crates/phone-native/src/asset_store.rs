//! App-owned media files. A persisted media key is never interpreted as a path.
use serde::{Deserialize, Serialize};
use std::{
    fs::{self, OpenOptions},
    io::{Read, Write},
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
    time::{SystemTime, UNIX_EPOCH},
};

pub const IMAGE_LIMIT: usize = 16 * 1024 * 1024;
pub const MEDIA_LIMIT: usize = 128 * 1024 * 1024;
static SERIAL: AtomicU64 = AtomicU64::new(0);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Kind {
    Image,
    Audio,
    Video,
}

impl Kind {
    pub fn limit(self) -> usize {
        if self == Self::Image {
            IMAGE_LIMIT
        } else {
            MEDIA_LIMIT
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Metadata {
    pub key: String,
    pub name: String,
    pub mime: String,
    pub kind: Kind,
    pub extension: String,
}

pub struct Asset {
    pub metadata: Metadata,
    pub bytes: Vec<u8>,
}

#[derive(Clone)]
pub struct AssetStore {
    root: PathBuf,
}

impl Default for AssetStore {
    fn default() -> Self {
        Self::new(crate::storage::active_data_path().with_file_name("assets"))
    }
}

impl AssetStore {
    pub fn new(root: PathBuf) -> Self {
        Self { root }
    }

    pub fn put(&self, name: &str, mime: &str, kind: Kind, bytes: &[u8]) -> Result<String, String> {
        validate_asset(name, mime, kind, bytes.len())?;
        let extension = extension(name, mime, kind)?.to_owned();
        fs::create_dir_all(&self.root).map_err(|error| error.to_string())?;
        let key = unique_key();
        let metadata = Metadata {
            key: key.clone(),
            name: name.into(),
            mime: mime.into(),
            kind,
            extension,
        };
        let path = self.data_path(&metadata)?;
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
            .map_err(|error| error.to_string())?;
        let mut manifest_created = false;
        let result = (|| {
            file.write_all(bytes).map_err(|error| error.to_string())?;
            file.sync_all().map_err(|error| error.to_string())?;
            let json = serde_json::to_vec(&metadata).map_err(|error| error.to_string())?;
            let mut manifest = OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(self.metadata_path(&key)?)
                .map_err(|error| error.to_string())?;
            manifest_created = true;
            manifest
                .write_all(&json)
                .map_err(|error| error.to_string())?;
            manifest.sync_all().map_err(|error| error.to_string())
        })();
        if let Err(error) = result {
            drop(file);
            let _ = fs::remove_file(path);
            if manifest_created {
                let _ = fs::remove_file(self.metadata_path(&key)?);
            }
            return Err(error);
        }
        Ok(key)
    }

    pub fn get(&self, key: &str) -> Result<Asset, String> {
        let metadata = self.metadata(key)?;
        let bytes = read_bounded(&self.data_path(&metadata)?, metadata.kind.limit())?;
        validate_asset(&metadata.name, &metadata.mime, metadata.kind, bytes.len())?;
        Ok(Asset { metadata, bytes })
    }

    pub fn playable(&self, key: &str) -> Result<(PathBuf, Metadata), String> {
        let metadata = self.metadata(key)?;
        if metadata.kind == Kind::Image {
            return Err("Choose an audio or video file.".into());
        }
        let path = self.data_path(&metadata)?;
        let size = regular_file(&path)?.len();
        if size == 0 || size > metadata.kind.limit() as u64 {
            return Err("This media file is invalid or exceeds the import limit.".into());
        }
        Ok((path, metadata))
    }

    pub fn remove(&self, key: &str) -> Result<(), String> {
        let metadata = match self.metadata(key) {
            Ok(metadata) => metadata,
            Err(_) if !self.metadata_path(key)?.exists() => return Ok(()),
            Err(error) => return Err(error),
        };
        match fs::remove_file(self.data_path(&metadata)?) {
            Ok(()) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(error.to_string()),
        }
        fs::remove_file(self.metadata_path(key)?).map_err(|error| error.to_string())
    }

    fn metadata(&self, key: &str) -> Result<Metadata, String> {
        let bytes = read_bounded(&self.metadata_path(key)?, 4096)?;
        let metadata: Metadata =
            serde_json::from_slice(&bytes).map_err(|error| error.to_string())?;
        validate_asset(&metadata.name, &metadata.mime, metadata.kind, 1)?;
        if metadata.key != key
            || metadata.extension != extension(&metadata.name, &metadata.mime, metadata.kind)?
        {
            return Err("This media manifest is invalid.".into());
        }
        Ok(metadata)
    }

    fn metadata_path(&self, key: &str) -> Result<PathBuf, String> {
        Ok(self.root.join(format!("{}.json", key_suffix(key)?)))
    }

    fn data_path(&self, metadata: &Metadata) -> Result<PathBuf, String> {
        let suffix = key_suffix(&metadata.key)?;
        let ext = extension(&metadata.name, &metadata.mime, metadata.kind)?;
        Ok(self.root.join(format!("{suffix}.{ext}")))
    }
}

pub fn unique_key() -> String {
    let time = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    format!(
        "media:desktop-{time}-{}-{}",
        std::process::id(),
        SERIAL.fetch_add(1, Ordering::Relaxed)
    )
}

pub fn key_suffix(key: &str) -> Result<&str, String> {
    key.strip_prefix("media:")
        .filter(|suffix| {
            !suffix.is_empty()
                && suffix.len() <= 128
                && suffix.as_bytes()[0].is_ascii_alphanumeric()
                && suffix
                    .bytes()
                    .all(|value| value.is_ascii_alphanumeric() || b"-_".contains(&value))
        })
        .ok_or_else(|| "Invalid media identifier.".into())
}

pub fn validate_asset(name: &str, mime: &str, kind: Kind, size: usize) -> Result<(), String> {
    // Legacy browser camera captions can contain slashes. Names are display
    // metadata only: on-disk paths use the validated key and approved extension.
    if name.is_empty() || name.len() > 1024 || name.chars().any(char::is_control) {
        return Err("Invalid media file name.".into());
    }
    if mime.len() > 128 || mime.chars().any(char::is_control) || size == 0 || size > kind.limit() {
        return Err("The media file is empty, invalid, or exceeds its import limit.".into());
    }
    extension(name, mime, kind).map(|_| ())
}

pub fn extension<'a>(name: &'a str, mime: &'a str, kind: Kind) -> Result<&'a str, String> {
    let by_mime = match mime.split(';').next().unwrap_or("").trim() {
        "image/jpeg" => Some((Kind::Image, "jpg")),
        "image/png" => Some((Kind::Image, "png")),
        "image/gif" => Some((Kind::Image, "gif")),
        "image/webp" => Some((Kind::Image, "webp")),
        "audio/mpeg" => Some((Kind::Audio, "mp3")),
        "audio/wav" | "audio/x-wav" => Some((Kind::Audio, "wav")),
        "audio/flac" => Some((Kind::Audio, "flac")),
        "audio/ogg" => Some((Kind::Audio, "ogg")),
        "audio/mp4" => Some((Kind::Audio, "m4a")),
        "audio/aac" => Some((Kind::Audio, "aac")),
        "video/mp4" => Some((Kind::Video, "mp4")),
        "video/webm" => Some((Kind::Video, "webm")),
        "video/quicktime" => Some((Kind::Video, "mov")),
        _ => None,
    };
    if let Some((matched, ext)) = by_mime {
        return if matched == kind {
            Ok(ext)
        } else {
            Err("The media kind does not match its content type.".into())
        };
    }
    let ext = name.rsplit('.').next().unwrap_or("");
    let allowed = match kind {
        Kind::Image => &["jpg", "jpeg", "png", "gif", "webp"][..],
        Kind::Audio => &["mp3", "wav", "flac", "ogg", "opus", "m4a", "aac", "wma"][..],
        Kind::Video => &[
            "mp4", "m4v", "mov", "mkv", "webm", "avi", "wmv", "mpg", "mpeg",
        ][..],
    };
    allowed
        .iter()
        .copied()
        .find(|value| ext.eq_ignore_ascii_case(value))
        .ok_or_else(|| "This media file type is not supported for import.".into())
}

fn regular_file(path: &Path) -> Result<fs::Metadata, String> {
    let metadata = fs::symlink_metadata(path)
        .map_err(|error| format!("This media file is unavailable: {error}"))?;
    if !metadata.is_file() {
        return Err("This media entry is not a regular file.".into());
    }
    Ok(metadata)
}

pub fn read_bounded(path: &Path, limit: usize) -> Result<Vec<u8>, String> {
    let size = regular_file(path)?.len();
    if size > limit as u64 {
        return Err("This file exceeds the import limit.".into());
    }
    let mut bytes = Vec::with_capacity(size as usize);
    fs::File::open(path)
        .map_err(|error| error.to_string())?
        .take(limit as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|error| error.to_string())?;
    if bytes.len() > limit {
        return Err("This file exceeds the import limit.".into());
    }
    Ok(bytes)
}

#[cfg(test)]
mod tests;
