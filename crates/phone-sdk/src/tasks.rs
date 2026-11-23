//! Typed, cancellable host choosers. Request dispatch happens when the task is
//! created, before polling, so browser picker calls retain native user activation.
mod file;
mod registry;
#[cfg(test)]
mod tests;
mod validation;

pub use file::{choose_file, FileTask};
pub use registry::{cancel_for_app, cancel_for_session, complete_file, is_file_task_pending};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FilePickerOptions {
    /// Canonical lowercase extensions including a dot; empty accepts any name.
    pub extensions: Vec<String>,
    /// Single-file byte limit, between one byte and 16 MiB.
    pub max_bytes: u32,
}
impl Default for FilePickerOptions {
    fn default() -> Self {
        Self {
            extensions: Vec::new(),
            max_bytes: phone_host::file_picker::MAX_BYTES,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PickedFile {
    pub name: String,
    pub mime: String,
    pub bytes: Vec<u8>,
}

pub type FileResult = Result<Option<PickedFile>, String>;
pub const MAX_PENDING_FILE_TASKS: usize = 16;
