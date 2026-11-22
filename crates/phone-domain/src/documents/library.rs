use super::*;
use std::collections::HashSet;

pub fn validate_name(name: &str) -> Result<(), String> {
    if name.trim().is_empty()
        || name.chars().count() > 120
        || name.trim() != name
        || name
            .chars()
            .any(|c| c.is_control() || "\\/:*?\"<>|".contains(c))
        || matches!(name, "." | "..")
        || name.ends_with('.')
    {
        return Err("Use a name of 1–120 characters without / \\ : * ? \" < > |.".into());
    }
    Ok(())
}

pub fn validate_document(document: &OfficeDocument) -> Result<(), String> {
    validate_name(&document.name)?;
    if document.text_size() > MAX_DOCUMENT_TEXT {
        return Err("This document exceeds the 2 MB text limit.".into());
    }
    if document.import_notice.len() > 2048 {
        return Err("The document import notice is too long.".into());
    }
    match &document.content {
        DocumentContent::Word(doc) => {
            if doc.paragraphs.is_empty() || doc.paragraphs.len() > 2000 {
                return Err("Use between 1 and 2,000 paragraphs.".into());
            }
            if doc.paragraphs.iter().any(|p| p.heading > 3) {
                return Err("This heading level is not supported.".into());
            }
        }
        DocumentContent::Excel(book) => {
            if book.sheets.is_empty() || book.sheets.len() > 16 {
                return Err("Use between 1 and 16 worksheets.".into());
            }
            let mut names = HashSet::new();
            for sheet in &book.sheets {
                if sheet.name.trim().is_empty()
                    || sheet.name.chars().count() > 31
                    || sheet
                        .name
                        .chars()
                        .any(|c| c.is_control() || "[]:*?/\\".contains(c))
                    || !names.insert(sheet.name.to_lowercase())
                {
                    return Err(
                        "Worksheet names must be unique and contain at most 31 valid characters."
                            .into(),
                    );
                }
                if sheet.cells.len() > MAX_SHEET_CELLS {
                    return Err("This worksheet exceeds 10,000 populated cells.".into());
                }
                for (address, value) in &sheet.cells {
                    if cell_coordinates(address).is_none_or(|(c, r)| cell_address(c, r) != *address)
                    {
                        return Err(format!(
                            "Cell {address} must use an uppercase address in A1:IV10000."
                        ));
                    }
                    if value.len() > 32767 {
                        return Err("A cell exceeds the 32,767-character limit.".into());
                    }
                }
            }
        }
        DocumentContent::PowerPoint(deck) => {
            if deck.slides.is_empty() || deck.slides.len() > 200 {
                return Err("Use between 1 and 200 slides.".into());
            }
        }
    }
    Ok(())
}

pub fn validate_library(
    documents: &[OfficeDocument],
    folders: &[DocumentFolder],
) -> Result<(), String> {
    if documents.len() > MAX_DOCUMENTS || folders.len() > MAX_FOLDERS {
        return Err("The document library is full.".into());
    }
    if documents
        .iter()
        .map(OfficeDocument::text_size)
        .sum::<usize>()
        > MAX_LIBRARY_TEXT
    {
        return Err("The document library exceeds the 8 MB text limit.".into());
    }
    let ids = folders.iter().map(|f| f.id).collect::<HashSet<_>>();
    if ids.len() != folders.len() || ids.contains(&0) {
        return Err("The document library has invalid folder identifiers.".into());
    }
    let mut names = HashSet::new();
    for folder in folders {
        validate_name(&folder.name)?;
        if !names.insert((folder.parent, folder.name.to_lowercase())) {
            return Err("Two folders have the same name in this location.".into());
        }
        let mut parent = folder.parent;
        let mut visited = HashSet::from([folder.id]);
        while let Some(id) = parent {
            if !visited.insert(id) {
                return Err("The document library contains a folder cycle.".into());
            }
            parent = folders
                .iter()
                .find(|f| f.id == id)
                .ok_or("A document folder no longer exists.")?
                .parent;
        }
    }
    let mut documents_ids = HashSet::new();
    let mut file_names = HashSet::new();
    for document in documents {
        if document.id == 0 || !documents_ids.insert(document.id) {
            return Err("The document library has invalid document identifiers.".into());
        }
        validate_document(document)?;
        if document.folder.is_some_and(|id| !ids.contains(&id)) {
            return Err("A document's folder no longer exists.".into());
        }
        if !file_names.insert((document.folder, document.file_name().to_lowercase())) {
            return Err("Two documents have the same name in this location.".into());
        }
    }
    Ok(())
}
