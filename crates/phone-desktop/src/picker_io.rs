//! Bounded, cooperative I/O shared by native file-picker workflows.
use ferroui_base::{platform::storage::IStorageFile, threading::Dispatcher};
use std::io::{Read, Write};

const CHUNK: usize = 256 * 1024;

pub async fn read(file: &dyn IStorageFile, limit: usize) -> Result<Vec<u8>, String> {
    if file
        .get_basic_properties_async()
        .await
        .size()
        .is_some_and(|size| size > limit as u64)
    {
        return Err(format!(
            "{} exceeds the {} MB limit.",
            file.name(),
            limit / 1024 / 1024
        ));
    }
    let mut stream = file
        .open_read_async()
        .await
        .map_err(|error| error.to_string())?;
    let mut bytes = Vec::new();
    let mut chunk = vec![0; CHUNK];
    loop {
        let count = stream.read(&mut chunk).map_err(|error| error.to_string())?;
        if count == 0 {
            return Ok(bytes);
        }
        if bytes
            .len()
            .checked_add(count)
            .is_none_or(|size| size > limit)
        {
            return Err(format!(
                "{} exceeds the {} MB limit.",
                file.name(),
                limit / 1024 / 1024
            ));
        }
        bytes.extend_from_slice(&chunk[..count]);
        Dispatcher::yield_now().await;
    }
}

pub async fn write(file: &dyn IStorageFile, bytes: &[u8]) -> Result<(), String> {
    let mut stream = file
        .open_write_async()
        .await
        .map_err(|error| error.to_string())?;
    for chunk in bytes.chunks(CHUNK) {
        stream.write_all(chunk).map_err(|error| error.to_string())?;
        Dispatcher::yield_now().await;
    }
    stream.flush().map_err(|error| error.to_string())
}
