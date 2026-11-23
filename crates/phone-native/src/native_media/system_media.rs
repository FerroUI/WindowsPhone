use super::*;
use ferroui_media_video::MediaPlaybackState;
use phone_host::native::media_session::{Action, MediaSession, PlaybackState, Snapshot, Status};

impl NativeMedia {
    pub(super) fn clear_system_media(&self) {
        self.os_generation
            .set(self.os_generation.get().wrapping_add(1).max(1));
        self.os_session.borrow_mut().take();
        self.os_attempted.set(false);
        self.media_ended.set(false);
    }

    pub(super) fn poll_system_media(self: &Rc<Self>, vm: &PhoneViewModel) {
        let active_player = self.player.borrow().is_some();
        if active_player && !self.os_attempted.replace(true) {
            let handle = self
                .window
                .upgrade()
                .and_then(|view| ferroui_controls::TopLevel::get_top_level(Some(&view)))
                .and_then(|top| crate::notifications::window_handle(&top));
            match self
                .platform
                .media_backend()
                .and_then(|provided| match provided {
                    Some(backend) => Ok(backend),
                    None => MediaSession::new(handle).map(|session| {
                        Box::new(session)
                            as Box<dyn phone_host::native::media_backend::MediaBackend>
                    }),
                }) {
                Ok(session) => {
                    *self.os_session.borrow_mut() = Some(session);
                }
                Err(error) => {
                    let mut environment = vm.environment();
                    environment.media_session_supported = Some(false);
                    vm.set_environment(environment);
                    eprintln!("Native media controls unavailable: {error}");
                }
            }
        }
        let status = self
            .os_session
            .borrow()
            .as_ref()
            .map(|session| session.status());
        if let Some(status) = status {
            let available = match &status {
                Status::Pending => None,
                Status::Ready => Some(true),
                Status::Unavailable(_) => Some(false),
            };
            let mut environment = vm.environment();
            if environment.media_session_supported != available {
                environment.media_session_supported = available;
                vm.set_environment(environment);
            }
            if let Status::Unavailable(error) = status {
                eprintln!("Native media controls unavailable: {error}");
                self.os_session.borrow_mut().take();
                return;
            }
        }
        if self.os_session.borrow().is_none() {
            return;
        }
        let commands = self
            .os_session
            .borrow()
            .as_ref()
            .map(|session| session.take_commands())
            .unwrap_or_default();
        for command in commands {
            if command.generation != self.os_generation.get() {
                continue;
            }
            let result = match command.action {
                Action::Play => self.control("play"),
                Action::Pause => self.control("pause"),
                Action::Toggle => self.control(if self.desired_playing.get() {
                    "pause"
                } else {
                    "play"
                }),
                Action::Stop => self.control("stop"),
                Action::Next | Action::Previous => {
                    // MPRIS Next/Previous preserve a paused queue. The UI command
                    // synchronously requests the next source; its prepare operation
                    // captures this intent before the override is cleared.
                    self.pending_play_intent
                        .set(Some(self.desired_playing.get()));
                    phone_ui::media_command(if command.action == Action::Next {
                        "next"
                    } else {
                        "previous"
                    });
                    self.pending_play_intent.set(None);
                    Ok(())
                }
                Action::Seek(position) => self.seek(position, false),
                Action::SeekBy(offset) => {
                    let playback = self
                        .player
                        .borrow()
                        .as_ref()
                        .map(|player| player.playback_session());
                    if let Some(playback) = playback {
                        let position = (playback.position.as_secs_f64() + offset).max(0.0);
                        if playback
                            .duration
                            .is_some_and(|duration| position > duration.as_secs_f64())
                        {
                            self.pending_play_intent
                                .set(Some(self.desired_playing.get()));
                            phone_ui::media_command("next");
                            self.pending_play_intent.set(None);
                            Ok(())
                        } else {
                            self.seek(position, false)
                        }
                    } else {
                        Ok(())
                    }
                }
                Action::Volume(volume) => {
                    let level = (volume * 30.0).round() as u8;
                    let mut settings = vm.data().settings.clone();
                    settings.volume = level;
                    settings.media_muted = false;
                    vm.set_settings(settings);
                    Ok(())
                }
                Action::Raise => {
                    if let Some(window) = self
                        .window
                        .upgrade()
                        .and_then(|view| ferroui_controls::TopLevel::get_top_level(Some(&view)))
                        .and_then(|top| top.cast::<ferroui_controls::Window>())
                    {
                        if window.window_state() == ferroui_controls::WindowState::Minimized {
                            window.set_window_state(ferroui_controls::WindowState::Normal);
                        }
                        window.activate();
                    }
                    Ok(())
                }
            };
            if let Err(error) = result {
                self.status(error);
            }
        }
        if self.os_last_sync.get().elapsed() < Duration::from_millis(250) {
            return;
        }
        self.os_last_sync.set(Instant::now());
        let source = self.source.borrow().clone();
        let player = self
            .player
            .borrow()
            .as_ref()
            .map(|player| player.playback_session());
        let data = vm.data();
        let track = data.tracks.iter().find(|track| track.source == source);
        let (can_next, can_previous) = phone_ui::media_capabilities();
        let snapshot = Snapshot {
            generation: self.os_generation.get(),
            active: !source.is_empty()
                && player.as_ref().is_some_and(|p| {
                    !matches!(
                        p.state,
                        MediaPlaybackState::Closed | MediaPlaybackState::Failed
                    )
                }),
            title: track
                .map(|track| track.title.clone())
                .unwrap_or_else(|| self.title.borrow().clone()),
            artist: track.map(|track| track.artist.clone()).unwrap_or_default(),
            video: track.is_some_and(|track| track.media_kind == "video"),
            state: player
                .as_ref()
                .map(|p| match p.state {
                    MediaPlaybackState::Playing => PlaybackState::Playing,
                    MediaPlaybackState::Paused if !self.media_stopped.get() => {
                        PlaybackState::Paused
                    }
                    _ => PlaybackState::Stopped,
                })
                .unwrap_or_default(),
            position_seconds: player.as_ref().map_or(0.0, |p| p.position.as_secs_f64()),
            duration_seconds: player
                .as_ref()
                .and_then(|p| p.duration.map(|duration| duration.as_secs_f64())),
            seekable: player.as_ref().is_some_and(|p| p.seekable),
            can_next,
            can_previous,
            volume: if data.settings.media_muted {
                0.0
            } else {
                f64::from(data.settings.volume) / 30.0
            },
            seek_serial: self.os_seek_serial.get(),
            ..Default::default()
        };
        drop(data);
        if let Some(session) = self.os_session.borrow().as_ref() {
            if let Err(error) = session.publish(&snapshot) {
                eprintln!("Native media update: {error}");
            }
        }
    }
}
