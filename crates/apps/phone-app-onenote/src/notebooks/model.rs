use phone_domain::Id;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

pub const QUICK_NOTES: u32 = 1;
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Section {
    pub id: u32,
    pub name: String,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Notebook {
    pub id: u32,
    pub name: String,
    pub sections: Vec<Section>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Library {
    version: u32,
    next_id: u32,
    pub notebooks: Vec<Notebook>,
    pub pages: BTreeMap<String, u32>,
}
impl Default for Library {
    fn default() -> Self {
        Self {
            version: 1,
            next_id: 2,
            notebooks: vec![Notebook {
                id: 1,
                name: "Personal".into(),
                sections: vec![Section {
                    id: QUICK_NOTES,
                    name: "Quick Notes".into(),
                }],
            }],
            pages: BTreeMap::new(),
        }
    }
}
impl Library {
    pub fn validate(&self) -> Result<(), String> {
        if self.version != 1 {
            return Err("This notebook data version is not supported.".into());
        }
        if self.notebooks.is_empty() || self.notebooks.len() > 32 || self.pages.len() > 10_000 {
            return Err("The notebook library exceeds its limits.".into());
        }
        let mut notebook_ids = BTreeSet::new();
        let mut section_ids = BTreeSet::new();
        let mut notebook_names = BTreeSet::new();
        let mut maximum = 1;
        for notebook in &self.notebooks {
            validate_name(&notebook.name)?;
            if notebook.id == 0
                || !notebook_ids.insert(notebook.id)
                || !notebook_names.insert(notebook.name.to_lowercase())
            {
                return Err("Notebook names and identifiers must be unique.".into());
            }
            maximum = maximum.max(notebook.id);
            let mut section_names = BTreeSet::new();
            for section in &notebook.sections {
                validate_name(&section.name)?;
                if section.id == 0
                    || !section_ids.insert(section.id)
                    || !section_names.insert(section.name.to_lowercase())
                {
                    return Err("Section names and identifiers must be unique.".into());
                }
                maximum = maximum.max(section.id);
            }
        }
        if section_ids.len() > 128
            || !section_ids.contains(&QUICK_NOTES)
            || self.next_id <= maximum
            || self.next_id == u32::MAX
        {
            return Err("The section library is invalid or full.".into());
        }
        for (page, section) in &self.pages {
            if page
                .parse::<Id>()
                .ok()
                .is_none_or(|id| id == 0 || id.to_string() != *page)
                || !section_ids.contains(section)
            {
                return Err("A page points to an invalid section.".into());
            }
        }
        Ok(())
    }
    pub fn section(&self, id: u32) -> Option<(&Notebook, &Section)> {
        self.notebooks.iter().find_map(|notebook| {
            notebook
                .sections
                .iter()
                .find(|section| section.id == id)
                .map(|section| (notebook, section))
        })
    }
    pub fn section_for(&self, page: Id) -> u32 {
        self.pages
            .get(&page.to_string())
            .copied()
            .unwrap_or(QUICK_NOTES)
    }
    pub fn prune(&mut self, ids: impl Iterator<Item = Id>) {
        let ids = ids.map(|id| id.to_string()).collect::<BTreeSet<_>>();
        self.pages.retain(|id, _| ids.contains(id));
    }
    pub fn move_page(&mut self, page: Id, section: u32) -> Result<(), String> {
        if page == 0 || self.section(section).is_none() {
            return Err("This page or section no longer exists.".into());
        }
        if section == QUICK_NOTES {
            self.pages.remove(&page.to_string());
        } else {
            self.pages.insert(page.to_string(), section);
        }
        self.validate()
    }
    pub fn add_notebook(&mut self, name: String) -> Result<u32, String> {
        validate_name(&name)?;
        if self.notebooks.len() >= 32
            || self
                .notebooks
                .iter()
                .any(|n| n.name.eq_ignore_ascii_case(&name))
        {
            return Err("Choose a unique notebook name; up to 32 notebooks are supported.".into());
        }
        let id = self.allocate()?;
        self.notebooks.push(Notebook {
            id,
            name,
            sections: vec![],
        });
        Ok(id)
    }
    pub fn add_section(&mut self, notebook: u32, name: String) -> Result<u32, String> {
        validate_name(&name)?;
        if self
            .notebooks
            .iter()
            .map(|n| n.sections.len())
            .sum::<usize>()
            >= 128
        {
            return Err("The section library is full.".into());
        }
        let target = self
            .notebooks
            .iter()
            .position(|n| n.id == notebook)
            .ok_or("This notebook no longer exists.")?;
        if self.notebooks[target]
            .sections
            .iter()
            .any(|s| s.name.eq_ignore_ascii_case(&name))
        {
            return Err("A section with this name already exists.".into());
        }
        let id = self.allocate()?;
        self.notebooks[target].sections.push(Section { id, name });
        Ok(id)
    }
    pub fn rename_notebook(&mut self, id: u32, name: String) -> Result<(), String> {
        validate_name(&name)?;
        if self
            .notebooks
            .iter()
            .any(|n| n.id != id && n.name.to_lowercase() == name.to_lowercase())
        {
            return Err("A notebook with this name already exists.".into());
        }
        self.notebooks
            .iter_mut()
            .find(|n| n.id == id)
            .ok_or("This notebook no longer exists.")?
            .name = name;
        Ok(())
    }
    pub fn rename_section(&mut self, id: u32, name: String) -> Result<(), String> {
        validate_name(&name)?;
        let notebook = self
            .notebooks
            .iter_mut()
            .find(|n| n.sections.iter().any(|s| s.id == id))
            .ok_or("This section no longer exists.")?;
        if notebook
            .sections
            .iter()
            .any(|s| s.id != id && s.name.to_lowercase() == name.to_lowercase())
        {
            return Err("A section with this name already exists.".into());
        }
        notebook
            .sections
            .iter_mut()
            .find(|s| s.id == id)
            .unwrap()
            .name = name;
        Ok(())
    }
    pub fn delete_section(&mut self, id: u32) -> Result<(), String> {
        if id == QUICK_NOTES {
            return Err("The Quick Notes section cannot be deleted.".into());
        }
        let notebook = self
            .notebooks
            .iter_mut()
            .find(|n| n.sections.iter().any(|s| s.id == id))
            .ok_or("This section no longer exists.")?;
        notebook.sections.retain(|s| s.id != id);
        self.pages.retain(|_, section| *section != id);
        Ok(())
    }
    pub fn delete_notebook(&mut self, id: u32) -> Result<(), String> {
        let notebook = self
            .notebooks
            .iter()
            .find(|n| n.id == id)
            .ok_or("This notebook no longer exists.")?;
        if notebook.sections.iter().any(|s| s.id == QUICK_NOTES) {
            return Err("The notebook containing Quick Notes cannot be deleted.".into());
        }
        let sections = notebook
            .sections
            .iter()
            .map(|s| s.id)
            .collect::<BTreeSet<_>>();
        self.pages.retain(|_, section| !sections.contains(section));
        self.notebooks.retain(|n| n.id != id);
        Ok(())
    }
    fn allocate(&mut self) -> Result<u32, String> {
        let id = self.next_id;
        self.next_id = self
            .next_id
            .checked_add(1)
            .filter(|next| *next < u32::MAX)
            .ok_or("The notebook identifier range is exhausted.")?;
        Ok(id)
    }
}
fn validate_name(value: &str) -> Result<(), String> {
    if value.trim().is_empty()
        || value.trim() != value
        || value.chars().count() > 80
        || value.chars().any(char::is_control)
    {
        Err("Use a name of 1–80 printable characters.".into())
    } else {
        Ok(())
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn deletion_preserves_pages_in_quick_notes_and_json_roundtrips() {
        let mut library = Library::default();
        let notebook = library.add_notebook("Work".into()).unwrap();
        let section = library.add_section(notebook, "Ideas".into()).unwrap();
        library.move_page(u64::MAX - 1, section).unwrap();
        let json = serde_json::to_value(&library).unwrap();
        phone_domain::validate_app_value(&json).unwrap();
        assert_eq!(serde_json::from_value::<Library>(json).unwrap(), library);
        library.delete_notebook(notebook).unwrap();
        assert_eq!(library.section_for(u64::MAX - 1), QUICK_NOTES);
        assert!(library.delete_section(QUICK_NOTES).is_err());
        assert!(library.delete_notebook(1).is_err());
        library.validate().unwrap();
    }
    #[test]
    fn names_locations_and_versions_are_validated() {
        let mut library = Library::default();
        assert!(library.add_notebook("personal".into()).is_err());
        assert!(library.add_section(1, "Quick Notes".into()).is_err());
        assert!(library.add_section(99, "Ideas".into()).is_err());
        assert!(library.move_page(8, 99).is_err());
        library.pages.insert("01".into(), 1);
        assert!(library.validate().is_err());
        library.pages.clear();
        library.version = 2;
        assert!(library.validate().is_err());
    }
    #[test]
    fn prune_only_drops_deleted_page_links() {
        let mut library = Library::default();
        let section = library.add_section(1, "Todo".into()).unwrap();
        library.move_page(20, section).unwrap();
        library.move_page(21, section).unwrap();
        library.prune([21].into_iter());
        assert_eq!(library.section_for(20), QUICK_NOTES);
        assert_eq!(library.section_for(21), section);
    }
}
