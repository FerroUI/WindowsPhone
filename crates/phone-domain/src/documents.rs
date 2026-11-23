//! Bounded local Office document models and portable Open XML interchange.
mod archive;
mod csv;
mod export;
mod formula;
mod history;
mod import;
mod library;
mod models;
mod range;
#[cfg(test)]
mod range_tests;
#[cfg(test)]
mod tests;

pub use csv::*;
pub use export::{export_office, ExportedDocument};
pub use formula::{cell_address, cell_coordinates, display_cell};
pub use history::{EditDirection, EditHistory};
pub use import::import_office;
pub use library::{validate_document, validate_library, validate_name};
pub use models::*;
pub use range::*;

pub const MAX_DOCUMENT_TEXT: usize = 2 * 1024 * 1024;
pub const MAX_LIBRARY_TEXT: usize = 8 * 1024 * 1024;
pub const MAX_DOCUMENTS: usize = 512;
pub const MAX_FOLDERS: usize = 256;
pub const MAX_SHEET_CELLS: usize = 10_000;
pub const MAX_ROWS: usize = 10_000;
pub const MAX_COLUMNS: usize = 256;

#[derive(Clone, Debug)]
pub enum DocumentAction {
    Save(OfficeDocument),
    Delete(Vec<crate::Id>),
    Rename {
        id: crate::Id,
        name: String,
    },
    Copy {
        id: crate::Id,
        folder: Option<crate::Id>,
    },
    CopyMany {
        ids: Vec<crate::Id>,
        folder: Option<crate::Id>,
    },
    Move {
        ids: Vec<crate::Id>,
        folder: Option<crate::Id>,
    },
    CreateFolder {
        name: String,
        parent: Option<crate::Id>,
    },
    RenameFolder {
        id: crate::Id,
        name: String,
    },
    DeleteFolder {
        id: crate::Id,
    },
}
