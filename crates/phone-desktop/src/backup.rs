//! Native backup pickers preserve media and the previous phone document.
mod codec;
mod encoding;

use crate::asset_store::AssetStore;
use ferroui_base::{
    platform::storage::{
        FilePickerFileType, FilePickerOpenOptions, FilePickerSaveOptions, IStorageProvider,
    },
    threading::Dispatcher,
};
use phone_state::{HostRequest, PhoneViewModel};
use std::{collections::BTreeMap, rc::Rc};

pub fn start(vm: Rc<PhoneViewModel>, provider: Rc<dyn IStorageProvider>, request: HostRequest) {
    drop(Dispatcher::current_dispatcher().spawn_local(async move {
        let result = match request {
            HostRequest::ExportBackup => export(&vm, provider).await,
            HostRequest::ImportBackup => import(&vm, provider).await,
            _ => unreachable!("native backup request"),
        };
        if let Err(error) = result {
            vm.set_status(error);
        }
    }));
}

async fn export(vm: &PhoneViewModel, provider: Rc<dyn IStorageProvider>) -> Result<(), String> {
    let mut options = FilePickerSaveOptions::new();
    options.set_title(Some("Save phone backup".into()));
    options.set_suggested_file_name(Some(format!(
        "windows-phone-{}.json",
        chrono::Local::now().format("%Y-%m-%d")
    )));
    options.set_show_overwrite_prompt(Some(true));
    let Some(file) = provider
        .save_file_picker_async(options)
        .await
        .map_err(|error| error.to_string())?
    else {
        return Ok(());
    };
    let phone = vm.data().clone();
    let (bytes, missing) =
        crate::background::run(move || codec::export(&phone, &AssetStore::default())).await?;
    crate::picker_io::write(file.as_ref(), &bytes).await?;
    vm.set_status(if missing == 0 { format!("Saved {} with your available media files.", file.name()) } else { format!("Saved {}. {missing} referenced media file(s) were unavailable and could not be included.", file.name()) });
    Ok(())
}

async fn import(vm: &PhoneViewModel, provider: Rc<dyn IStorageProvider>) -> Result<(), String> {
    let mut options = FilePickerOpenOptions::new();
    options.set_title(Some("Restore phone backup".into()));
    options.set_allow_multiple(false);
    options.set_file_type_filter(Some(vec![
        FilePickerFileType::new(Some("Phone backup")).with_patterns(&["*.json"]),
    ]));
    let files = provider
        .open_file_picker_async(options)
        .await
        .map_err(|error| error.to_string())?;
    let Some(file) = files.first() else {
        return Ok(());
    };
    let bytes = crate::picker_io::read(file.as_ref(), codec::ENVELOPE_LIMIT).await?;
    let mut restored = crate::background::run(move || codec::decode(&bytes)).await?;
    let recovery = crate::storage::active_data_path().with_file_name(format!(
        "before-restore-{}.json",
        crate::asset_store::unique_key().trim_start_matches("media:")
    ));
    let store = AssetStore::default();
    let mut keys = BTreeMap::new();
    for asset in restored.assets {
        let original = asset.key.clone();
        let writer = store.clone();
        match crate::background::run(move || {
            writer.put(&asset.name, &asset.mime, asset.kind, &asset.bytes)
        })
        .await
        {
            Ok(key) => {
                keys.insert(original, key);
            }
            Err(error) => {
                cleanup(&store, &keys);
                return Err(error);
            }
        }
        Dispatcher::yield_now().await;
    }
    codec::remap(&mut restored.phone, &keys);
    // Preserve the latest phone immediately before replacement, after all
    // cooperative file I/O. Changes made while a picker/import was pending
    // are included. Existing immutable assets remain valid for recovery.
    if let Err(error) = crate::storage::save(&recovery, &vm.data()) {
        cleanup(&store, &keys);
        return Err(error);
    }
    if let Err(error) = vm.replace_data_after_save(restored.phone, |phone| {
        crate::storage::save(&crate::storage::active_data_path(), phone)
    }) {
        cleanup(&store, &keys);
        return Err(error);
    }
    crate::media_files::clear_images();
    phone_ui::show_media(String::new(), String::new(), false);
    vm.set_status(format!(
        "Restored {}. Previous phone data is preserved in {}.{}",
        file.name(),
        recovery.display(),
        if restored.missing == 0 {
            String::new()
        } else {
            format!(
                " {} media file(s) were not included in this backup.",
                restored.missing
            )
        }
    ));
    Ok(())
}

fn cleanup(store: &AssetStore, keys: &BTreeMap<String, String>) {
    for key in keys.values() {
        let _ = store.remove(key);
    }
}
