//! Atomic contact/calendar import and immutable share/export snapshots.
use phone_domain::{
    interchange::{self, InterchangeFile},
    *,
};
use std::collections::BTreeSet;

impl crate::PhoneViewModel {
    pub fn import_vcards(&self, bytes: &[u8]) -> Result<Vec<Id>, String> {
        self.ensure_pim_idle()?;
        let mut incoming = interchange::import_vcards(bytes)?;
        let mut candidate = self.data().clone();
        if candidate.contacts.len().saturating_add(incoming.len()) > 20_000 {
            return Err("The phone can hold up to 20,000 contacts.".into());
        }
        let ids = allocate(&mut candidate, incoming.len())?;
        for (contact, id) in incoming.iter_mut().zip(&ids) {
            contact.id = *id;
            contact.color = candidate.settings.accent.clone();
        }
        candidate.contacts.extend(incoming);
        self.commit_pim(candidate, false)?;
        Ok(ids)
    }

    pub fn export_vcards(&self, ids: &[Id]) -> Result<InterchangeFile, String> {
        let data = self.data();
        let selected = selection(ids)?;
        let contacts = selected
            .into_iter()
            .map(|id| {
                data.contact(id)
                    .cloned()
                    .ok_or_else(|| "A selected contact no longer exists.".to_owned())
            })
            .collect::<Result<Vec<_>, _>>()?;
        interchange::export_vcards(&contacts)
    }

    pub fn import_icalendar(&self, bytes: &[u8]) -> Result<Vec<Id>, String> {
        self.ensure_pim_idle()?;
        let mut incoming = interchange::import_icalendar(bytes)?;
        let mut candidate = self.data().clone();
        if candidate.events.len().saturating_add(incoming.len()) > MAX_CALENDAR_EVENTS {
            return Err("The phone calendar can hold up to 4,096 appointments and series.".into());
        }
        let ids = allocate(&mut candidate, incoming.len())?;
        for (event, id) in incoming.iter_mut().zip(&ids) {
            event.id = *id;
        }
        candidate.events.extend(incoming);
        self.commit_pim(candidate, true)?;
        Ok(ids)
    }

    pub fn export_icalendar(&self, ids: &[Id]) -> Result<InterchangeFile, String> {
        let data = self.data();
        let selected = selection(ids)?;
        let events = selected
            .into_iter()
            .map(|id| {
                data.events
                    .iter()
                    .find(|e| e.id == id)
                    .cloned()
                    .ok_or_else(|| "A selected appointment no longer exists.".to_owned())
            })
            .collect::<Result<Vec<_>, _>>()?;
        interchange::export_icalendar(&events, self.now())
    }

    fn ensure_pim_idle(&self) -> Result<(), String> {
        if self.applying.get() {
            Err("A contact or calendar import cannot run inside another command.".into())
        } else {
            Ok(())
        }
    }

    fn commit_pim(&self, mut candidate: PhoneData, schedule_changed: bool) -> Result<(), String> {
        candidate.normalize()?;
        if candidate.to_json()?.len() > MAX_BACKUP_BYTES {
            return Err("This import would exceed the phone's 16 MB document limit.".into());
        }
        self.applying.set(true);
        *self.document.borrow_mut() = candidate;
        if schedule_changed {
            self.schedule_revision
                .set(self.schedule_revision.get().wrapping_add(1));
        }
        self.synchronize();
        self.persist();
        self.notify();
        self.applying.set(false);
        // A host save observer may queue another command. Run it only after the
        // complete import snapshot is published, using the usual command pump.
        self.apply(None);
        Ok(())
    }
}

fn allocate(data: &mut PhoneData, count: usize) -> Result<Vec<Id>, String> {
    let end = data
        .next_id
        .checked_add(count as Id)
        .filter(|end| *end < Id::MAX)
        .ok_or("The phone identifier range is exhausted.")?;
    let ids = (data.next_id..end).collect();
    data.next_id = end;
    Ok(ids)
}

fn selection(ids: &[Id]) -> Result<BTreeSet<Id>, String> {
    if ids.is_empty() || ids.len() > interchange::MAX_INTERCHANGE_ITEMS {
        return Err("Choose between 1 and 512 items.".into());
    }
    let selected = ids.iter().copied().collect::<BTreeSet<_>>();
    if selected.contains(&0) {
        return Err("A selected item has no saved identifier.".into());
    }
    Ok(selected)
}
