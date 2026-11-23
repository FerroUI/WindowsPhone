use super::{FilePickerOptions, FileResult};

pub(super) fn result(value: FileResult, options: &FilePickerOptions) -> FileResult {
    let file = match value {
        Ok(Some(file)) => file,
        Ok(None) => return Ok(None),
        Err(error) => return Err(error.chars().take(1024).collect()),
    };
    if file.name.trim().is_empty()
        || file.name.len() > 255
        || matches!(file.name.as_str(), "." | "..")
        || file
            .name
            .chars()
            .any(|ch| ch.is_control() || matches!(ch, '/' | '\\'))
    {
        return Err("The selected file has an invalid portable name.".into());
    }
    if file.mime.len() > 128 || !file.mime.is_ascii() || file.mime.chars().any(char::is_control) {
        return Err("The selected file has an invalid media type.".into());
    }
    if !phone_host::file_picker::accepts_name(&options.extensions, &file.name) {
        return Err("The selected file does not match the requested file types.".into());
    }
    if file.bytes.len() > options.max_bytes as usize {
        return Err("The selected file exceeds the requested size limit.".into());
    }
    Ok(Some(file))
}
