//! Deterministic tile cadence uses the existing clock; each update produces
//! one finite native animation and never creates a per-tile timer.
use ferroui_controls::Control;
use ferroui_view::prelude::*;

/// Read just the visible prefix; a pinned workbook must never serialize its
/// entire cell collection on the shell's clock tick.
pub(super) fn document_preview(document: &phone_domain::OfficeDocument, limit: usize) -> String {
    let parts: Box<dyn Iterator<Item = &str> + '_> = match &document.content {
        phone_domain::DocumentContent::Word(word) => Box::new(
            word.paragraphs
                .iter()
                .map(|paragraph| paragraph.text.as_str()),
        ),
        phone_domain::DocumentContent::Excel(book) => Box::new(
            book.sheets
                .iter()
                .flat_map(|sheet| sheet.cells.values().map(String::as_str)),
        ),
        phone_domain::DocumentContent::PowerPoint(deck) => Box::new(
            deck.slides
                .iter()
                .flat_map(|slide| [slide.title.as_str(), slide.body.as_str()]),
        ),
    };
    parts
        .flat_map(|part| part.chars().chain(std::iter::once('\n')))
        .take(limit)
        .collect()
}

pub(super) fn frame(key: String, content: Element, distance: f64, animate: bool) -> Element {
    border()
        .key(key)
        .clip_to_bounds(true)
        .on_loaded(move |event| {
            if animate {
                if let Some(control) = event.source().and_then(|source| source.cast::<Control>()) {
                    phone_controls::live_tile_enter(&control, distance);
                }
            }
        })
        .child(content)
        .into()
}
