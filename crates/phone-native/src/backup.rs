//! Native backup pickers preserve media and the previous phone document.
mod codec;
mod encoding;

use crate::operations::{Lease, StagedAssets};
use ferroui_base::{
    platform::storage::{
        FilePickerFileType, FilePickerOpenOptions, FilePickerSaveOptions, IStorageProvider,
    },
    threading::Dispatcher,
};
use phone_state::{HostRequest, PhoneViewModel};
use std::{collections::BTreeMap, rc::Rc};

pub fn start(vm: Rc<PhoneViewModel>, provider: Rc<dyn IStorageProvider>, request: HostRequest) {
    let Ok(lease) = Lease::capture(&vm) else {
        return;
    };
    drop(Dispatcher::current_dispatcher().spawn_local(async move {
        let result = match request {
            HostRequest::ExportBackup => export(&lease, &vm, provider).await,
            HostRequest::ImportBackup => import(&lease, &vm, provider).await,
            _ => unreachable!("native backup request"),
        };
        if let Err(error) = result {
            lease.report(error);
        }
    }));
}

async fn export(
    lease: &Lease,
    vm: &PhoneViewModel,
    provider: Rc<dyn IStorageProvider>,
) -> Result<(), String> {
    lease.check()?;
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
    lease.check()?;
    let phone = vm.data().clone();
    let store = lease.store();
    let (bytes, missing) = crate::background::run(move || codec::export(&phone, &store)).await?;
    lease.write(file.as_ref(), &bytes).await?;
    lease.check()?;
    vm.set_status(if missing == 0 { format!("Saved {} with your available media files.", file.name()) } else { format!("Saved {}. {missing} referenced media file(s) were unavailable and could not be included.", file.name()) });
    Ok(())
}

async fn import(
    lease: &Lease,
    vm: &PhoneViewModel,
    provider: Rc<dyn IStorageProvider>,
) -> Result<(), String> {
    lease.check()?;
    let mut options = FilePickerOpenOptions::new();
    options.set_title(Some("Restore phone backup".into()));
    options.set_allow_multiple(false);
    options.set_file_type_filter(Some(vec![
        FilePickerFileType::new(Some("Phone backup")).with_patterns(&["*.json"])
    ]));
    let files = provider
        .open_file_picker_async(options)
        .await
        .map_err(|error| error.to_string())?;
    lease.check()?;
    let Some(file) = files.first() else {
        return Ok(());
    };
    if files.len() != 1 {
        return Err("Choose one phone backup to restore.".into());
    }
    let bytes = lease.read(file.as_ref(), codec::ENVELOPE_LIMIT).await?;
    let mut restored = crate::background::run(move || codec::decode(&bytes)).await?;
    lease.check()?;
    let recovery = lease.path().with_file_name(format!(
        "before-restore-{}.json",
        crate::asset_store::unique_key().trim_start_matches("media:")
    ));
    let store = lease.store();
    let mut staged = StagedAssets::new(store.clone());
    let mut keys = BTreeMap::new();
    for asset in restored.assets {
        lease.check()?;
        let original = asset.key.clone();
        let writer = store.clone();
        let key = crate::background::run(move || {
            writer.put(&asset.name, &asset.mime, asset.kind, &asset.bytes)
        })
        .await?;
        staged.add(key.clone());
        keys.insert(original, key);
        lease.check()?;
        Dispatcher::yield_now().await;
    }
    lease.check()?;
    codec::remap(&mut restored.phone, &keys);
    // Preserve the latest phone immediately before replacement, after all
    // cooperative file I/O. Changes made while a picker/import was pending
    // are included. Existing immutable assets remain valid for recovery.
    crate::storage::save(&recovery, &vm.data())?;
    lease.check()?;
    vm.replace_data_after_save(restored.phone, |phone| {
        crate::storage::save(lease.path(), phone)
    })?;
    // The successful replacement deliberately changes this lease's epoch.
    // No asynchronous boundary may intervene before committing its new assets.
    staged.commit();
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
