use serde::{Deserialize, Serialize};
pub const MAX_EPISODES: usize = 32;
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Episode {
    pub title: String,
    pub source: String,
    pub description: String,
    pub published: i64,
    pub duration: u32,
    pub video: bool,
    pub played: bool,
    pub position: u32,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Show {
    pub id: u32,
    pub title: String,
    pub description: String,
    pub feed: Option<String>,
    pub episodes: Vec<Episode>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Saved {
    pub schema: u32,
    pub next_id: u32,
    pub shows: Vec<Show>,
}
impl Default for Saved {
    fn default() -> Self {
        Self {
            schema: 1,
            next_id: 1,
            shows: Vec::new(),
        }
    }
}
impl Saved {
    pub fn validate(&self) -> Result<(), String> {
        if self.schema != 1
            || self.shows.len() > 20
            || self.next_id == 0
            || self
                .shows
                .iter()
                .map(|show| show.episodes.len())
                .sum::<usize>()
                > 256
        {
            return Err(
                "The podcast collection is unsupported or full (20 shows / 256 episodes).".into(),
            );
        }
        let mut ids = std::collections::HashSet::new();
        let mut feeds = std::collections::HashSet::new();
        for show in &self.shows {
            if show.id == 0
                || show.id >= self.next_id
                || !ids.insert(show.id)
                || !valid_text(&show.title, 240)
                || show.description.len() > 1024
                || show.episodes.len() > MAX_EPISODES
            {
                return Err("The podcast collection contains invalid show metadata.".into());
            }
            if let Some(feed) = &show.feed {
                phone_host::http::validate_url(feed)?;
                if !feeds.insert(feed) {
                    return Err("A podcast feed is subscribed twice.".into());
                }
            }
            let mut sources = std::collections::HashSet::new();
            for episode in &show.episodes {
                phone_host::http::validate_url(&episode.source)?;
                if !sources.insert(&episode.source)
                    || !valid_text(&episode.title, 240)
                    || episode.description.len() > 1024
                    || episode.duration > 31_536_000
                    || episode.position > 31_536_000
                {
                    return Err("The podcast collection contains invalid episode metadata.".into());
                }
            }
        }
        Ok(())
    }
    pub fn upsert(&mut self, mut show: Show) -> Result<u32, String> {
        let existing = if show.id != 0 {
            self.shows.iter().position(|old| old.id == show.id)
        } else {
            show.feed.as_ref().and_then(|feed| {
                self.shows
                    .iter()
                    .position(|old| old.feed.as_ref() == Some(feed))
            })
        };
        if let Some(index) = existing {
            let old = &self.shows[index];
            show.id = old.id;
            for episode in &mut show.episodes {
                if let Some(previous) = old.episodes.iter().find(|old| old.source == episode.source)
                {
                    episode.played = previous.played;
                    episode.position = previous.position;
                }
            }
            self.shows[index] = show;
            self.validate()?;
            Ok(self.shows[index].id)
        } else {
            if show.id != 0 {
                return Err("The podcast was removed while refreshing.".into());
            }
            show.id = self.next_id;
            self.next_id = self
                .next_id
                .checked_add(1)
                .ok_or("Podcast identifiers are exhausted.")?;
            let id = show.id;
            self.shows.push(show);
            self.validate()?;
            Ok(id)
        }
    }
}
fn valid_text(text: &str, max: usize) -> bool {
    !text.trim().is_empty() && text.chars().count() <= max && !text.chars().any(char::is_control)
}
