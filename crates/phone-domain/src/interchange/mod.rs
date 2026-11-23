//! Bounded, platform-independent contact and appointment file interchange.
//! Parsing completes before callers publish a document mutation. Unsupported
//! information is rejected rather than silently removed from an imported copy.
mod icalendar;
mod text;
mod vcard;

pub use icalendar::{export_icalendar, import_icalendar};
pub use vcard::{export_vcards, import_vcards, validate_contact_card};

pub const MAX_INTERCHANGE_BYTES: usize = 2 * 1024 * 1024;
pub const MAX_INTERCHANGE_ITEMS: usize = 512;

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct InterchangeFile {
    pub name: String,
    pub mime: String,
    pub bytes: Vec<u8>,
}

fn file(name: &str, mime: &str, text: String) -> Result<InterchangeFile, String> {
    if text.len() > MAX_INTERCHANGE_BYTES {
        return Err("Choose fewer items; the exported file exceeds 2 MB.".into());
    }
    Ok(InterchangeFile {
        name: name.into(),
        mime: mime.into(),
        bytes: text.into_bytes(),
    })
}
