//! Batch admission used by feed and collection apps; one save for the whole queue.
use crate::PhoneViewModel;
use phone_domain::{Id, MediaTrack, MAX_BACKUP_BYTES};
impl PhoneViewModel {
    pub fn import_media_tracks(&self, tracks: Vec<MediaTrack>) -> Result<Vec<Id>, String> {
        if self.applying.get() || tracks.len() > 256 {
            return Err("A media import is active or contains more than 256 items.".into());
        }
        let mut candidate = self.document.borrow().clone();
        let mut ids = Vec::with_capacity(tracks.len());
        for mut track in tracks {
            if track.title.trim().is_empty()
                || track.title.len() > 1024
                || track.artist.len() > 1024
                || !["audio", "video"].contains(&track.media_kind.as_str())
                || !track.duration_seconds.is_finite()
                || !(0.0..=31_536_000.0).contains(&track.duration_seconds)
            {
                return Err("The media item contains invalid metadata.".into());
            }
            if track.source.starts_with("https:") {
                track.source = phone_host::http::validate_url(&track.source)?;
            } else if !track.source.starts_with("media:")
                || track.source.len() > 256
                || track.source.chars().any(char::is_control)
            {
                return Err(
                    "Media must reference an imported file or a public HTTPS stream.".into(),
                );
            }
            if let Some(existing) = candidate
                .tracks
                .iter()
                .find(|item| item.source == track.source)
            {
                ids.push(existing.id);
                continue;
            }
            if candidate.tracks.len() >= 10_000 || candidate.next_id >= Id::MAX - 1 {
                return Err("The media library is full.".into());
            }
            track.id = candidate.allocate_id();
            ids.push(track.id);
            candidate.tracks.push(track);
        }
        candidate.normalize()?;
        if candidate.to_json()?.len() > MAX_BACKUP_BYTES {
            return Err("The phone document is full.".into());
        }
        if *self.document.borrow() == candidate {
            return Ok(ids);
        }
        self.applying.set(true);
        *self.document.borrow_mut() = candidate;
        self.persist();
        self.notify();
        self.applying.set(false);
        self.apply(None);
        Ok(ids)
    }
}
