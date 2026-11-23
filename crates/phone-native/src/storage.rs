use phone_domain::PhoneData;
use std::{cell::RefCell, io, path::PathBuf};

thread_local! { static ACTIVE_PATH: RefCell<Option<PathBuf>> = const { RefCell::new(None) }; }

pub fn set_active_data_path(path: PathBuf) {
    ACTIVE_PATH.with(|slot| *slot.borrow_mut() = Some(path));
}

pub fn active_data_path() -> PathBuf {
    ACTIVE_PATH
        .with(|slot| slot.borrow().clone())
        .unwrap_or_else(data_path)
}

pub fn data_path() -> PathBuf {
    phone_host::native::data_directory("ferroui-windows-phone").join("data.json")
}

fn legacy_data_path() -> PathBuf {
    std::env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("APPDATA").map(PathBuf::from))
        .or_else(|| std::env::var_os("HOME").map(|p| PathBuf::from(p).join(".local/share")))
        .unwrap_or_else(std::env::temp_dir)
        .join("ferroui-windows-phone/data.json")
}

pub fn load(path: &std::path::Path) -> (PhoneData, Option<String>, PathBuf) {
    let legacy = legacy_data_path();
    if cfg!(any(
        target_os = "windows",
        target_os = "macos",
        target_os = "linux"
    )) && !path.exists()
        && path != legacy
        && legacy.is_file()
    {
        let (data, warning, original) = load_existing(&legacy);
        if warning.is_none() {
            match save(path, &data) {
                Ok(()) => return (data, Some("Your existing phone data was copied to this platform's application data folder.".into()), path.to_owned()),
                Err(error) => return (data, Some(format!("Could not move existing data; continuing in the previous folder. {error}")), original),
            }
        }
        // A damaged legacy document retains its recovery behavior and location.
        return (data, warning, original);
    }
    load_existing(path)
}

fn load_existing(path: &std::path::Path) -> (PhoneData, Option<String>, PathBuf) {
    match std::fs::read_to_string(path) {
        Ok(json) => match PhoneData::from_json(&json) {
            Ok(data) => (data, None, path.to_owned()),
            Err(error) => {
                let recovery = path.with_extension(format!(
                    "recovery-{}.json",
                    chrono::Utc::now().timestamp_millis()
                ));
                let preserved = std::fs::copy(path, &recovery).is_ok();
                (
                    PhoneData::demo(),
                    Some(format!(
                        "Saved data could not be loaded. {} {error}",
                        if preserved {
                            "A recovery copy was preserved beside data.json."
                        } else {
                            "The original file remains on disk; changes will be saved in a separate recovery document."
                        }
                    )),
                    if preserved {
                        path.to_owned()
                    } else {
                        path.with_file_name("data-recovered.json")
                    },
                )
            }
        },
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            (PhoneData::demo(), None, path.to_owned())
        }
        Err(error) => (
            PhoneData::demo(),
            Some(format!("Saved data could not be read: {error}")),
            path.with_file_name("data-recovered.json"),
        ),
    }
}

pub fn save(path: &std::path::Path, data: &PhoneData) -> Result<(), String> {
    let json = data.to_json()?;
    phone_host::native::atomic_write(path, json.as_bytes()).map_err(|e| e.to_string())
}
