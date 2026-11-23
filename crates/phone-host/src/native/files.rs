use std::{
    fs::{self, OpenOptions},
    io::{self, Write},
    path::Path,
    sync::atomic::{AtomicU64, Ordering},
};

static NEXT_FILE: AtomicU64 = AtomicU64::new(0);

/// Replace an app-owned file without truncating the previous copy first.
/// Same-directory create_new staging also prevents concurrent writes from
/// sharing a temporary file. Picker-selected files use their storage provider.
pub fn atomic_write(path: &Path, bytes: &[u8]) -> io::Result<()> {
    let parent = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "Missing data directory"))?;
    fs::create_dir_all(parent)?;
    let mut temporary = None;
    let mut output = None;
    for _ in 0..32 {
        let sequence = NEXT_FILE.fetch_add(1, Ordering::Relaxed);
        let candidate = parent.join(format!(
            ".phone-write-{}-{sequence}.tmp",
            std::process::id()
        ));
        let mut options = OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        match options.open(&candidate) {
            Ok(file) => {
                temporary = Some(candidate);
                output = Some(file);
                break;
            }
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
            Err(error) => return Err(error),
        }
    }
    let temporary = temporary
        .ok_or_else(|| io::Error::new(io::ErrorKind::AlreadyExists, "No staging file available"))?;
    let result = (|| {
        let mut output = output.expect("staging file and path are created together");
        output.write_all(bytes)?;
        output.sync_all()?;
        drop(output);
        replace(&temporary, path)?;
        #[cfg(unix)]
        if let Ok(directory) = fs::File::open(parent) {
            let _ = directory.sync_all();
        }
        Ok(())
    })();
    if result.is_err() {
        let _ = fs::remove_file(temporary);
    }
    result
}

#[cfg(not(target_os = "windows"))]
fn replace(from: &Path, to: &Path) -> io::Result<()> {
    fs::rename(from, to)
}

#[cfg(target_os = "windows")]
fn replace(from: &Path, to: &Path) -> io::Result<()> {
    use std::os::windows::ffi::OsStrExt;
    #[link(name = "kernel32")]
    extern "system" {
        fn MoveFileExW(from: *const u16, to: *const u16, flags: u32) -> i32;
    }
    let encode = |path: &Path| -> io::Result<Vec<u16>> {
        let mut value: Vec<_> = path.as_os_str().encode_wide().collect();
        if value.contains(&0) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "NUL in file path",
            ));
        }
        value.push(0);
        Ok(value)
    };
    let from = encode(from)?;
    let to = encode(to)?;
    // Both NUL-terminated path buffers outlive the synchronous Win32 call.
    if unsafe { MoveFileExW(from.as_ptr(), to.as_ptr(), 0x1 | 0x8) } == 0 {
        Err(io::Error::last_os_error())
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn replacement_preserves_previous_file_if_commit_fails() {
        let directory =
            std::env::temp_dir().join(format!("phone-atomic-test-{}", std::process::id()));
        fs::create_dir_all(&directory).unwrap();
        let file = directory.join("data.json");
        atomic_write(&file, b"first").unwrap();
        atomic_write(&file, b"second").unwrap();
        assert_eq!(fs::read(&file).unwrap(), b"second");
        let blocked = directory.join("directory");
        fs::create_dir_all(&blocked).unwrap();
        assert!(atomic_write(&blocked, b"invalid").is_err());
        assert!(blocked.is_dir());
        assert_eq!(fs::read_dir(&directory).unwrap().count(), 2);
        fs::remove_dir_all(directory).unwrap();
    }
}
