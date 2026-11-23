//! The native implementation of the SDK's single-file result contract.
use ferroui_base::{
    platform::storage::{FilePickerFileType, FilePickerOpenOptions, IStorageProvider},
    threading::Dispatcher,
};
use phone_state::PhoneViewModel;
use std::rc::Rc;

pub fn start(
    vm: &Rc<PhoneViewModel>,
    provider: Rc<dyn IStorageProvider>,
    request_id: u32,
    extensions: Vec<String>,
    max_bytes: u32,
) {
    if let Err(error) = phone_host::file_picker::validate_options(&extensions, max_bytes) {
        phone_ui::complete_file(request_id, Err(error));
        return;
    }
    let epoch = vm.document_epoch();
    let weak = Rc::downgrade(vm);
    drop(Dispatcher::current_dispatcher().spawn_local(async move {
        let result = choose(provider, request_id, extensions, max_bytes as usize).await;
        if weak
            .upgrade()
            .is_some_and(|vm| vm.document_epoch() == epoch)
        {
            phone_ui::complete_file(request_id, result);
        } else {
            phone_ui::complete_file(
                request_id,
                Err("The phone session changed while choosing a file.".into()),
            );
        }
    }));
}

async fn choose(
    provider: Rc<dyn IStorageProvider>,
    request_id: u32,
    extensions: Vec<String>,
    max_bytes: usize,
) -> Result<Option<phone_ui::PickedFile>, String> {
    if !phone_ui::is_file_task_pending(request_id) {
        return Err("File selection was cancelled.".into());
    }
    if !provider.can_open() {
        return Err("This device does not provide a file picker.".into());
    }
    let mut options = FilePickerOpenOptions::new();
    options.set_title(Some("Choose a file".into()));
    options.set_allow_multiple(false);
    if !extensions.is_empty() {
        let patterns: Vec<String> = extensions
            .iter()
            .map(|extension| format!("*{extension}"))
            .collect();
        let patterns: Vec<&str> = patterns.iter().map(String::as_str).collect();
        options.set_file_type_filter(Some(vec![
            FilePickerFileType::new(Some("Supported files")).with_patterns(&patterns)
        ]));
    }
    let files = provider
        .open_file_picker_async(options)
        .await
        .map_err(|error| error.to_string())?;
    if files.len() > 1 {
        return Err("The device returned more than one selected file.".into());
    }
    let Some(file) = files.first() else {
        return Ok(None);
    };
    let name = file.name();
    if !phone_host::file_picker::accepts_name(&extensions, &name) {
        return Err("The selected file does not match the requested file type.".into());
    }
    let bytes = crate::picker_io::read_while(file.as_ref(), max_bytes, || {
        phone_ui::is_file_task_pending(request_id)
    })
    .await?;
    Ok(Some(phone_ui::PickedFile {
        mime: mime_for_name(&name).into(),
        name,
        bytes,
    }))
}

// IStorageFile has no MIME property. This is an advisory suffix hint, never
// used to admit or decode untrusted bytes; unknown types remain opaque.
fn mime_for_name(name: &str) -> &'static str {
    match name
        .rsplit_once('.')
        .map(|(_, suffix)| suffix.to_ascii_lowercase())
        .as_deref()
    {
        Some("txt") => "text/plain",
        Some("csv") => "text/csv",
        Some("json") => "application/json",
        Some("jpg" | "jpeg") => "image/jpeg",
        Some("png") => "image/png",
        Some("gif") => "image/gif",
        Some("webp") => "image/webp",
        Some("pdf") => "application/pdf",
        Some("mp3") => "audio/mpeg",
        Some("wav") => "audio/wav",
        Some("mp4") => "video/mp4",
        _ => "application/octet-stream",
    }
}
