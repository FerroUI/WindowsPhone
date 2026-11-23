use super::*;
use ferroui_media_video::{MediaPlaybackState, MediaPlayerOptions, MediaSource};

impl NativeMedia {
    pub(super) fn play(self: &Rc<Self>, source: String) -> Result<(), String> {
        let vm = self.vm.upgrade().ok_or("The phone has closed.")?;
        if !source.starts_with("media:") {
            phone_host::http::validate_url(&source)?;
        }
        if self.os_session.borrow().is_none() {
            // A new playback request can retry a previously absent desktop
            // service; idle frame polling never loops on registration failures.
            self.os_attempted.set(false);
        }
        let autoplay = self.pending_play_intent.take().unwrap_or(true);
        self.desired_playing.set(autoplay);
        self.media_stopped.set(false);
        self.media_ended.set(false);
        self.media_generation
            .set(self.media_generation.get().wrapping_add(1));
        let generation = self.media_generation.get();
        let epoch = vm.document_epoch();
        // Resume the same player at its real native position.
        if *self.source.borrow() == source {
            if let Some(player) = self.player.borrow().clone() {
                if !autoplay {
                    player.pause().map_err(|error| error.to_string())?;
                    let playback = player.playback_session();
                    phone_ui::media_progress(
                        playback.position.as_secs_f64(),
                        playback
                            .duration
                            .map_or(0.0, |duration| duration.as_secs_f64()),
                    );
                    self.set_playing(false);
                    return Ok(());
                }
                self.playing
                    .set(player.playback_session().state == MediaPlaybackState::Playing);
                self.publish_media();
                // Record the provider intent synchronously. A later Pause must
                // supersede it even if the dispatcher has not polled this future.
                let pending = player.play();
                let state = Rc::downgrade(self);
                drop(Dispatcher::current_dispatcher().spawn_local(async move {
                    if let Err(error) = pending.await {
                        if let Some(state) = state.upgrade().filter(|state| {
                            state.media_generation.get() == generation
                                && vm.document_epoch() == epoch
                        }) {
                            state.status(error.to_string());
                        }
                    }
                }));
                return Ok(());
            }
        }
        self.os_generation
            .set(self.os_generation.get().wrapping_add(1).max(1));
        if let Some(old) = self.player.borrow_mut().take() {
            old.clear_source();
        }
        *self.source.borrow_mut() = source.clone();
        *self.title.borrow_mut() = vm
            .data()
            .tracks
            .iter()
            .find(|track| track.source == source)
            .map(|track| track.title.clone())
            .unwrap_or_default();
        self.playing.set(false);
        self.publish_media();
        self.refresh_video_surfaces();
        let state = Rc::downgrade(self);
        drop(Dispatcher::current_dispatcher().spawn_local(async move {
            let lookup = source.clone();
            let result = if lookup.starts_with("media:") {
                crate::background::run(move || crate::media_files::open_media(&lookup))
                    .await
                    .map(|(path, title)| (MediaSource::File(path), title))
            } else {
                phone_host::http::validate_url(&lookup)
                    .map(|url| (MediaSource::Url(url), String::new()))
            };
            let Some(state) = state
                .upgrade()
                .filter(|s| s.media_generation.get() == generation)
            else {
                return;
            };
            if vm.document_epoch() != epoch {
                return;
            }
            let result = async {
                let (native_source, title) = result?;
                let interop = state.interop().await?;
                if state.media_generation.get() != generation || vm.document_epoch() != epoch {
                    return Ok(());
                }
                let provider =
                    providers::video(&*interop, state.budget.clone()).map_err(|e| e.to_string())?;
                let player = MediaPlayer::new(provider);
                let data = vm.data();
                let options = (data.settings.volume, data.settings.media_muted);
                drop(data);
                player
                    .set_options(MediaPlayerOptions {
                        autoplay,
                        volume: f64::from(options.0) / 30.0,
                        muted: options.1,
                        ..Default::default()
                    })
                    .map_err(|e| e.to_string())?;
                let pending = player.set_source(native_source);
                if let Some(old) = state.player.borrow_mut().replace(player.clone()) {
                    old.clear_source();
                }
                *state.source.borrow_mut() = source.clone();
                if state.title.borrow().is_empty() {
                    *state.title.borrow_mut() = title;
                }
                state.publish_media();
                state.audio_options.set(Some(options));
                state.refresh_video_surfaces();
                pending.await.map_err(|e| e.to_string())?;
                Ok::<_, String>(())
            }
            .await;
            if state.media_generation.get() == generation && vm.document_epoch() == epoch {
                if let Err(error) = result {
                    state.desired_playing.set(false);
                    state.set_playing(false);
                    state.status(format!("Playback: {error}"));
                }
            }
        }));
        Ok(())
    }

    pub(super) fn control(self: &Rc<Self>, action: &str) -> Result<(), String> {
        if matches!(action, "forward" | "rewind") {
            let player = self
                .player
                .borrow()
                .clone()
                .ok_or("Choose a media item first.")?;
            let offset = if action == "forward" { 10.0 } else { -10.0 };
            return self.seek(
                (player.playback_session().position.as_secs_f64() + offset).max(0.0),
                false,
            );
        }
        if let Some(position) = action.strip_prefix("seek:") {
            return self.seek(
                position
                    .parse()
                    .map_err(|_| "Enter a valid playback position.")?,
                false,
            );
        }
        if action == "restart" {
            let resume = self.pending_play_intent.take().unwrap_or(true);
            if self
                .player
                .borrow()
                .as_ref()
                .is_some_and(|player| !player.playback_session().seekable)
            {
                let source = self.source.borrow().clone();
                if let Some(player) = self.player.borrow_mut().take() {
                    player.clear_source();
                }
                self.pending_play_intent.set(Some(resume));
                return self.play(source);
            }
            return self.seek(0.0, resume);
        }
        if !matches!(action, "external" | "pause" | "stop" | "play") {
            return Err("This playback command is not supported.".into());
        }
        if action == "play" {
            let source = self.source.borrow().clone();
            if source.is_empty() {
                return Err("Choose a media item first.".into());
            }
            return self.play(source);
        }
        self.media_generation
            .set(self.media_generation.get().wrapping_add(1));
        let generation = self.media_generation.get();
        self.desired_playing.set(false);
        self.media_stopped.set(action != "pause");
        if action == "external" {
            let source = self.source.borrow().clone();
            if source.is_empty() {
                return Err("Choose a media item first.".into());
            }
            if let Some(player) = self.player.borrow_mut().take() {
                player.clear_source();
            }
            self.set_playing(false);
            self.clear_system_media();
            self.refresh_video_surfaces();
            let state = Rc::downgrade(self);
            drop(Dispatcher::current_dispatcher().spawn_local(async move {
                let result = crate::background::run(move || {
                    if source.starts_with("media:") {
                        let (path, _) = crate::media_files::open_media(&source)?;
                        phone_host::native::open_external(path.as_os_str())
                    } else {
                        let url = phone_host::http::validate_url(&source)?;
                        phone_host::native::open_external(std::ffi::OsStr::new(&url))
                    }
                }).await;
                if let Some(state) = state.upgrade().filter(|state| state.media_generation.get() == generation) {
                    state.status(match result { Ok(()) => "Requested playback in your device's media app. Use its playback controls.".into(), Err(error) => error });
                }
            }));
            return Ok(());
        }
        let player = self.player.borrow().clone();
        let Some(player) = player else {
            return Ok(());
        };
        match action {
            "pause" => {
                player.pause().map_err(|e| e.to_string())?;
                let playback = player.playback_session();
                phone_ui::media_progress(
                    playback.position.as_secs_f64(),
                    playback
                        .duration
                        .map_or(0.0, |duration| duration.as_secs_f64()),
                );
                self.set_playing(false);
            }
            "stop" => {
                let playback = player.playback_session();
                phone_ui::media_progress(
                    playback.position.as_secs_f64(),
                    playback
                        .duration
                        .map_or(0.0, |duration| duration.as_secs_f64()),
                );
                let pending = player.stop();
                self.set_playing(false);
                let state = Rc::downgrade(self);
                drop(Dispatcher::current_dispatcher().spawn_local(async move {
                    if let Err(error) = pending.await {
                        if let Some(state) = state
                            .upgrade()
                            .filter(|state| state.media_generation.get() == generation)
                        {
                            state.status(error.to_string());
                        }
                    }
                }));
            }
            _ => unreachable!("validated native playback command"),
        }
        Ok(())
    }

    pub(super) fn seek(self: &Rc<Self>, position: f64, resume: bool) -> Result<(), String> {
        if !position.is_finite() || !(0.0..=31_536_000.0).contains(&position) {
            return Err("Enter a valid playback position.".into());
        }
        let player = self
            .player
            .borrow()
            .clone()
            .ok_or("Choose a media item first.")?;
        let playback = player.playback_session();
        if !playback.seekable {
            return Err("This media does not support seeking.".into());
        }
        let position = playback
            .duration
            .map_or(position, |duration| position.min(duration.as_secs_f64()));
        let pending = player.seek(Duration::from_secs_f64(position));
        let generation = self.media_generation.get();
        self.media_seek_generation
            .set(self.media_seek_generation.get().wrapping_add(1));
        let seek_generation = self.media_seek_generation.get();
        let weak = Rc::downgrade(self);
        if resume {
            self.desired_playing.set(true);
            self.media_stopped.set(false);
        }
        // Do not dispatch the same Ended state again while a restart/seek is
        // pending. The next observed non-ended native state re-arms the event.
        self.media_ended.set(true);
        drop(Dispatcher::current_dispatcher().spawn_local(async move {
            let result = pending.await;
            let Some(state) = weak.upgrade().filter(|s| {
                s.media_generation.get() == generation
                    && s.media_seek_generation.get() == seek_generation
            }) else {
                return;
            };
            match result {
                Ok(achieved) => {
                    state
                        .os_seek_serial
                        .set(state.os_seek_serial.get().wrapping_add(1));
                    phone_ui::media_progress(
                        achieved.as_secs_f64(),
                        player
                            .playback_session()
                            .duration
                            .map_or(0.0, |duration| duration.as_secs_f64()),
                    );
                    if resume && state.desired_playing.get() {
                        if let Err(error) = player.play().await {
                            if state.media_generation.get() == generation
                                && state.media_seek_generation.get() == seek_generation
                            {
                                state.status(error.to_string());
                            }
                        }
                    }
                }
                Err(error) => state.status(error.to_string()),
            }
        }));
        Ok(())
    }

    pub(super) fn set_playing(&self, value: bool) {
        if self.playing.replace(value) != value {
            self.publish_media();
        }
    }

    fn publish_media(&self) {
        phone_ui::show_media(
            self.title.borrow().clone(),
            self.source.borrow().clone(),
            self.playing.get(),
        );
    }

    pub(super) fn poll_playback(self: &Rc<Self>, vm: &PhoneViewModel) {
        let player = self.player.borrow().clone();
        let Some(player) = player else {
            return;
        };
        let data = vm.data();
        let options = (data.settings.volume, data.settings.media_muted);
        drop(data);
        if self.audio_options.replace(Some(options)) != Some(options) {
            let mut setting = player.options();
            setting.volume = f64::from(options.0) / 30.0;
            setting.muted = options.1;
            if let Err(error) = player.set_options(setting) {
                self.status(error.to_string());
            }
        }
        match player.poll() {
            Ok(Some(pending)) => {
                let weak = Rc::downgrade(self);
                let generation = self.media_generation.get();
                drop(Dispatcher::current_dispatcher().spawn_local(async move {
                    if let Err(error) = pending.await {
                        if let Some(state) = weak
                            .upgrade()
                            .filter(|state| state.media_generation.get() == generation)
                        {
                            state.set_playing(false);
                            state.status(format!("Playback: {error}"));
                        }
                    }
                }));
            }
            Ok(None) => {}
            Err(error) => {
                self.status(error.to_string());
                player.clear_source();
            }
        }
        let playback = player.playback_session();
        let generation = self.media_generation.get();
        let playing = playback.state == MediaPlaybackState::Playing;
        let ended = playback.state == MediaPlaybackState::Ended;
        let newly_ended = ended && !self.media_ended.get();
        if self.media_last_progress.get().elapsed() >= Duration::from_millis(250)
            || playing != self.playing.get()
            || newly_ended
        {
            self.media_last_progress.set(Instant::now());
            // Pause and end events persist listening checkpoints. Publish the
            // actual final position before either event can flush that state.
            phone_ui::media_progress(
                playback.position.as_secs_f64(),
                playback
                    .duration
                    .map_or(0.0, |duration| duration.as_secs_f64()),
            );
        }
        if self.media_generation.get() == generation {
            if !ended {
                self.media_ended.set(false);
            } else if !self.media_ended.replace(true) && self.desired_playing.get() {
                phone_ui::media_command("ended");
            }
        }
        // Queue advancement can synchronously select another source. Do not
        // overwrite its host state with the previous player's final observation.
        if self.media_generation.get() == generation {
            self.set_playing(playing);
        }
        Self::present(&self.video_views);
    }
}
