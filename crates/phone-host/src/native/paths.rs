use std::{env, path::PathBuf};

/// The app-private data directory for the current host. `application` must be a
/// single portable identifier; it is a program constant, never a document name.
pub fn data_directory(application: &str) -> PathBuf {
    assert!(
        !application.is_empty()
            && application.len() <= 128
            && application
                .bytes()
                .all(|c| c.is_ascii_alphanumeric() || b"_-".contains(&c))
    );
    let absolute = |name| {
        env::var_os(name)
            .map(PathBuf::from)
            .filter(|p| p.is_absolute())
    };
    #[cfg(target_os = "windows")]
    let base = absolute("LOCALAPPDATA").or_else(|| absolute("APPDATA"));
    #[cfg(target_os = "macos")]
    let base = absolute("HOME").map(|p| p.join("Library/Application Support"));
    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    let base =
        absolute("XDG_DATA_HOME").or_else(|| absolute("HOME").map(|p| p.join(".local/share")));
    base.unwrap_or_else(env::temp_dir).join(application)
}
