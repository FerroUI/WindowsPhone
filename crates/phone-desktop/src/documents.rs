//! Native Office import/export through FerroUI's platform storage provider.
use ferroui_base::platform::storage::{
    FilePickerFileType, FilePickerOpenOptions, FilePickerSaveOptions, IStorageProvider,
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
            _ => unreachable!("document host requests"),
        };
        if let Err(error) = result {
            vm.set_status(error);
        }
    }));
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
    options.set_suggested_file_name(Some(exported.name));
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
