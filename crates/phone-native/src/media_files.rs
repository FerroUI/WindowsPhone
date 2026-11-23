//! File-picker imports and image requests for each native storage provider.
use crate::asset_store::{AssetStore, Kind};
use crate::operations::{Lease, StagedAssets};
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
    IMAGE_GENERATION.with(|value| value.set(value.get().wrapping_add(1)));
    let owner_path = crate::storage::active_data_path();
    phone_ui::set_image_loader(Rc::new(move |key| {
        let generation = IMAGE_GENERATION.with(Cell::get);
        let owner_path = owner_path.clone();
        let store = AssetStore::new(owner_path.with_file_name("assets"));
        drop(Dispatcher::current_dispatcher().spawn_local(async move {
            // Initial image requests can occur while constructing the view;
            // the host attaches its runtime before this dispatcher continuation.
            Dispatcher::yield_now().await;
            if IMAGE_GENERATION.with(Cell::get) != generation {
                return;
            }
            let Ok(lease) = Lease::current() else {
                return;
            };
            if lease.path() != owner_path {
                return;
            }
            let source = key.clone();
            let result = crate::background::run(move || store.get(&source)).await;
            if IMAGE_GENERATION.with(Cell::get) != generation || !lease.is_active() {
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
    let Ok(lease) = Lease::capture(&vm) else {
        return;
    };
    let kind = match request {
        HostRequest::ImportPhotos => Kind::Image,
        HostRequest::ImportMusic => Kind::Audio,
        HostRequest::ImportVideo => Kind::Video,
        _ => unreachable!("native media import"),
    };
    drop(Dispatcher::current_dispatcher().spawn_local(async move {
        if let Err(error) = import(&lease, &vm, provider, kind).await {
            lease.report(error);
        }
    }));
}

async fn import(
    lease: &Lease,
    vm: &PhoneViewModel,
    provider: Rc<dyn IStorageProvider>,
    kind: Kind,
) -> Result<(), String> {
    lease.check()?;
    let store = lease.store();
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
        FilePickerFileType::new(Some(title)).with_patterns(&patterns)
    ]));
    let files = provider
        .open_file_picker_async(options)
        .await
        .map_err(|error| error.to_string())?;
    lease.check()?;
    if files.len() > 512 {
        return Err("Choose at most 512 media files at once.".into());
    }
    let mut count = 0;
    for file in files {
        let name = file.name();
        let bytes = lease.read(file.as_ref(), kind.limit()).await?;
        let writer = store.clone();
        let (key, name, bytes) = crate::background::run(move || {
            let key = writer.put(&name, "", kind, &bytes)?;
            Ok((key, name, bytes))
        })
        .await?;
        let mut staged = StagedAssets::new(store.clone());
        staged.add(key.clone());
        lease.check()?;
        if kind == Kind::Image {
            // Use the same bounded codec admission as browser-loaded images.
            if let Err(error) = phone_ui::register_image(key.clone(), bytes) {
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
        staged.commit();
        count += 1;
        Dispatcher::yield_now().await;
    }
    if count > 0 {
        lease.check()?;
        vm.set_status(format!(
            "Imported {count} media file(s). Copies are stored with this app."
        ));
    }
    Ok(())
}

/// Only app-owned, validated files can be handed to an OS media application.
pub fn open_media(store: &AssetStore, source: &str) -> Result<(PathBuf, String), String> {
    let (path, metadata) = store.playable(source)?;
    Ok((path, metadata.name))
}

pub fn remove(store: &AssetStore, source: &str) -> Result<(), String> {
    if !source.starts_with("media:") {
        return Ok(());
    }
    store.remove(source)
}

/// Preserve real media bytes for a desktop attachment workflow. This does not
/// claim that another application received the file or that a share sheet ran.
pub fn share_copy(vm: Rc<PhoneViewModel>, provider: Rc<dyn IStorageProvider>, source: String) {
    let Ok(lease) = Lease::capture(&vm) else {
        return;
    };
    drop(Dispatcher::current_dispatcher().spawn_local(async move {
        if let Err(error) = save_copy(&lease, &vm, provider, source).await {
            lease.report(error);
        }
    }));
}

async fn save_copy(
    lease: &Lease,
    vm: &PhoneViewModel,
    provider: Rc<dyn IStorageProvider>,
    source: String,
) -> Result<(), String> {
    lease.check()?;
    let store = lease.store();
    let asset = crate::background::run(move || store.get(&source)).await?;
    lease.check()?;
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
    lease.write(file.as_ref(), &asset.bytes).await?;
    lease.check()?;
    vm.set_status(format!(
        "Saved {}. Attach this copy in your messaging or sharing app.",
        file.name()
    ));
    Ok(())
}

fn copy_name(metadata: &crate::asset_store::Metadata) -> String {
    phone_host::file_names::copy_name(&metadata.name, "WindowsPhone", &metadata.extension)
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
