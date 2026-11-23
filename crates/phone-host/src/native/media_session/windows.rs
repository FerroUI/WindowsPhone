use super::{Action, PlaybackState, Shared, Snapshot, Status};
use std::{cell::RefCell, sync::Arc};
use windows::{
    core::HSTRING,
    Foundation::{TimeSpan, TypedEventHandler},
    Media::{
        MediaPlaybackStatus, MediaPlaybackType, PlaybackPositionChangeRequestedEventArgs,
        SystemMediaTransportControls, SystemMediaTransportControlsButton,
        SystemMediaTransportControlsButtonPressedEventArgs,
        SystemMediaTransportControlsTimelineProperties,
    },
    Win32::{
        Foundation::HWND,
        System::WinRT::{
            ISystemMediaTransportControlsInterop, RoInitialize, RoUninitialize,
            RO_INIT_SINGLETHREADED,
        },
    },
};

struct Runtime(bool);
impl Drop for Runtime {
    fn drop(&mut self) {
        if self.0 {
            unsafe {
                RoUninitialize();
            }
        }
    }
}

pub(super) struct Session {
    controls: SystemMediaTransportControls,
    button_token: i64,
    seek_token: i64,
    previous: RefCell<Snapshot>,
    _runtime: Runtime,
}

impl Session {
    pub(super) fn new(window: Option<isize>, shared: Arc<Shared>) -> Result<Self, String> {
        let handle = window
            .filter(|value| *value != 0)
            .ok_or("A native window is required for Windows media controls.")?;
        let runtime = match unsafe { RoInitialize(RO_INIT_SINGLETHREADED) } {
            Ok(()) => Runtime(true),
            Err(error) if error.code().0 == 0x80010106u32 as i32 => Runtime(false),
            Err(error) => return Err(error.to_string()),
        };
        let factory = windows::core::factory::<
            SystemMediaTransportControls,
            ISystemMediaTransportControlsInterop,
        >()
        .map_err(|e| e.to_string())?;
        let controls: SystemMediaTransportControls =
            unsafe { factory.GetForWindow(HWND(handle as *mut _)) }.map_err(|e| e.to_string())?;
        // No process-wide window hooks or synthetic keystrokes. The OS routes
        // commands only to the transport controller associated with our HWND.
        let commands = shared.clone();
        let button_token = controls
            .ButtonPressed(&TypedEventHandler::<
                SystemMediaTransportControls,
                SystemMediaTransportControlsButtonPressedEventArgs,
            >::new(move |_, args| {
                if let Some(args) = args.as_ref() {
                    let action = match args.Button()? {
                        SystemMediaTransportControlsButton::Play => Some(Action::Play),
                        SystemMediaTransportControlsButton::Pause => Some(Action::Pause),
                        SystemMediaTransportControlsButton::Stop => Some(Action::Stop),
                        SystemMediaTransportControlsButton::Next => Some(Action::Next),
                        SystemMediaTransportControlsButton::Previous => Some(Action::Previous),
                        _ => None,
                    };
                    if let Some(action) = action {
                        commands.enqueue(action);
                    }
                }
                Ok(())
            }))
            .map_err(|e| e.to_string())?;
        let commands = shared.clone();
        let seek_token =
            match controls.PlaybackPositionChangeRequested(&TypedEventHandler::<
                SystemMediaTransportControls,
                PlaybackPositionChangeRequestedEventArgs,
            >::new(move |_, args| {
                if let Some(args) = args.as_ref() {
                    let position = args.RequestedPlaybackPosition()?.Duration as f64 / 10_000_000.0;
                    commands.enqueue(Action::Seek(position));
                }
                Ok(())
            })) {
                Ok(token) => token,
                Err(error) => {
                    let _ = controls.RemoveButtonPressed(button_token);
                    return Err(error.to_string());
                }
            };
        shared.status(Status::Ready);
        Ok(Self {
            controls,
            button_token,
            seek_token,
            previous: RefCell::new(Snapshot::default()),
            _runtime: runtime,
        })
    }

    pub(super) fn publish(&self, current: &Snapshot) -> Result<(), String> {
        self.update(current).map_err(|e| e.to_string())?;
        *self.previous.borrow_mut() = current.clone();
        Ok(())
    }

    fn update(&self, s: &Snapshot) -> windows::core::Result<()> {
        let previous = self.previous.borrow();
        let controls = &self.controls;
        controls.SetIsEnabled(s.active)?;
        controls.SetIsPlayEnabled(s.active)?;
        controls.SetIsPauseEnabled(s.active)?;
        controls.SetIsStopEnabled(s.active)?;
        controls.SetIsNextEnabled(s.active && s.can_next)?;
        controls.SetIsPreviousEnabled(s.active && s.can_previous)?;
        controls.SetPlaybackStatus(match s.state {
            PlaybackState::Playing => MediaPlaybackStatus::Playing,
            PlaybackState::Paused => MediaPlaybackStatus::Paused,
            PlaybackState::Stopped => MediaPlaybackStatus::Stopped,
        })?;
        if (
            s.generation,
            s.active,
            &s.title,
            &s.artist,
            &s.album,
            s.video,
        ) != (
            previous.generation,
            previous.active,
            &previous.title,
            &previous.artist,
            &previous.album,
            previous.video,
        ) {
            let updater = controls.DisplayUpdater()?;
            updater.ClearAll()?;
            if s.active {
                updater.SetAppMediaId(&HSTRING::from(s.generation.to_string()))?;
                updater.SetType(if s.video {
                    MediaPlaybackType::Video
                } else {
                    MediaPlaybackType::Music
                })?;
                if s.video {
                    let properties = updater.VideoProperties()?;
                    properties.SetTitle(&HSTRING::from(&s.title))?;
                    properties.SetSubtitle(&HSTRING::from(&s.artist))?;
                } else {
                    let properties = updater.MusicProperties()?;
                    properties.SetTitle(&HSTRING::from(&s.title))?;
                    properties.SetArtist(&HSTRING::from(&s.artist))?;
                    properties.SetAlbumTitle(&HSTRING::from(&s.album))?;
                }
            }
            updater.Update()?;
        }
        let timeline = SystemMediaTransportControlsTimelineProperties::new()?;
        let zero = TimeSpan { Duration: 0 };
        let end = timespan(s.duration_seconds.unwrap_or(s.position_seconds));
        timeline.SetStartTime(zero)?;
        timeline.SetEndTime(end)?;
        timeline.SetMinSeekTime(if s.seekable {
            zero
        } else {
            timespan(s.position_seconds)
        })?;
        timeline.SetMaxSeekTime(if s.seekable {
            end
        } else {
            timespan(s.position_seconds)
        })?;
        timeline.SetPosition(timespan(s.position_seconds))?;
        controls.UpdateTimelineProperties(&timeline)?;
        Ok(())
    }
}

fn timespan(seconds: f64) -> TimeSpan {
    TimeSpan {
        Duration: (seconds * 10_000_000.0) as i64,
    }
}

impl Drop for Session {
    fn drop(&mut self) {
        let _ = self.controls.RemoveButtonPressed(self.button_token);
        let _ = self
            .controls
            .RemovePlaybackPositionChangeRequested(self.seek_token);
        let _ = self.controls.SetPlaybackStatus(MediaPlaybackStatus::Closed);
        let _ = self.controls.SetIsEnabled(false);
        if let Ok(updater) = self.controls.DisplayUpdater() {
            let _ = updater.ClearAll();
            let _ = updater.Update();
        }
    }
}
