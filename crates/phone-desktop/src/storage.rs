use phone_domain::PhoneData;
use std::{io, path::PathBuf};

pub fn data_path() -> PathBuf {
    std::env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("APPDATA").map(PathBuf::from))
        .or_else(|| std::env::var_os("HOME").map(|p| PathBuf::from(p).join(".local/share")))
        .unwrap_or_else(std::env::temp_dir)
        .join("ferroui-windows-phone/data.json")
}

pub fn load(path: &std::path::Path) -> (PhoneData, Option<String>, PathBuf) {
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
    let parent = path.parent().ok_or("Missing data directory")?;
    std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    let temporary = path.with_extension("tmp");
    std::fs::write(&temporary, json).map_err(|e| e.to_string())?;
    std::fs::rename(temporary, path).map_err(|e| e.to_string())
}
