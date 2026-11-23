//! Shared playback queue used by app commands and host media keys.
//!
//! The queue contains references to the current phone's media library. It is
//! transient: restoring a phone never resumes an old document's media sources.
mod observations;
mod queue;
#[cfg(test)]
mod tests;
pub use observations::{flush, publish_progress, publish_state, PlaybackEvent, PlaybackSnapshot};
pub use queue::{Advance, MediaQueue, RepeatMode, MAX_QUEUE_ITEMS};

use crate::session::Session;
use ferroui_view::prelude::*;
use phone_state::{HostRequest, PhoneViewModel};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

pub struct PlaybackSession {
    pub queue: Signal<MediaQueue>,
    pub position: Signal<f64>,
    pub duration: Signal<f64>,
    epoch: Cell<Option<u64>>,
    // Host observations survive a pending queue change until its source really
    // changes. The visible position may already have reset for the next item.
    observed: RefCell<Option<observations::ObservedPlayback>>,
    observation_serial: Cell<u64>,
    transport_revision: Cell<u64>,
}

impl Default for PlaybackSession {
    fn default() -> Self {
        Self {
            queue: signal(MediaQueue::default()),
            position: signal(0.0),
            duration: signal(0.0),
            epoch: Cell::new(None),
            observed: RefCell::new(None),
            observation_serial: Cell::new(0),
            transport_revision: Cell::new(0),
        }
    }
}

fn prepare(vm: &PhoneViewModel, ui: &Session) -> std::rc::Rc<PlaybackSession> {
    let state = ui.state::<PlaybackSession>();
    if state.epoch.get() != Some(vm.document_epoch()) {
        state.queue.set(MediaQueue::default());
        state.position.set(0.0);
        state.duration.set(0.0);
        state.observed.borrow_mut().take();
        state
            .transport_revision
            .set(state.transport_revision.get().wrapping_add(1));
        state.epoch.set(Some(vm.document_epoch()));
    }
    state
}

/// Starts a bounded queue of existing library sources. The host alone reports
/// whether playback actually started; selecting an item does not assert success.
pub fn start(
    vm: &PhoneViewModel,
    ui: &Session,
    sources: Vec<String>,
    selected: &str,
    repeat: RepeatMode,
    shuffle: bool,
) -> Result<(), String> {
    let data = vm.data();
    let known = data
        .tracks
        .iter()
        .map(|t| t.source.as_str())
        .collect::<std::collections::HashSet<_>>();
    if sources
        .iter()
        .any(|source| !known.contains(source.as_str()))
    {
        return Err("A queued media item is no longer in your collection.".into());
    }
    let queue = MediaQueue::new(sources, selected, repeat, shuffle, vm.now() as u64)?;
    drop(data);
    let state = prepare(vm, ui);
    state
        .transport_revision
        .set(state.transport_revision.get().wrapping_add(1));
    state.queue.set(queue);
    state.position.set(0.0);
    state.duration.set(0.0);
    vm.request(HostRequest::PlayMedia {
        source: selected.into(),
    });
    Ok(())
}

/// Handles queue commands, returning false for controls owned by the host.
/// Both on-screen commands and OS transports must call this same entry point.
pub fn control(vm: &Rc<PhoneViewModel>, ui: &Rc<Session>, action: &str) -> bool {
    let advance = match action {
        "next" => Advance::Next,
        "previous" => Advance::Previous,
        "ended" => Advance::Ended,
        _ => return false,
    };
    let state = prepare(vm, ui);
    let transport = state.transport_revision.get();
    if let Some(snapshot) = observations::snapshot(vm, ui) {
        let event = if advance == Advance::Ended {
            let mut observed = state.observed.borrow_mut();
            let Some(observed) = observed.as_mut() else {
                return true;
            };
            if observed.ended {
                return true;
            }
            // Latch completion before callbacks: a reentrant or repeated host
            // notification cannot advance the queue twice.
            observed.ended = true;
            Some(PlaybackEvent::Ended(snapshot))
        } else if !state
            .observed
            .borrow()
            .as_ref()
            .is_some_and(|value| value.ended)
        {
            Some(PlaybackEvent::Paused(snapshot))
        } else {
            None
        };
        if let Some(event) = event {
            ui.registry().media_event(vm, ui, &event);
            if !event.snapshot().is_current(vm, ui) || state.transport_revision.get() != transport {
                return true;
            }
        }
    } else if advance == Advance::Ended {
        // A completion without a current, epoch-bound host observation is stale.
        return true;
    }
    state.transport_revision.set(transport.wrapping_add(1));
    let mut queue = state.queue.get_untracked();
    let current = ui.media_source.get_untracked();
    let data = vm.data();
    let known = data
        .tracks
        .iter()
        .map(|t| t.source.as_str())
        .collect::<std::collections::HashSet<_>>();
    let removed_current = queue.retain(|source| known.contains(source));
    if queue.is_empty() && !current.is_empty() {
        if let Some(selected) = data.tracks.iter().find(|track| track.source == current) {
            let sources = data
                .tracks
                .iter()
                .filter(|track| track.media_kind == selected.media_kind)
                .map(|track| track.source.clone())
                .collect();
            queue =
                MediaQueue::new(sources, &current, RepeatMode::Off, false, 1).unwrap_or_default();
        }
    }
    drop(data);
    let source = if removed_current && !queue.is_empty() {
        queue.current().map(str::to_owned)
    } else {
        queue.advance(advance).map(str::to_owned)
    };
    state.queue.set(queue);
    if let Some(source) = source {
        state.position.set(0.0);
        if source == current {
            vm.request(HostRequest::MediaControl {
                action: "restart".into(),
            });
        } else {
            state.duration.set(0.0);
            vm.request(HostRequest::PlayMedia { source });
        }
    } else if advance == Advance::Next {
        vm.request(HostRequest::MediaControl {
            action: "stop".into(),
        });
    }
    true
}

pub fn capabilities(vm: &PhoneViewModel, ui: &Session) -> (bool, bool) {
    let state = prepare(vm, ui);
    let data = vm.data();
    let known = data
        .tracks
        .iter()
        .map(|track| track.source.as_str())
        .collect::<std::collections::HashSet<_>>();
    state.queue.with_untracked(|queue| {
        let mut any = false;
        let mut next = false;
        for (index, source) in queue.sources().enumerate() {
            if known.contains(source) {
                any = true;
                next |= index > queue.position();
            }
        }
        (any && (next || queue.repeat == RepeatMode::All), any)
    })
}

pub fn clear(ui: &Session) {
    let state = ui.state::<PlaybackSession>();
    state.queue.set(MediaQueue::default());
    state.position.set(0.0);
    state.duration.set(0.0);
    state.epoch.set(None);
    state.observed.borrow_mut().take();
    state
        .transport_revision
        .set(state.transport_revision.get().wrapping_add(1));
}

/// The supplied values are observations from the active host player, in seconds.
pub fn progress(ui: &Session, position: f64, duration: f64) {
    let state = ui.state::<PlaybackSession>();
    let duration = if duration.is_finite() && duration > 0.0 {
        duration
    } else {
        0.0
    };
    let position = if position.is_finite() {
        position.max(0.0)
    } else {
        0.0
    };
    let position = if duration > 0.0 {
        position.min(duration)
    } else {
        position
    };
    if (state.position.get_untracked() - position).abs() >= 0.25 || position == 0.0 {
        state.position.set(position);
    }
    if state.duration.get_untracked() != duration {
        state.duration.set(duration);
    }
}
