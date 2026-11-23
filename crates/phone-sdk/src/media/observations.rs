use super::*;

/// An immutable host observation, bound to a source, document and observation
/// serial. It is never manufactured from the queue's optimistic UI position.
#[derive(Clone, Debug, PartialEq)]
pub struct PlaybackSnapshot {
    pub source: String,
    pub position: f64,
    pub duration: f64,
    document_epoch: u64,
    serial: u64,
}

impl PlaybackSnapshot {
    pub fn document_epoch(&self) -> u64 {
        self.document_epoch
    }

    /// Revalidate after callbacks, restores or asynchronous work. An observation
    /// becomes stale even when a replacement document contains the same source.
    pub fn is_current(&self, vm: &PhoneViewModel, ui: &Session) -> bool {
        self.document_epoch == vm.document_epoch()
            && !self.source.is_empty()
            && ui
                .media_source
                .with_untracked(|source| source == &self.source)
            && ui
                .state::<PlaybackSession>()
                .observed
                .borrow()
                .as_ref()
                .is_some_and(|value| value.snapshot == *self)
            && vm
                .data()
                .tracks
                .iter()
                .any(|track| track.source == self.source)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum PlaybackEvent {
    Progress(PlaybackSnapshot),
    /// A checkpoint before pausing, replacing a source, changing transport, or
    /// suspending the host. This notification does not itself pause the player.
    Paused(PlaybackSnapshot),
    /// Delivered once, before the queue or position advances to the next item.
    Ended(PlaybackSnapshot),
}

impl PlaybackEvent {
    pub fn snapshot(&self) -> &PlaybackSnapshot {
        match self {
            Self::Progress(value) | Self::Paused(value) | Self::Ended(value) => value,
        }
    }
}

pub(super) struct ObservedPlayback {
    pub snapshot: PlaybackSnapshot,
    pub ended: bool,
}

pub(super) fn snapshot(vm: &PhoneViewModel, ui: &Session) -> Option<PlaybackSnapshot> {
    let value = ui
        .state::<PlaybackSession>()
        .observed
        .borrow()
        .as_ref()
        .map(|value| value.snapshot.clone());
    value.filter(|value| value.is_current(vm, ui))
}

fn next_serial(state: &PlaybackSession) -> u64 {
    let serial = state.observation_serial.get().wrapping_add(1);
    state.observation_serial.set(serial);
    serial
}

/// Flush the last real host position without changing playback. Used before a
/// host visibility/suspension boundary, including when another app is active.
pub fn flush(vm: &Rc<PhoneViewModel>, ui: &Rc<Session>) {
    let state = ui.state::<PlaybackSession>();
    if state
        .observed
        .borrow()
        .as_ref()
        .is_some_and(|value| value.ended)
    {
        return;
    }
    if let Some(value) = snapshot(vm, ui) {
        ui.registry()
            .media_event(vm, ui, &PlaybackEvent::Paused(value));
    }
}

/// Host-only state publication. Pending queue changes leave the previous source
/// observable until the host actually switches it, so its final progress can be
/// flushed even after `start` reset the visible position for a new queue.
pub fn publish_state(
    vm: &Rc<PhoneViewModel>,
    ui: &Rc<Session>,
    title: String,
    source: String,
    playing: bool,
) {
    let state = prepare(vm, ui);
    let previous = ui.media_source.get_untracked();
    let changed = previous != source;
    let stopped = ui.media_playing.get_untracked() && !playing;
    if changed || stopped {
        let transport = state.transport_revision.get();
        let before = snapshot(vm, ui);
        flush(vm, ui);
        if before.is_some_and(|value| !value.is_current(vm, ui))
            || state.transport_revision.get() != transport
        {
            // A subscriber replaced/restored the player while receiving the
            // old-source checkpoint. Do not overwrite that newer state.
            return;
        }
    }
    if source.is_empty() {
        clear(ui);
    } else {
        let ended = state
            .observed
            .borrow()
            .as_ref()
            .is_some_and(|value| value.ended);
        let needs_binding = changed || snapshot(vm, ui).is_none() || (playing && ended);
        if needs_binding {
            let known = vm.data().tracks.iter().any(|track| track.source == source);
            state.observed.replace(known.then(|| ObservedPlayback {
                snapshot: PlaybackSnapshot {
                    source: source.clone(),
                    position: 0.0,
                    duration: 0.0,
                    document_epoch: vm.document_epoch(),
                    serial: next_serial(&state),
                },
                ended: false,
            }));
            progress(ui, 0.0, 0.0);
        }
    }
    ui.media_title.set(title);
    ui.media_source.set(source);
    ui.media_playing.set(playing);
}

/// Host-only progress publication. A source must first be bound by publish_state;
/// a late sample from an earlier document cannot resurrect its observation.
pub fn publish_progress(vm: &Rc<PhoneViewModel>, ui: &Rc<Session>, position: f64, duration: f64) {
    let state = ui.state::<PlaybackSession>();
    let Some(mut value) = snapshot(vm, ui) else {
        return;
    };
    if state
        .observed
        .borrow()
        .as_ref()
        .is_some_and(|value| value.ended)
    {
        return;
    }
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
    value.position = if duration > 0.0 {
        position.min(duration)
    } else {
        position
    };
    value.duration = duration;
    value.serial = next_serial(&state);
    state.observed.replace(Some(ObservedPlayback {
        snapshot: value.clone(),
        ended: false,
    }));
    progress(ui, value.position, value.duration);
    ui.registry()
        .media_event(vm, ui, &PlaybackEvent::Progress(value));
}
