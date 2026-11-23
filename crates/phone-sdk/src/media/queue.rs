use serde::{Deserialize, Serialize};

pub const MAX_QUEUE_ITEMS: usize = 10_000;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum RepeatMode {
    #[default]
    Off,
    All,
    One,
}
impl RepeatMode {
    pub fn next(self) -> Self {
        match self {
            Self::Off => Self::All,
            Self::All => Self::One,
            Self::One => Self::Off,
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            Self::Off => "repeat off",
            Self::All => "repeat all",
            Self::One => "repeat one",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Advance {
    Previous,
    Next,
    Ended,
}

/// Keeps original order and a permutation separately so switching shuffle off
/// restores the playlist without moving the currently playing item.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct MediaQueue {
    sources: Vec<String>,
    order: Vec<usize>,
    cursor: usize,
    pub repeat: RepeatMode,
    shuffled: bool,
}
impl MediaQueue {
    pub fn new(
        sources: Vec<String>,
        selected: &str,
        repeat: RepeatMode,
        shuffle: bool,
        seed: u64,
    ) -> Result<Self, String> {
        if sources.is_empty() || sources.len() > MAX_QUEUE_ITEMS {
            return Err("Choose between 1 and 10,000 media items.".into());
        }
        let mut seen = std::collections::HashSet::new();
        if sources.iter().any(|source| {
            source.is_empty()
                || source.len() > 4096
                || source.chars().any(char::is_control)
                || !seen.insert(source.as_str())
        }) {
            return Err("The playback queue contains an invalid or duplicate source.".into());
        }
        let cursor = sources
            .iter()
            .position(|source| source == selected)
            .ok_or("The selected item is not in this queue.")?;
        let order = (0..sources.len()).collect();
        let mut result = Self {
            sources,
            order,
            cursor,
            repeat,
            shuffled: false,
        };
        result.set_shuffle(shuffle, seed);
        Ok(result)
    }
    pub fn is_empty(&self) -> bool {
        self.order.is_empty()
    }
    pub fn len(&self) -> usize {
        self.order.len()
    }
    pub fn position(&self) -> usize {
        self.cursor
    }
    pub fn shuffled(&self) -> bool {
        self.shuffled
    }
    pub fn current(&self) -> Option<&str> {
        self.order
            .get(self.cursor)
            .map(|index| self.sources[*index].as_str())
    }
    pub fn sources(&self) -> impl Iterator<Item = &str> {
        self.order.iter().map(|index| self.sources[*index].as_str())
    }
    pub fn can_next(&self) -> bool {
        !self.is_empty() && (self.cursor + 1 < self.len() || self.repeat == RepeatMode::All)
    }
    pub fn can_previous(&self) -> bool {
        !self.is_empty()
    }
    pub fn set_shuffle(&mut self, shuffle: bool, mut seed: u64) {
        if self.shuffled == shuffle || self.is_empty() {
            self.shuffled = shuffle;
            return;
        }
        let current = self.order[self.cursor];
        self.order = (0..self.sources.len()).collect();
        if shuffle {
            self.order.swap(0, current);
            seed = seed.max(1);
            for index in (2..self.order.len()).rev() {
                seed ^= seed << 13;
                seed ^= seed >> 7;
                seed ^= seed << 17;
                let other = 1 + (seed % index as u64) as usize;
                self.order.swap(index, other);
            }
            self.cursor = 0;
        } else {
            self.cursor = current;
        }
        self.shuffled = shuffle;
    }
    pub fn select(&mut self, source: &str) -> bool {
        if let Some(index) = self
            .order
            .iter()
            .position(|index| self.sources[*index] == source)
        {
            self.cursor = index;
            true
        } else {
            false
        }
    }
    pub fn advance(&mut self, advance: Advance) -> Option<&str> {
        if self.is_empty() {
            return None;
        }
        match advance {
            Advance::Ended if self.repeat == RepeatMode::One => {}
            Advance::Previous if self.cursor > 0 => self.cursor -= 1,
            Advance::Previous if self.repeat == RepeatMode::All => self.cursor = self.len() - 1,
            Advance::Previous => {}
            Advance::Next | Advance::Ended if self.cursor + 1 < self.len() => self.cursor += 1,
            Advance::Next | Advance::Ended if self.repeat == RepeatMode::All => self.cursor = 0,
            Advance::Next | Advance::Ended => return None,
        }
        self.current()
    }
    /// Removes stale references without changing the order of surviving items.
    /// Returns true when the current item disappeared and selects its successor.
    pub fn retain(&mut self, mut keep: impl FnMut(&str) -> bool) -> bool {
        let current = self.current().map(str::to_owned);
        let old_cursor = self.cursor;
        let original = self.sources.clone();
        let mut remap = vec![None; original.len()];
        let mut retained = Vec::new();
        for (index, source) in original.iter().enumerate() {
            if keep(source) {
                remap[index] = Some(retained.len());
                retained.push(source.clone());
            }
        }
        let before = self
            .order
            .iter()
            .take(old_cursor)
            .filter(|index| remap[**index].is_some())
            .count();
        self.order.retain(|index| remap[*index].is_some());
        for index in &mut self.order {
            *index = remap[*index].expect("retained source");
        }
        self.sources = retained;
        let removed = current
            .as_ref()
            .is_some_and(|source| !self.sources.contains(source));
        self.cursor = before.min(self.order.len().saturating_sub(1));
        removed
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn queue() -> MediaQueue {
        MediaQueue::new(
            vec!["a".into(), "b".into(), "c".into()],
            "a",
            RepeatMode::Off,
            false,
            1,
        )
        .unwrap()
    }
    #[test]
    fn queue_stops_without_repeat_and_repeats_only_requested_modes() {
        let mut q = queue();
        assert_eq!(q.advance(Advance::Next), Some("b"));
        assert_eq!(q.advance(Advance::Ended), Some("c"));
        assert_eq!(q.advance(Advance::Ended), None);
        q.repeat = RepeatMode::One;
        assert_eq!(q.advance(Advance::Ended), Some("c"));
        assert_eq!(q.advance(Advance::Next), None);
        q.repeat = RepeatMode::All;
        assert_eq!(q.advance(Advance::Ended), Some("a"));
        assert_eq!(q.advance(Advance::Previous), Some("c"));
    }
    #[test]
    fn shuffle_preserves_current_and_every_source_once_then_restores_original_order() {
        let mut q = queue();
        q.select("b");
        q.set_shuffle(true, 829);
        assert_eq!(q.current(), Some("b"));
        assert_eq!(q.position(), 0);
        let mut seen = q.sources().collect::<Vec<_>>();
        seen.sort();
        assert_eq!(seen, vec!["a", "b", "c"]);
        q.set_shuffle(false, 1);
        assert_eq!(q.current(), Some("b"));
        assert_eq!(q.sources().collect::<Vec<_>>(), vec!["a", "b", "c"]);
    }
    #[test]
    fn removing_references_preserves_position_and_chooses_successor() {
        let mut q = queue();
        q.select("b");
        assert!(!q.retain(|source| source != "a"));
        assert_eq!(q.current(), Some("b"));
        assert!(q.retain(|source| source != "b"));
        assert_eq!(q.current(), Some("c"));
        assert!(q.retain(|_| false));
        assert!(q.is_empty());
    }
    #[test]
    fn invalid_queue_is_rejected_without_silent_truncation() {
        assert!(
            MediaQueue::new(vec!["a".into(), "a".into()], "a", RepeatMode::Off, false, 1).is_err()
        );
        assert!(MediaQueue::new(vec!["a".into()], "b", RepeatMode::Off, false, 1).is_err());
        assert!(MediaQueue::new(vec![], "", RepeatMode::Off, false, 1).is_err());
    }
}
