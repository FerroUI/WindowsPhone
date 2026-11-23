//! Composite note/app-metadata transactions for independently registered editors.
use crate::PhoneViewModel;
use phone_domain::{Action, Id, Note, MAX_APP_NAMESPACES, MAX_BACKUP_BYTES};

impl PhoneViewModel {
    /// Save a note and its app-owned index as one validated phone-document
    /// mutation. For a new note the caller may index `data().next_id` before
    /// this synchronous call; validation failures never consume that ID.
    pub fn save_note_with_app_data(
        &self,
        note: Note,
        namespace: &str,
        value: serde_json::Value,
    ) -> Result<Id, String> {
        if self.applying.get() {
            return Err("A note transaction is already active.".into());
        }
        phone_domain::validate_app_key(namespace)?;
        phone_domain::validate_app_value(&value)?;
        if note.title.trim().is_empty() || note.title.chars().count() > 256 {
            return Err("Give your note a title of 1–256 characters.".into());
        }
        let mut candidate = self.document.borrow().clone();
        let id = if note.id == 0 {
            if candidate.notes.len() >= 10_000 || candidate.next_id >= Id::MAX - 1 {
                return Err("The note library is full.".into());
            }
            candidate.next_id
        } else {
            if candidate.note(note.id).is_none() {
                return Err("This note no longer exists.".into());
            }
            note.id
        };
        crate::mutation::mutate(&mut candidate, Action::SaveNote(note), self.now())?;
        candidate.app_data.insert(namespace.into(), value);
        if candidate.app_data.len() > MAX_APP_NAMESPACES {
            return Err("Too many app data namespaces.".into());
        }
        candidate.normalize()?;
        if candidate.to_json()?.len() > MAX_BACKUP_BYTES {
            return Err("The phone document is full.".into());
        }
        if *self.document.borrow() == candidate {
            return Ok(id);
        }
        self.applying.set(true);
        *self.document.borrow_mut() = candidate;
        self.persist();
        self.notify();
        self.applying.set(false);
        self.apply(None);
        Ok(id)
    }
}
