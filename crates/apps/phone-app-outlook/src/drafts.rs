//! Local composition is app-owned data. A host email request never commits a
//! draft, removes one, or fabricates an entry in the sent mailbox.
use phone_sdk::AppStorage;
use phone_state::{HostRequest, PhoneViewModel};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeSet, rc::Rc};

pub const APP_ID: &str = "phone.outlook";
const MAX_DRAFTS: usize = 100;
const MAX_HEADER_BYTES: usize = 4096;
const MAX_BODY_BYTES: usize = 64 * 1024;

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Fields {
    pub to: String,
    pub subject: String,
    pub body: String,
}

impl Fields {
    pub fn is_empty(&self) -> bool {
        self.to.trim().is_empty() && self.subject.trim().is_empty() && self.body.trim().is_empty()
    }

    fn validate(&self) -> Result<(), String> {
        if self.to.len() > MAX_HEADER_BYTES || self.subject.len() > MAX_HEADER_BYTES {
            return Err("The recipient or subject is too long (maximum 4 KiB each).".into());
        }
        if self.body.len() > MAX_BODY_BYTES {
            return Err("The message is too long (maximum 64 KiB).".into());
        }
        if self.to.chars().any(char::is_control) || self.subject.chars().any(char::is_control) {
            return Err("The recipient and subject must each be on one line.".into());
        }
        if self.body.contains('\0') {
            return Err("The message contains an unsupported null character.".into());
        }
        Ok(())
    }

    pub fn email_request(&self) -> Result<HostRequest, String> {
        self.validate()?;
        let to = self.to.trim();
        if to.is_empty()
            || to.split([',', ';']).any(|address| {
                let address = address.trim();
                address.chars().any(char::is_whitespace)
                    || address.split_once('@').is_none_or(|(local, domain)| {
                        local.is_empty() || domain.is_empty() || domain.contains('@')
                    })
            })
        {
            return Err("Enter an email address. Separate multiple addresses with commas.".into());
        }
        Ok(HostRequest::Email {
            to: to.to_owned(),
            subject: self.subject.clone(),
            body: self.body.clone(),
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Draft {
    pub id: u32,
    pub fields: Fields,
    pub updated: i64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Library {
    version: u32,
    next_id: u32,
    pub items: Vec<Draft>,
}

impl Default for Library {
    fn default() -> Self {
        Self {
            version: 1,
            next_id: 1,
            items: Vec::new(),
        }
    }
}

impl Library {
    fn validate(&self) -> Result<(), String> {
        if self.version != 1 {
            return Err("This draft data version is not supported.".into());
        }
        if self.next_id == 0 || self.items.len() > MAX_DRAFTS {
            return Err("The draft library is invalid or exceeds its limit.".into());
        }
        let mut ids = BTreeSet::new();
        for draft in &self.items {
            if draft.id == 0
                || draft.id >= self.next_id
                || !ids.insert(draft.id)
                || draft.updated.unsigned_abs() > 9_007_199_254_740_991
            {
                return Err("The draft library contains invalid identifiers or timestamps.".into());
            }
            draft.fields.validate()?;
        }
        Ok(())
    }
}

pub fn read(vm: &Rc<PhoneViewModel>) -> Result<Library, String> {
    let library = AppStorage::new(vm.clone(), APP_ID).read::<Library>()?;
    library.validate()?;
    Ok(library)
}

/// Read, validate and persist one complete namespace transaction. The caller
/// keeps its editor text on every failure, including the SDK's total size limit.
pub fn save(vm: &Rc<PhoneViewModel>, id: Option<u32>, fields: Fields) -> Result<Draft, String> {
    fields.validate()?;
    if fields.is_empty() {
        return Err("Add a recipient, subject or message before saving a draft.".into());
    }
    let mut library = read(vm)?;
    let index = if let Some(id) = id {
        let index = library
            .items
            .iter()
            .position(|draft| draft.id == id)
            .ok_or("This draft no longer exists. Your unsaved text is still in the editor.")?;
        if library.items[index].fields == fields {
            return Ok(library.items[index].clone());
        }
        index
    } else {
        if library.items.len() >= MAX_DRAFTS {
            return Err("The draft folder is full. Delete a draft before saving another.".into());
        }
        let id = library.next_id;
        library.next_id = id
            .checked_add(1)
            .ok_or("The draft identifiers are exhausted.")?;
        library.items.push(Draft {
            id,
            fields: Fields::default(),
            updated: vm.now(),
        });
        library.items.len() - 1
    };
    library.items[index].fields = fields;
    library.items[index].updated = vm.now();
    library.validate()?;
    let draft = library.items[index].clone();
    AppStorage::new(vm.clone(), APP_ID).write(&library)?;
    Ok(draft)
}

pub fn delete(vm: &Rc<PhoneViewModel>, id: u32) -> Result<(), String> {
    let mut library = read(vm)?;
    let before = library.items.len();
    library.items.retain(|draft| draft.id != id);
    if library.items.len() == before {
        return Ok(());
    }
    AppStorage::new(vm.clone(), APP_ID).write(&library)
}

#[cfg(test)]
mod tests;
