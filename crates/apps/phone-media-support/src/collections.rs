//! Bounded, named, ordered references shared by playlists and photo albums.
use phone_domain::Id;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

const MAX_COLLECTIONS: usize = 128;
const MAX_ITEMS: usize = 4096;
const MAX_TOTAL: usize = 12_288;
const MAX_SAFE_ID: u64 = 9_007_199_254_740_991;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Collection {
    pub id: Id,
    pub name: String,
    pub items: Vec<Id>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Collections {
    pub next_id: Id,
    pub groups: Vec<Collection>,
}
impl Default for Collections {
    fn default() -> Self {
        Self {
            next_id: 1,
            groups: Vec::new(),
        }
    }
}
pub fn valid_name(name: &str) -> Result<String, String> {
    let name = name.trim();
    if name.is_empty() || name.chars().count() > 80 || name.chars().any(char::is_control) {
        Err("Enter a name of 1–80 printable characters.".into())
    } else {
        Ok(name.into())
    }
}
impl Collections {
    pub fn validate(&self) -> Result<(), String> {
        if self.groups.len() > MAX_COLLECTIONS || self.next_id == 0 || self.next_id > MAX_SAFE_ID {
            return Err("The collection has invalid limits or identifiers.".into());
        }
        let mut ids = BTreeSet::new();
        let mut names = BTreeSet::new();
        let mut total = 0;
        for group in &self.groups {
            valid_name(&group.name)?;
            if group.id == 0
                || group.id >= self.next_id
                || !ids.insert(group.id)
                || !names.insert(group.name.to_lowercase())
            {
                return Err("Collection names and identifiers must be unique.".into());
            }
            let mut items = BTreeSet::new();
            if group.items.len() > MAX_ITEMS
                || group
                    .items
                    .iter()
                    .any(|id| *id == 0 || *id > MAX_SAFE_ID || !items.insert(*id))
            {
                return Err("A collection contains too many, duplicate or invalid items.".into());
            }
            total += group.items.len();
        }
        if total > MAX_TOTAL {
            return Err("Collections contain too many item references.".into());
        }
        Ok(())
    }
    pub fn get(&self, id: Id) -> Option<&Collection> {
        self.groups.iter().find(|group| group.id == id)
    }
    pub fn create(&mut self, name: &str) -> Result<Id, String> {
        let name = self.unique_name(name, None)?;
        if self.groups.len() >= MAX_COLLECTIONS || self.next_id >= MAX_SAFE_ID {
            return Err("The collection limit has been reached.".into());
        }
        let id = self.next_id;
        self.next_id += 1;
        self.groups.push(Collection {
            id,
            name,
            items: Vec::new(),
        });
        Ok(id)
    }
    pub fn rename(&mut self, id: Id, name: &str) -> Result<(), String> {
        let name = self.unique_name(name, Some(id))?;
        self.groups
            .iter_mut()
            .find(|group| group.id == id)
            .ok_or("This collection no longer exists.")?
            .name = name;
        Ok(())
    }
    fn unique_name(&self, name: &str, except: Option<Id>) -> Result<String, String> {
        let name = valid_name(name)?;
        if self.groups.iter().any(|group| {
            Some(group.id) != except && group.name.to_lowercase() == name.to_lowercase()
        }) {
            return Err("A collection with that name already exists.".into());
        }
        Ok(name)
    }
    pub fn remove(&mut self, id: Id) {
        self.groups.retain(|group| group.id != id);
    }
    pub fn replace_items(
        &mut self,
        id: Id,
        selected: &BTreeSet<Id>,
        available: &[Id],
    ) -> Result<(), String> {
        let available_set = available.iter().copied().collect::<BTreeSet<_>>();
        if selected.len() > MAX_ITEMS || !selected.is_subset(&available_set) {
            return Err("The selection contains unavailable items or exceeds 4,096 items.".into());
        }
        let mut candidate = self.clone();
        let group = candidate
            .groups
            .iter_mut()
            .find(|group| group.id == id)
            .ok_or("This collection no longer exists.")?;
        // Preserve manually arranged items; append newly selected items in their
        // library order, independent of checkbox order and numeric identifiers.
        group.items.retain(|item| selected.contains(item));
        let mut included = group.items.iter().copied().collect::<BTreeSet<_>>();
        for item in available {
            if selected.contains(item) && included.insert(*item) {
                group.items.push(*item);
            }
        }
        candidate.validate()?;
        *self = candidate;
        Ok(())
    }
    pub fn move_item(&mut self, id: Id, item: Id, delta: isize) -> Result<(), String> {
        let group = self
            .groups
            .iter_mut()
            .find(|group| group.id == id)
            .ok_or("This collection no longer exists.")?;
        let index = group
            .items
            .iter()
            .position(|value| *value == item)
            .ok_or("This item is no longer in the collection.")?;
        let target = index
            .saturating_add_signed(delta)
            .min(group.items.len() - 1);
        group.items.swap(index, target);
        Ok(())
    }
    pub fn remove_item(&mut self, id: Id, item: Id) {
        if let Some(group) = self.groups.iter_mut().find(|group| group.id == id) {
            group.items.retain(|value| *value != item);
        }
    }
    pub fn retain_items(&mut self, mut available: impl FnMut(Id) -> bool) {
        for group in &mut self.groups {
            group.items.retain(|id| available(*id));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn names_and_limits_are_validated_without_mutation() {
        let mut groups = Collections::default();
        groups.create(" Road trip ").unwrap();
        let original = groups.clone();
        assert!(groups.create("ROAD TRIP").is_err());
        assert!(groups.rename(1, "\n").is_err());
        assert_eq!(groups, original);
        let mut bad = groups;
        bad.next_id = 1;
        assert!(bad.validate().is_err());
    }
    #[test]
    fn selection_preserves_order_appends_new_items_and_rejects_stale_ids_atomically() {
        let mut groups = Collections::default();
        let id = groups.create("Favorites").unwrap();
        groups
            .replace_items(id, &BTreeSet::from([1, 2]), &[2, 1, 3])
            .unwrap();
        groups.move_item(id, 1, -1).unwrap();
        groups
            .replace_items(id, &BTreeSet::from([1, 2, 3]), &[3, 2, 1])
            .unwrap();
        assert_eq!(groups.get(id).unwrap().items, vec![1, 2, 3]);
        let original = groups.clone();
        assert!(groups
            .replace_items(id, &BTreeSet::from([8]), &[1, 2, 3])
            .is_err());
        assert_eq!(groups, original);
        groups.retain_items(|id| id != 2);
        assert_eq!(groups.get(id).unwrap().items, vec![1, 3]);
        groups.remove(id);
        assert!(groups.groups.is_empty());
    }
    #[test]
    fn deleting_album_never_deletes_another_collections_references() {
        let mut groups = Collections::default();
        let a = groups.create("One").unwrap();
        let b = groups.create("Two").unwrap();
        for id in [a, b] {
            groups
                .replace_items(id, &BTreeSet::from([1]), &[1])
                .unwrap();
        }
        groups.remove(a);
        assert_eq!(groups.get(b).unwrap().items, vec![1]);
    }
}
