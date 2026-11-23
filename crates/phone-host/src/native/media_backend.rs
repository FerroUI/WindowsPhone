//! Decoder-independent native media-session extension point. Mobile plugins can
//! provide their platform session while retaining the shared transport engine.
use super::media_session::{Command, MediaSession, Snapshot, Status};
pub trait MediaBackend {
    fn status(&self) -> Status;
    fn publish(&self, snapshot: &Snapshot) -> Result<(), String>;
    fn take_commands(&self) -> Vec<Command>;
}
impl MediaBackend for MediaSession {
    fn status(&self) -> Status {
        MediaSession::status(self)
    }
    fn publish(&self, snapshot: &Snapshot) -> Result<(), String> {
        MediaSession::publish(self, snapshot.clone())
    }
    fn take_commands(&self) -> Vec<Command> {
        MediaSession::take_commands(self)
    }
}
