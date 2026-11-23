//! App-private cache of explicitly viewed tiles. No area download/prefetch API.
//! HTTP expiry/validators are retained, with a seven-day fallback. Entries that
//! are still fresh are never evicted merely to make room for another download.
use phone_host::{
    maps,
    native::http::{self, ImageCacheHeaders, ImageResponse},
};
use serde::{Deserialize, Serialize};
use std::{
    fs,
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
    time::{SystemTime, UNIX_EPOCH},
};
const MAX_FILES: usize = 2048;
const MAX_BYTES: u64 = 64 * 1024 * 1024;
#[derive(Serialize, Deserialize)]
struct Metadata {
    expires: i64,
    size: usize,
    headers: ImageCacheHeaders,
}
pub fn get(root: PathBuf, zoom: u8, x: u32, y: u32) -> Result<Vec<u8>, String> {
    maps::validate(zoom, x, y)?;
    let now = chrono::Utc::now().timestamp();
    cached(root, zoom, x, y, now, |headers| {
        http::image(&maps::url(zoom, x, y)?, headers)
    })
}
fn cached(
    root: PathBuf,
    zoom: u8,
    x: u32,
    y: u32,
    now: i64,
    fetch: impl FnOnce(Option<&ImageCacheHeaders>) -> Result<ImageResponse, String>,
) -> Result<Vec<u8>, String> {
    maps::validate(zoom, x, y)?;
    ensure_directory(&root)?;
    let key = format!("{zoom}-{x}-{y}");
    let data = root.join(format!("{key}.png"));
    let meta = root.join(format!("{key}.json"));
    let previous = read(&data, &meta);
    if let Some((metadata, bytes)) = &previous {
        if metadata.expires > now {
            return Ok(bytes.clone());
        }
    }
    let response = fetch(previous.as_ref().map(|(v, _)| &v.headers))?;
    let revalidated = response.bytes.is_none();
    let bytes = match response.bytes {
        Some(bytes) => bytes,
        None => previous
            .as_ref()
            .map(|(_, bytes)| bytes.clone())
            .ok_or("Map service revalidated a missing tile.")?,
    };
    maps::validate_png(&bytes)?;
    if response.headers.no_store {
        let _ = fs::remove_file(&data);
        let _ = fs::remove_file(&meta);
        return Ok(bytes);
    }
    let mut headers = response.headers;
    if let Some((previous, _)) = &previous {
        if revalidated && headers.max_age.is_none() && headers.expires.is_none() {
            headers.max_age = previous.headers.max_age;
            headers.expires = previous.headers.expires.clone();
        }
        if headers.etag.is_none() {
            headers.etag = previous.headers.etag.clone();
        }
        if headers.last_modified.is_none() {
            headers.last_modified = previous.headers.last_modified.clone();
        }
    }
    let expires = headers
        .max_age
        .map(|v| now.saturating_add(v.min(366 * 86400) as i64))
        .or_else(|| {
            headers
                .expires
                .as_deref()
                .and_then(|v| chrono::DateTime::parse_from_rfc2822(v).ok())
                .map(|v| v.timestamp())
        })
        .unwrap_or_else(|| now.saturating_add(maps::CACHE_SECONDS as i64));
    make_room(&root, &key, bytes.len() as u64, now)?;
    let metadata = Metadata {
        expires,
        size: bytes.len(),
        headers,
    };
    write_atomic(&data, &bytes)?;
    if let Err(error) = write_atomic(
        &meta,
        &serde_json::to_vec(&metadata).map_err(|e| e.to_string())?,
    ) {
        let _ = fs::remove_file(&data);
        let _ = fs::remove_file(&meta);
        return Err(error);
    }
    Ok(bytes)
}
fn ensure_directory(root: &Path) -> Result<(), String> {
    fs::create_dir_all(root).map_err(|e| format!("Cannot create map cache: {e}"))?;
    let metadata = fs::symlink_metadata(root).map_err(|e| e.to_string())?;
    if !metadata.is_dir() || metadata.file_type().is_symlink() {
        return Err("The map cache must be an app-owned directory.".into());
    }
    Ok(())
}
fn regular(path: &Path, limit: u64) -> Option<Vec<u8>> {
    let metadata = fs::symlink_metadata(path).ok()?;
    if !metadata.is_file() || metadata.file_type().is_symlink() || metadata.len() > limit {
        return None;
    }
    // The cache contains public map data, never a user-selected external path.
    let file = fs::File::open(path).ok()?;
    let after = file.metadata().ok()?;
    if !after.is_file() || after.len() > limit {
        return None;
    }
    use std::io::Read;
    let mut bytes = Vec::new();
    file.take(limit + 1).read_to_end(&mut bytes).ok()?;
    (bytes.len() as u64 <= limit).then_some(bytes)
}
fn read(data: &Path, meta: &Path) -> Option<(Metadata, Vec<u8>)> {
    let metadata = read_metadata(data, meta)?;
    let bytes = regular(data, maps::MAX_TILE_BYTES as u64)?;
    if metadata.size != bytes.len() || maps::validate_png(&bytes).is_err() {
        return None;
    }
    Some((metadata, bytes))
}
fn read_metadata(data: &Path, meta: &Path) -> Option<Metadata> {
    let metadata: Metadata = serde_json::from_slice(&regular(meta, 8192)?).ok()?;
    let file = fs::symlink_metadata(data).ok()?;
    if !file.is_file()
        || file.file_type().is_symlink()
        || metadata.size > maps::MAX_TILE_BYTES as usize
        || metadata.size as u64 != file.len()
    {
        return None;
    }
    Some(metadata)
}
fn make_room(root: &Path, key: &str, incoming: u64, now: i64) -> Result<(), String> {
    let mut count = 0usize;
    let mut used = 0u64;
    for (index, entry) in fs::read_dir(root).map_err(|e| e.to_string())?.enumerate() {
        if index > MAX_FILES * 3 + 16 {
            return Err("The viewed map cache exceeds its entry limit.".into());
        }
        let entry = entry.map_err(|e| e.to_string())?;
        let path = entry.path();
        if path.extension().and_then(|v| v.to_str()) != Some("json") {
            continue;
        }
        let Some(stem) = path.file_stem().and_then(|v| v.to_str()) else {
            continue;
        };
        if stem == key {
            continue;
        }
        let data = root.join(format!("{stem}.png"));
        // Account using small metadata records, never reread all cached PNGs.
        if let Some(metadata) = read_metadata(&data, &path) {
            if metadata.expires <= now {
                let _ = fs::remove_file(&data);
                let _ = fs::remove_file(&path);
            } else {
                count += 1;
                used = used.saturating_add(metadata.size as u64);
            }
        } else {
            let _ = fs::remove_file(&data);
            let _ = fs::remove_file(&path);
        }
    }
    if count >= MAX_FILES || used.saturating_add(incoming) > MAX_BYTES {
        return Err("The map cache is full of recently viewed tiles. Previously viewed maps remain available.".into());
    }
    Ok(())
}
fn write_atomic(path: &Path, bytes: &[u8]) -> Result<(), String> {
    use std::io::Write;
    static NEXT: AtomicU64 = AtomicU64::new(0);
    let serial = NEXT.fetch_add(1, Ordering::Relaxed);
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let temp = path.with_extension(format!("tmp-{}-{stamp}-{serial}", std::process::id()));
    let result = (|| {
        let mut file = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temp)
            .map_err(|e| e.to_string())?;
        file.write_all(bytes).map_err(|e| e.to_string())?;
        file.sync_all().map_err(|e| e.to_string())?;
        if path.exists() {
            let metadata = fs::symlink_metadata(path).map_err(|e| e.to_string())?;
            if !metadata.is_file() || metadata.file_type().is_symlink() {
                return Err("Invalid map cache file.".into());
            }
        }
        #[cfg(windows)]
        if path.exists() {
            fs::remove_file(path).map_err(|e| e.to_string())?;
        }
        fs::rename(&temp, path).map_err(|e| e.to_string())
    })();
    if result.is_err() {
        let _ = fs::remove_file(temp);
    }
    result
}
#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;
    fn png() -> Vec<u8> {
        let mut v = vec![0; 33];
        v[..8].copy_from_slice(b"\x89PNG\r\n\x1a\n");
        v[8..12].copy_from_slice(&13u32.to_be_bytes());
        v[12..16].copy_from_slice(b"IHDR");
        v[16..20].copy_from_slice(&256u32.to_be_bytes());
        v[20..24].copy_from_slice(&256u32.to_be_bytes());
        v
    }
    #[test]
    fn fresh_tiles_do_not_repeat_requests_and_expired_tiles_revalidate() {
        let root = std::env::temp_dir().join(format!(
            "phone-map-test-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let calls = Cell::new(0);
        let bytes = cached(root.clone(), 2, 1, 1, 100, |_| {
            calls.set(1);
            Ok(ImageResponse {
                bytes: Some(png()),
                headers: ImageCacheHeaders {
                    etag: Some("tag".into()),
                    ..Default::default()
                },
            })
        })
        .unwrap();
        assert_eq!(
            cached(root.clone(), 2, 1, 1, 200, |_| panic!(
                "fresh cache made a network request"
            ))
            .unwrap(),
            bytes
        );
        cached(
            root.clone(),
            2,
            1,
            1,
            100 + maps::CACHE_SECONDS as i64,
            |previous| {
                assert_eq!(previous.unwrap().etag.as_deref(), Some("tag"));
                calls.set(2);
                Ok(ImageResponse {
                    bytes: None,
                    headers: ImageCacheHeaders {
                        max_age: Some(600),
                        ..Default::default()
                    },
                })
            },
        )
        .unwrap();
        assert_eq!(calls.get(), 2);
        fs::remove_dir_all(root).unwrap();
    }
}
