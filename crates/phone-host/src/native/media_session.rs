//! OS now-playing adapters independent of the UI and decoder. Native callbacks
//! enqueue bounded, source-stamped commands; the owning UI thread executes them.
use std::{
    collections::VecDeque,
    sync::{Arc, Mutex},
};

#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "windows")]
mod windows;
#[cfg(target_os = "linux")]
use linux as platform;
#[cfg(target_os = "macos")]
use macos as platform;
#[cfg(target_os = "windows")]
use windows as platform;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum PlaybackState {
    #[default]
    Stopped,
    Paused,
    Playing,
}

/// No paths or network URLs cross the OS metadata boundary. A host assigns a
/// new generation when the selected source or restored phone document changes.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Snapshot {
    pub generation: u64,
    pub active: bool,
    pub title: String,
    pub artist: String,
    pub album: String,
    pub video: bool,
    pub state: PlaybackState,
    pub position_seconds: f64,
    pub duration_seconds: Option<f64>,
    pub seekable: bool,
    pub can_next: bool,
    pub can_previous: bool,
    pub volume: f64,
    /// Increment only after a seek has completed, never for ordinary playback.
    pub seek_serial: u64,
}

impl Snapshot {
    fn normalize(&mut self) {
        for text in [&mut self.title, &mut self.artist, &mut self.album] {
            *text = text
                .chars()
                .filter(|c| !c.is_control())
                .take(1024)
                .collect();
        }
        self.position_seconds = finite_seconds(self.position_seconds).unwrap_or(0.0);
        self.duration_seconds = self.duration_seconds.and_then(finite_seconds);
        if let Some(duration) = self.duration_seconds {
            self.position_seconds = self.position_seconds.min(duration);
        }
        self.volume = if self.volume.is_finite() {
            self.volume.clamp(0.0, 1.0)
        } else {
            0.0
        };
        if !self.active || self.generation == 0 {
            self.active = false;
            self.state = PlaybackState::Stopped;
            self.title.clear();
            self.artist.clear();
            self.album.clear();
            self.seekable = false;
            self.can_next = false;
            self.can_previous = false;
            self.position_seconds = 0.0;
            self.duration_seconds = None;
        }
    }
}

fn finite_seconds(value: f64) -> Option<f64> {
    // The ceiling also keeps conversions to WinRT 100 ns ticks within i64.
    (value.is_finite() && (0.0..=31_536_000.0).contains(&value)).then_some(value)
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Action {
    Play,
    Pause,
    Toggle,
    Stop,
    Next,
    Previous,
    Seek(f64),
    SeekBy(f64),
    Volume(f64),
    Raise,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Command {
    pub generation: u64,
    pub action: Action,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum Status {
    #[default]
    Pending,
    Ready,
    Unavailable(String),
}

#[derive(Default)]
struct Shared {
    state: Mutex<SharedState>,
}
#[derive(Default)]
struct SharedState {
    snapshot: Snapshot,
    commands: VecDeque<Command>,
    status: Status,
    closed: bool,
}

impl Shared {
    fn snapshot(&self) -> Snapshot {
        self.state
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .snapshot
            .clone()
    }
    fn status(&self, status: Status) {
        self.state.lock().unwrap_or_else(|e| e.into_inner()).status = status;
    }
    fn enqueue(&self, action: Action) -> bool {
        let generation = self.snapshot().generation;
        self.enqueue_for(generation, action)
    }
    fn enqueue_for(&self, generation: u64, action: Action) -> bool {
        let mut state = self.state.lock().unwrap_or_else(|e| e.into_inner());
        let s = &state.snapshot;
        if state.closed || generation != s.generation || (!s.active && action != Action::Raise) {
            return false;
        }
        let allowed = match action {
            Action::Next => s.can_next,
            Action::Previous => s.can_previous,
            Action::Seek(value) => {
                s.seekable
                    && finite_seconds(value).is_some()
                    && s.duration_seconds.is_none_or(|duration| value <= duration)
            }
            Action::SeekBy(value) => s.seekable && value.is_finite() && value.abs() <= 31_536_000.0,
            Action::Volume(value) => value.is_finite() && (0.0..=1.0).contains(&value),
            _ => true,
        };
        if !allowed || state.commands.len() >= 32 {
            return false;
        }
        state.commands.push_back(Command { generation, action });
        true
    }
}

/// Own on the application's UI thread; callbacks never capture a UI object.
/// Linux registration completes on its own worker and reports Pending meanwhile.
pub struct MediaSession {
    shared: Arc<Shared>,
    // WinRT apartment teardown and Cocoa command-center ownership must stay on
    // the thread that created the session, including Drop.
    _ui_thread: std::marker::PhantomData<std::rc::Rc<()>>,
    #[cfg(any(target_os = "linux", target_os = "macos", target_os = "windows"))]
    platform: platform::Session,
}

impl MediaSession {
    pub fn new(window: Option<isize>) -> Result<Self, String> {
        let shared = Arc::new(Shared::default());
        #[cfg(any(target_os = "linux", target_os = "macos", target_os = "windows"))]
        {
            let platform = platform::Session::new(window, shared.clone())?;
            Ok(Self {
                shared,
                platform,
                _ui_thread: std::marker::PhantomData,
            })
        }
        #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
        {
            let _ = (window, shared);
            Err("This platform does not provide native media sessions.".into())
        }
    }

    pub fn status(&self) -> Status {
        self.shared
            .state
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .status
            .clone()
    }

    pub fn publish(&self, mut snapshot: Snapshot) -> Result<(), String> {
        snapshot.normalize();
        {
            let mut state = self.shared.state.lock().unwrap_or_else(|e| e.into_inner());
            if state.snapshot == snapshot {
                return Ok(());
            }
            if state.snapshot.generation != snapshot.generation || !snapshot.active {
                state.commands.clear();
            }
            state.snapshot = snapshot.clone();
        }
        #[cfg(any(target_os = "linux", target_os = "macos", target_os = "windows"))]
        if let Err(error) = self.platform.publish(&snapshot) {
            self.shared.status(Status::Unavailable(error.clone()));
            return Err(error);
        }
        Ok(())
    }

    pub fn take_commands(&self) -> Vec<Command> {
        #[cfg(target_os = "macos")]
        self.platform.drain(&self.shared);
        self.shared
            .state
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .commands
            .drain(..)
            .collect()
    }
}

impl Drop for MediaSession {
    fn drop(&mut self) {
        let mut state = self.shared.state.lock().unwrap_or_else(|e| e.into_inner());
        state.closed = true;
        state.commands.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    pub(super) fn shared() -> Shared {
        Shared {
            state: Mutex::new(SharedState {
                snapshot: Snapshot {
                    generation: 7,
                    active: true,
                    seekable: true,
                    can_next: true,
                    duration_seconds: Some(120.0),
                    ..Default::default()
                },
                ..Default::default()
            }),
        }
    }

    #[test]
    fn remote_commands_are_bounded_and_stale_sources_cannot_seek() {
        let shared = shared();
        assert!(!shared.enqueue_for(6, Action::Seek(30.0)));
        assert!(!shared.enqueue(Action::Seek(f64::NAN)));
        assert!(!shared.enqueue(Action::Seek(-1.0)));
        assert!(!shared.enqueue(Action::Seek(121.0)));
        assert!(!shared.enqueue(Action::Previous));
        for _ in 0..32 {
            assert!(shared.enqueue(Action::Pause));
        }
        assert!(!shared.enqueue(Action::Play));
        assert_eq!(shared.state.lock().unwrap().commands.len(), 32);
    }

    #[test]
    fn inactive_or_closed_sessions_cannot_act_on_player() {
        let shared = shared();
        shared.state.lock().unwrap().snapshot.active = false;
        assert!(!shared.enqueue(Action::Play));
        assert!(shared.enqueue(Action::Raise));
        shared.state.lock().unwrap().closed = true;
        assert!(!shared.enqueue(Action::Raise));
    }

    #[test]
    fn metadata_has_bounded_plain_text_and_finite_timeline() {
        let mut snapshot = Snapshot {
            generation: 2,
            active: true,
            title: format!("a\0\nb{}", "😀".repeat(2000)),
            position_seconds: f64::INFINITY,
            duration_seconds: Some(f64::NAN),
            volume: 2.0,
            ..Default::default()
        };
        snapshot.normalize();
        assert_eq!(snapshot.title.chars().count(), 1024);
        assert!(snapshot.title.starts_with("ab"));
        assert_eq!(snapshot.position_seconds, 0.0);
        assert_eq!(snapshot.duration_seconds, None);
        assert_eq!(snapshot.volume, 1.0);
        snapshot.active = false;
        snapshot.normalize();
        assert!(snapshot.title.is_empty());
        assert!(!snapshot.seekable);
    }
}
