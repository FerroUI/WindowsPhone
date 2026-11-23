//! Bounded, cooperative I/O shared by native file-picker workflows.
use ferroui_base::{platform::storage::IStorageFile, threading::Dispatcher};
use std::io::Read;

const CHUNK: usize = 256 * 1024;

pub async fn read_while(
    file: &dyn IStorageFile,
    limit: usize,
    active: impl Fn() -> bool,
) -> Result<Vec<u8>, String> {
    if !active() {
        return Err("File selection was cancelled.".into());
    }
    if file
        .get_basic_properties_async()
        .await
        .size()
        .is_some_and(|size| size > limit as u64)
    {
        return Err(format!(
            "{} exceeds the {} limit.",
            file.name(),
            limit_text(limit)
        ));
    }
    if !active() {
        return Err("File selection was cancelled.".into());
    }
    let mut stream = file
        .open_read_async()
        .await
        .map_err(|error| error.to_string())?;
    let mut bytes = Vec::new();
    let mut chunk = vec![0; CHUNK];
    loop {
        if !active() {
            return Err("File selection was cancelled.".into());
        }
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
                "{} exceeds the {} limit.",
                file.name(),
                limit_text(limit)
            ));
        }
        bytes.extend_from_slice(&chunk[..count]);
        Dispatcher::yield_now().await;
    }
}

fn limit_text(limit: usize) -> String {
    if limit >= 1024 * 1024 && limit % (1024 * 1024) == 0 {
        format!("{} MiB", limit / 1024 / 1024)
    } else if limit >= 1024 && limit % 1024 == 0 {
        format!("{} KiB", limit / 1024)
    } else {
        format!("{limit} byte")
    }
}
