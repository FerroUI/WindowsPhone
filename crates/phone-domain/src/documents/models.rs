use crate::Id;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct DocumentFolder {
    pub id: Id,
    pub parent: Option<Id>,
    pub name: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct OfficeDocument {
    pub id: Id,
    pub name: String,
    pub folder: Option<Id>,
    pub modified: i64,
    pub content: DocumentContent,
    /// Import does not claim full desktop layout fidelity. Export is a named copy.
    pub import_notice: String,
}
impl Default for OfficeDocument {
    fn default() -> Self {
        Self {
            id: 0,
            name: "Document".into(),
            folder: None,
            modified: 0,
            content: DocumentContent::Word(WordDocument::default()),
            import_notice: String::new(),
        }
    }
}
impl OfficeDocument {
    pub fn blank(kind: DocumentKind, name: String, folder: Option<Id>) -> Self {
        Self {
            name,
            folder,
            content: match kind {
                DocumentKind::Word => DocumentContent::Word(WordDocument::default()),
                DocumentKind::Excel => DocumentContent::Excel(Workbook::default()),
                DocumentKind::PowerPoint => DocumentContent::PowerPoint(Presentation::default()),
            },
            ..Self::default()
        }
    }
    pub fn kind(&self) -> DocumentKind {
        self.content.kind()
    }
    pub fn file_name(&self) -> String {
        format!("{}.{}", self.name, self.kind().extension())
    }
    pub fn text_size(&self) -> usize {
        match &self.content {
            DocumentContent::Word(doc) => doc.paragraphs.iter().map(|p| p.text.len()).sum(),
            DocumentContent::Excel(book) => book
                .sheets
                .iter()
                .flat_map(|s| s.cells.values())
                .map(String::len)
                .sum(),
            DocumentContent::PowerPoint(deck) => deck
                .slides
                .iter()
                .map(|s| s.title.len() + s.body.len() + s.notes.len())
                .sum(),
        }
    }
    pub fn plain_text(&self) -> String {
        match &self.content {
            DocumentContent::Word(doc) => doc
                .paragraphs
                .iter()
                .map(|p| p.text.as_str())
                .collect::<Vec<_>>()
                .join("\n\n"),
            DocumentContent::Excel(book) => book
                .sheets
                .iter()
                .map(|s| format!("{}\n{}", s.name, super::sheet_to_csv(s)))
                .collect::<Vec<_>>()
                .join("\n\n"),
            DocumentContent::PowerPoint(deck) => deck
                .slides
                .iter()
                .map(|s| format!("{}\n{}", s.title, s.body))
                .collect::<Vec<_>>()
                .join("\n\n"),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum DocumentKind {
    Word,
    Excel,
    PowerPoint,
}
impl DocumentKind {
    pub fn label(self) -> &'static str {
        match self {
            Self::Word => "Word",
            Self::Excel => "Excel",
            Self::PowerPoint => "PowerPoint",
        }
    }
    pub fn extension(self) -> &'static str {
        match self {
            Self::Word => "docx",
            Self::Excel => "xlsx",
            Self::PowerPoint => "pptx",
        }
    }
    pub fn color(self) -> &'static str {
        match self {
            Self::Word => "#185ABD",
            Self::Excel => "#107C41",
            Self::PowerPoint => "#D24726",
        }
    }
    pub fn letter(self) -> &'static str {
        match self {
            Self::Word => "W",
            Self::Excel => "X",
            Self::PowerPoint => "P",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum DocumentContent {
    Word(WordDocument),
    Excel(Workbook),
    PowerPoint(Presentation),
}
impl DocumentContent {
    pub fn kind(&self) -> DocumentKind {
        match self {
            Self::Word(_) => DocumentKind::Word,
            Self::Excel(_) => DocumentKind::Excel,
            Self::PowerPoint(_) => DocumentKind::PowerPoint,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct WordDocument {
    pub paragraphs: Vec<Paragraph>,
}
impl Default for WordDocument {
    fn default() -> Self {
        Self {
            paragraphs: vec![Paragraph::default()],
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Paragraph {
    pub text: String,
    pub bold: bool,
    pub italic: bool,
    pub underline: bool,
    pub heading: u8,
    pub bullet: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Workbook {
    pub sheets: Vec<Worksheet>,
}
impl Default for Workbook {
    fn default() -> Self {
        Self {
            sheets: vec![Worksheet::default()],
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Worksheet {
    pub name: String,
    pub cells: BTreeMap<String, String>,
}
impl Default for Worksheet {
    fn default() -> Self {
        Self {
            name: "Sheet1".into(),
            cells: BTreeMap::new(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Presentation {
    pub slides: Vec<Slide>,
}
impl Default for Presentation {
    fn default() -> Self {
        Self {
            slides: vec![Slide::default()],
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Slide {
    pub title: String,
    pub body: String,
    pub notes: String,
}
