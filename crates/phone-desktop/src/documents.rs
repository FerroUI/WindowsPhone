//! Native Office import/export through FerroUI's platform storage provider.
use ferroui_base::platform::storage::{
    FilePickerFileType, FilePickerOpenOptions, FilePickerSaveOptions, FolderPickerOpenOptions,
    IStorageProvider,
};
use ferroui_base::threading::Dispatcher;
use phone_domain::Id;
use phone_state::{HostRequest, PhoneViewModel};
use std::rc::Rc;

pub fn start(vm: Rc<PhoneViewModel>, provider: Rc<dyn IStorageProvider>, request: HostRequest) {
    drop(Dispatcher::current_dispatcher().spawn_local(async move {
        let result = match request {
            HostRequest::ImportDocuments { folder } => import(&vm, provider, folder).await,
            HostRequest::ExportDocument { id } => export(&vm, provider, id).await,
            HostRequest::ShareDocuments { ids } => share(&vm, provider, ids).await,
            _ => unreachable!("document host requests"),
        };
        if let Err(error) = result {
            vm.set_status(error);
        }
    }));
}

async fn share(
    vm: &PhoneViewModel,
    provider: Rc<dyn IStorageProvider>,
    ids: Vec<Id>,
) -> Result<(), String> {
    if ids.is_empty() || ids.len() > 128 {
        return Err("Choose between 1 and 128 documents to share.".into());
    }
    let mut unique = std::collections::BTreeSet::new();
    let documents = {
        let data = vm.data();
        ids.into_iter()
            .filter(|id| unique.insert(*id))
            .map(|id| {
                data.document(id)
                    .cloned()
                    .ok_or_else(|| "A selected document no longer exists.".to_owned())
            })
            .collect::<Result<Vec<_>, _>>()?
    };
    let files = crate::background::run(move || {
        let mut bytes = 0usize;
        documents
            .iter()
            .map(|document| {
                let file = phone_domain::export_office(document)?;
                bytes = bytes
                    .checked_add(file.bytes.len())
                    .ok_or("Document copies are too large.")?;
                if bytes > 64 * 1024 * 1024 {
                    return Err("Select at most 64 MiB of document copies at once.".into());
                }
                Ok(file)
            })
            .collect::<Result<Vec<_>, String>>()
    })
    .await?;
    if files.len() == 1 {
        let file = &files[0];
        let mut options = FilePickerSaveOptions::new();
        options.set_title(Some("Save a document copy to share".into()));
        options.set_suggested_file_name(Some(copy_name(&file.name, "WindowsPhone")));
        options.set_show_overwrite_prompt(Some(true));
        if let Some(target) = provider
            .save_file_picker_async(options)
            .await
            .map_err(|e| e.to_string())?
        {
            crate::picker_io::write(target.as_ref(), &file.bytes).await?;
            vm.set_status(format!(
                "Saved {}. Attach this copy in your sharing app.",
                target.name()
            ));
        }
        return Ok(());
    }
    let mut options = FolderPickerOpenOptions::new();
    options.set_title(Some(format!(
        "Choose a folder for {} document copies",
        files.len()
    )));
    options.set_allow_multiple(false);
    let folders = provider
        .open_folder_picker_async(options)
        .await
        .map_err(|e| e.to_string())?;
    let Some(folder) = folders.first() else {
        return Ok(());
    };
    let batch = crate::asset_store::unique_key().replace("media:", "WindowsPhone-");
    let mut count = 0;
    for (index, file) in files.into_iter().enumerate() {
        let name = copy_name(&file.name, &format!("{batch}-{}", index + 1));
        // The per-batch unique prefix preserves every existing destination file.
        if folder.get_file_async(&name).await.is_some() {
            return Err(format!(
                "A destination already exists; {count} copies were saved."
            ));
        }
        let target = folder
            .create_file_async(&name)
            .await
            .map_err(|e| format!("{e}; {count} copies were saved."))?
            .ok_or_else(|| {
                format!("The selected folder cannot create files; {count} copies were saved.")
            })?;
        crate::picker_io::write(target.as_ref(), &file.bytes)
            .await
            .map_err(|e| format!("{e}; {count} complete copies were saved."))?;
        count += 1;
    }
    vm.set_status(format!(
        "Saved {count} document copies. Attach them in your sharing app."
    ));
    Ok(())
}

async fn import(
    vm: &PhoneViewModel,
    provider: Rc<dyn IStorageProvider>,
    folder: Option<Id>,
) -> Result<(), String> {
    let file_type = FilePickerFileType::new(Some("Office documents"))
        .with_patterns(&["*.docx", "*.xlsx", "*.pptx", "*.txt", "*.csv"]);
    let mut options = FilePickerOpenOptions::new();
    options.set_title(Some("Import documents".into()));
    options.set_allow_multiple(true);
    options.set_file_type_filter(Some(vec![file_type]));
    let files = provider
        .open_file_picker_async(options)
        .await
        .map_err(|error| error.to_string())?;
    let mut count = 0;
    for file in files {
        let bytes = crate::picker_io::read(file.as_ref(), 8 * 1024 * 1024).await?;
        let mut document = phone_domain::import_office(&file.name(), &bytes)?;
        document.folder = folder;
        vm.import_document(document)?;
        count += 1;
    }
    if count > 0 {
        vm.set_status(format!("Imported {count} documents."));
    }
    Ok(())
}

async fn export(
    vm: &PhoneViewModel,
    provider: Rc<dyn IStorageProvider>,
    id: Id,
) -> Result<(), String> {
    let exported = {
        let data = vm.data();
        let document = data.document(id).ok_or("This document no longer exists.")?;
        phone_domain::export_office(document)?
    };
    let mut options = FilePickerSaveOptions::new();
    options.set_title(Some("Save a document copy".into()));
    options.set_suggested_file_name(Some(copy_name(&exported.name, "WindowsPhone")));
    options.set_show_overwrite_prompt(Some(true));
    let file = provider
        .save_file_picker_async(options)
        .await
        .map_err(|error| error.to_string())?;
    if let Some(file) = file {
        crate::picker_io::write(file.as_ref(), &exported.bytes).await?;
        vm.set_status(format!("Saved {}.", file.name()));
    }
    Ok(())
}

fn copy_name(name: &str, prefix: &str) -> String {
    let extension = name
        .rsplit_once('.')
        .map_or("bin", |(_, extension)| extension);
    phone_host::file_names::copy_name(name, prefix, extension)
}
