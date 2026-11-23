//! File-picker imports and image requests for each native storage provider.
use crate::asset_store::{AssetStore, Kind};
use ferroui_base::{
    platform::storage::{
        FilePickerFileType, FilePickerOpenOptions, FilePickerSaveOptions, IStorageProvider,
    },
    threading::Dispatcher,
};
use phone_state::{HostRequest, PhoneViewModel};
use std::{cell::Cell, path::PathBuf, rc::Rc};

thread_local! { static IMAGE_GENERATION: Cell<u64> = const { Cell::new(0) }; }

pub fn install_image_loader() {
    phone_ui::set_image_loader(Rc::new(|key| {
        let generation = IMAGE_GENERATION.with(Cell::get);
        drop(Dispatcher::current_dispatcher().spawn_local(async move {
            Dispatcher::yield_now().await;
            if IMAGE_GENERATION.with(Cell::get) != generation {
                return;
            }
            let source = key.clone();
            let result = crate::background::run(move || AssetStore::default().get(&source)).await;
            if IMAGE_GENERATION.with(Cell::get) != generation {
                return;
            }
            let result = result.and_then(|asset| {
                if asset.metadata.kind != Kind::Image {
                    return Err("This media file is not a photo.".into());
                }
                phone_ui::register_image(key.clone(), asset.bytes)
            });
            if result.is_err() {
                phone_ui::image_failed(key);
            }
        }));
    }));
}

pub fn clear_images() {
    IMAGE_GENERATION.with(|value| value.set(value.get().wrapping_add(1)));
    phone_ui::clear_images();
}

pub fn start(vm: Rc<PhoneViewModel>, provider: Rc<dyn IStorageProvider>, request: HostRequest) {
    let kind = match request {
        HostRequest::ImportPhotos => Kind::Image,
        HostRequest::ImportMusic => Kind::Audio,
        HostRequest::ImportVideo => Kind::Video,
        _ => unreachable!("native media import"),
    };
    drop(Dispatcher::current_dispatcher().spawn_local(async move {
        if let Err(error) = import(&vm, provider, kind).await {
            vm.set_status(error);
        }
    }));
}

async fn import(
    vm: &PhoneViewModel,
    provider: Rc<dyn IStorageProvider>,
    kind: Kind,
) -> Result<(), String> {
    let (title, patterns) = match kind {
        Kind::Image => (
            "Choose photos",
            vec!["*.jpg", "*.jpeg", "*.png", "*.gif", "*.webp"],
        ),
        Kind::Audio => (
            "Choose music",
            vec![
                "*.mp3", "*.wav", "*.flac", "*.ogg", "*.opus", "*.m4a", "*.aac", "*.wma",
            ],
        ),
        Kind::Video => (
            "Choose videos",
            vec![
                "*.mp4", "*.m4v", "*.mov", "*.mkv", "*.webm", "*.avi", "*.wmv", "*.mpg", "*.mpeg",
            ],
        ),
    };
    let mut options = FilePickerOpenOptions::new();
    options.set_title(Some(title.into()));
    options.set_allow_multiple(true);
    options.set_file_type_filter(Some(vec![
        FilePickerFileType::new(Some(title)).with_patterns(&patterns),
    ]));
    let files = provider
        .open_file_picker_async(options)
        .await
        .map_err(|error| error.to_string())?;
    if files.len() > 512 {
        return Err("Choose at most 512 media files at once.".into());
    }
    let store = AssetStore::default();
    let mut count = 0;
    for file in files {
        let name = file.name();
        let bytes = crate::picker_io::read(file.as_ref(), kind.limit()).await?;
        let writer = store.clone();
        let (key, name, bytes) = crate::background::run(move || {
            let key = writer.put(&name, "", kind, &bytes)?;
            Ok((key, name, bytes))
        })
        .await?;
        if kind == Kind::Image {
            // Use the same bounded codec admission as browser-loaded images.
            if let Err(error) = phone_ui::register_image(key.clone(), bytes) {
                let _ = store.remove(&key);
                return Err(format!("{name}: {error} Imported {count} earlier file(s)."));
            }
            vm.add_photo(name, key);
        } else {
            let title = PathBuf::from(&name)
                .file_stem()
                .and_then(|value| value.to_str())
                .unwrap_or(&name)
                .to_owned();
            vm.add_media(
                title,
                key,
                if kind == Kind::Audio {
                    "audio"
                } else {
                    "video"
                }
                .into(),
            );
        }
        count += 1;
        Dispatcher::yield_now().await;
    }
    if count > 0 {
        vm.set_status(format!(
            "Imported {count} media file(s). Copies are stored with this app."
        ));
    }
    Ok(())
}

/// Only app-owned, validated files can be handed to an OS media application.
pub fn open_media(source: &str) -> Result<(PathBuf, String), String> {
    let (path, metadata) = AssetStore::default().playable(source)?;
    Ok((path, metadata.name))
}

pub fn remove(source: &str) -> Result<(), String> {
    if !source.starts_with("media:") {
        return Ok(());
    }
    AssetStore::default().remove(source)
}

/// Preserve real media bytes for a desktop attachment workflow. This does not
/// claim that another application received the file or that a share sheet ran.
pub fn share_copy(vm: Rc<PhoneViewModel>, provider: Rc<dyn IStorageProvider>, source: String) {
    drop(Dispatcher::current_dispatcher().spawn_local(async move {
        if let Err(error) = save_copy(&vm, provider, source).await {
            vm.set_status(error);
        }
    }));
}

async fn save_copy(
    vm: &PhoneViewModel,
    provider: Rc<dyn IStorageProvider>,
    source: String,
) -> Result<(), String> {
    let asset = crate::background::run(move || AssetStore::default().get(&source)).await?;
    let mut options = FilePickerSaveOptions::new();
    options.set_title(Some("Save a media copy to share".into()));
    options.set_suggested_file_name(Some(copy_name(&asset.metadata)));
    options.set_show_overwrite_prompt(Some(true));
    let Some(file) = provider
        .save_file_picker_async(options)
        .await
        .map_err(|error| error.to_string())?
    else {
        return Ok(());
    };
    crate::picker_io::write(file.as_ref(), &asset.bytes).await?;
    vm.set_status(format!(
        "Saved {}. Attach this copy in your messaging or sharing app.",
        file.name()
    ));
    Ok(())
}

fn copy_name(metadata: &crate::asset_store::Metadata) -> String {
    let mut clean = String::new();
    for value in metadata.name.chars() {
        let value = if value.is_control() || "<>:\"/\\|?*".contains(value) {
            '_'
        } else {
            value
        };
        if clean.len() + value.len_utf8() > 120 {
            break;
        }
        clean.push(value);
    }
    let stem = clean
        .rsplit_once('.')
        .map(|(stem, _)| stem)
        .unwrap_or(&clean)
        .trim_matches([' ', '.']);
    // The prefix also avoids DOS device names and leading-dot hidden files.
    format!(
        "WindowsPhone-{}.{}",
        if stem.is_empty() { "media" } else { stem },
        metadata.extension
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn exported_media_names_keep_the_actual_format_and_safe_leaf_name() {
        let metadata = crate::asset_store::Metadata {
            key: "media:a".into(),
            name: "Photo 11/23/2026: 12:00.PNG".into(),
            mime: "image/jpeg".into(),
            kind: Kind::Image,
            extension: "jpg".into(),
        };
        assert_eq!(
            copy_name(&metadata),
            "WindowsPhone-Photo 11_23_2026_ 12_00.jpg"
        );
    }
}
