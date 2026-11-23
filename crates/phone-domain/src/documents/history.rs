//! Bounded content history. Library metadata is deliberately outside editor undo.
use super::DocumentContent;
use crate::Id;
use std::collections::VecDeque;

const MAX_STEPS: usize = 32;
const MAX_BYTES: usize = 8 * 1024 * 1024;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EditDirection {
    Undo,
    Redo,
}

#[derive(Default)]
pub struct EditHistory {
    identity: Option<(u64, Id)>,
    current: Option<DocumentContent>,
    undo: VecDeque<(DocumentContent, usize)>,
    redo: VecDeque<(DocumentContent, usize)>,
    bytes: usize,
}

impl EditHistory {
    /// A replacement phone document or an edit outside this editor invalidates
    /// old history. Undo cannot resurrect data from before restore/reset.
    pub fn record(
        &mut self,
        epoch: u64,
        id: Id,
        before: &DocumentContent,
        after: &DocumentContent,
    ) {
        if self.identity != Some((epoch, id)) || self.current.as_ref() != Some(before) {
            *self = Self::default();
            self.identity = Some((epoch, id));
        }
        self.current = Some(after.clone());
        if before == after {
            return;
        }
        self.bytes -= self.redo.iter().map(|(_, size)| size).sum::<usize>();
        self.redo.clear();
        let size = estimate_bytes(before);
        if size > MAX_BYTES {
            self.undo.clear();
            self.bytes = 0;
            return;
        }
        self.undo.push_back((before.clone(), size));
        self.bytes += size;
        self.trim();
    }

    pub fn candidate(
        &self,
        epoch: u64,
        id: Id,
        current: &DocumentContent,
        direction: EditDirection,
    ) -> Option<&DocumentContent> {
        if self.identity != Some((epoch, id)) || self.current.as_ref() != Some(current) {
            return None;
        }
        match direction {
            EditDirection::Undo => self.undo.back(),
            EditDirection::Redo => self.redo.back(),
        }
        .map(|(content, _)| content)
    }

    /// Commit a previously validated candidate only after the document accepts
    /// it. A rejected library mutation leaves both stacks unchanged.
    pub fn accept(&mut self, direction: EditDirection, content: &DocumentContent) -> bool {
        let (source, destination) = match direction {
            EditDirection::Undo => (&mut self.undo, &mut self.redo),
            EditDirection::Redo => (&mut self.redo, &mut self.undo),
        };
        if source.back().is_none_or(|(next, _)| next != content) {
            return false;
        }
        let (_, removed) = source.pop_back().expect("checked above");
        self.bytes -= removed;
        if let Some(previous) = self.current.replace(content.clone()) {
            let size = estimate_bytes(&previous);
            destination.push_back((previous, size));
            self.bytes += size;
        }
        self.trim();
        true
    }

    fn trim(&mut self) {
        while self.undo.len() + self.redo.len() > MAX_STEPS || self.bytes > MAX_BYTES {
            let removed = if self.undo.len() > self.redo.len() {
                self.undo.pop_front()
            } else {
                self.redo.pop_front()
            };
            let Some((_, size)) = removed else { break };
            self.bytes -= size;
        }
    }
}

fn estimate_bytes(content: &DocumentContent) -> usize {
    // Count payloads plus conservative container/node overhead. No unbounded
    // serialized scratch copy is needed to account for a sparse workbook.
    std::mem::size_of::<DocumentContent>()
        + match content {
            DocumentContent::Word(word) => word
                .paragraphs
                .iter()
                .map(|p| std::mem::size_of_val(p) + p.text.len())
                .sum::<usize>(),
            DocumentContent::Excel(book) => book
                .sheets
                .iter()
                .map(|sheet| {
                    std::mem::size_of_val(sheet)
                        + sheet.name.len()
                        + sheet
                            .cells
                            .iter()
                            .map(|(address, value)| 128 + address.len() + value.len())
                            .sum::<usize>()
                })
                .sum::<usize>(),
            DocumentContent::PowerPoint(deck) => deck
                .slides
                .iter()
                .map(|s| std::mem::size_of_val(s) + s.title.len() + s.body.len() + s.notes.len())
                .sum::<usize>(),
        }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Paragraph, WordDocument};
    fn content(text: &str) -> DocumentContent {
        DocumentContent::Word(WordDocument {
            paragraphs: vec![Paragraph {
                text: text.into(),
                ..Default::default()
            }],
        })
    }
    #[test]
    fn undo_redo_branches_and_replacement_are_isolated() {
        let mut history = EditHistory::default();
        let (a, b, c) = (content("a"), content("b"), content("c"));
        history.record(0, 42, &a, &b);
        assert_eq!(history.candidate(0, 42, &b, EditDirection::Undo), Some(&a));
        assert!(history.candidate(1, 42, &b, EditDirection::Undo).is_none());
        assert!(history.candidate(0, 43, &b, EditDirection::Undo).is_none());
        assert!(history.candidate(0, 42, &c, EditDirection::Undo).is_none());
        assert!(!history.accept(EditDirection::Undo, &c));
        assert!(history.accept(EditDirection::Undo, &a));
        assert_eq!(history.candidate(0, 42, &a, EditDirection::Redo), Some(&b));
        history.record(0, 42, &a, &c);
        assert!(history.candidate(0, 42, &c, EditDirection::Redo).is_none());
        history.record(1, 42, &a, &b);
        assert!(history.accept(EditDirection::Undo, &a));
        assert!(history.candidate(1, 42, &a, EditDirection::Undo).is_none());
    }
    #[test]
    fn history_evicts_old_entries_and_obeys_memory_bound() {
        let mut history = EditHistory::default();
        let mut before = content("");
        for index in 0..80 {
            let after = content(&format!("{index}{}", "x".repeat(256 * 1024)));
            history.record(0, 1, &before, &after);
            before = after;
        }
        assert!(history.bytes <= MAX_BYTES);
        assert!(history.undo.len() + history.redo.len() <= MAX_STEPS);
        let mut undos = 0;
        while let Some(next) = history
            .candidate(0, 1, &before, EditDirection::Undo)
            .cloned()
        {
            assert!(history.accept(EditDirection::Undo, &next));
            before = next;
            undos += 1;
        }
        assert!((1..=32).contains(&undos));
    }
}
