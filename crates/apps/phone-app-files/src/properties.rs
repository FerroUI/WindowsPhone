//! Properties are a bounded read-only snapshot, never a status toast or disk estimate.
use super::*;
use phone_domain::{export_office, DocumentContent};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FileProperties {
    pub name: String,
    pub rows: Vec<(String, String)>,
}

fn counted(count: usize, noun: &str) -> String {
    format!("{count} {noun}{}", if count == 1 { "" } else { "s" })
}

pub(super) fn folder(vm: &PhoneViewModel, id: Id) -> Result<FileProperties, String> {
    let data = vm.data();
    let folder = data
        .document_folders
        .iter()
        .find(|f| f.id == id)
        .ok_or("This folder no longer exists.")?;
    let mut descendants = BTreeSet::from([id]);
    for _ in 0..phone_domain::MAX_FOLDERS {
        let previous = descendants.len();
        for folder in &data.document_folders {
            if folder.parent.is_some_and(|p| descendants.contains(&p)) {
                descendants.insert(folder.id);
            }
        }
        if descendants.len() == previous {
            break;
        }
    }
    let documents = data
        .documents
        .iter()
        .filter(|d| d.folder.is_some_and(|f| descendants.contains(&f)))
        .collect::<Vec<_>>();
    Ok(FileProperties {
        name: folder.name.clone(),
        rows: vec![
            ("type".into(), "folder".into()),
            ("location".into(), breadcrumb(vm, folder.parent)),
            (
                "contains".into(),
                format!(
                    "{}, {}",
                    counted(documents.len(), "file"),
                    counted(descendants.len().saturating_sub(1), "folder")
                ),
            ),
            (
                "document text".into(),
                size_text(documents.iter().map(|d| d.text_size()).sum()),
            ),
        ],
    })
}

pub(super) fn document(vm: &PhoneViewModel, id: Id) -> Result<FileProperties, String> {
    let data = vm.data();
    let document = data.document(id).ok_or("This document no longer exists.")?;
    let exported = export_office(document)?;
    let details = match &document.content {
        DocumentContent::Word(word) => counted(word.paragraphs.len(), "paragraph"),
        DocumentContent::Excel(book) => format!(
            "{}, {}",
            counted(book.sheets.len(), "worksheet"),
            counted(book.sheets.iter().map(|s| s.cells.len()).sum(), "cell")
        ),
        DocumentContent::PowerPoint(deck) => counted(deck.slides.len(), "slide"),
    };
    let mut rows = vec![
        (
            "type".into(),
            format!(
                "{} document (.{})",
                document.kind().label(),
                document.kind().extension()
            ),
        ),
        ("location".into(), breadcrumb(vm, document.folder)),
        (
            "modified".into(),
            phone_office_support::date_text(document.modified),
        ),
        (
            "size of saved copy".into(),
            format!(
                "{} ({} bytes)",
                size_text(exported.bytes.len()),
                exported.bytes.len()
            ),
        ),
        ("contains".into(), details),
    ];
    if !document.import_notice.is_empty() {
        rows.push(("saved copy".into(), exported.name));
    }
    Ok(FileProperties {
        name: document.file_name(),
        rows,
    })
}

pub(super) fn view(ui: Rc<Session>, properties: FileProperties) -> Element {
    let close = ui.clone();
    let rows = properties
        .rows
        .into_iter()
        .map(|(label, value)| {
            rsx! {StackPanel{spacing:4.0,
                {muted(&label, 16.0)}
                {text(&value, 23.0)}
            }}
        })
        .collect::<Vec<_>>();
    let body = rsx! {StackPanel{spacing:20.0,margin:(20.0,4.0,20.0,24.0),
        {text("properties", 48.0)}
        {text(&properties.name, 28.0)}
        for row in rows {{row}}
    }};
    page(
        "files",
        scroll(body),
        app_bar(
            vec![action(Icon::Check, "done", move || {
                close.state::<crate::FilesSession>().clear_properties()
            })],
            false,
            vec![],
        ),
    )
}
