use super::*;
use ferroui_media_video::{MediaPlaybackState, MediaPlayerOptions, MediaSource};

impl NativeMedia {
    pub(super) fn play(self: &Rc<Self>, source: String) -> Result<(), String> {
        let vm = self.vm.upgrade().ok_or("The phone has closed.")?;
        // Resume the same player at its real native position.
        if *self.source.borrow() == source {
            if let Some(player) = self.player.borrow().clone() {
                self.playing
                    .set(player.playback_session().state == MediaPlaybackState::Playing);
                self.publish_media();
                let state = Rc::downgrade(self);
                drop(Dispatcher::current_dispatcher().spawn_local(async move {
                    if let Err(error) = player.play().await {
                        if let Some(state) = state.upgrade() {
                            state.status(error.to_string());
                        }
                    }
                }));
                return Ok(());
            }
        }
        self.media_generation
            .set(self.media_generation.get().wrapping_add(1));
        let generation = self.media_generation.get();
        let epoch = vm.document_epoch();
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
            let result =
                crate::background::run(move || crate::media_files::open_media(&lookup)).await;
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
                let (path, title) = result?;
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
                        autoplay: true,
                        volume: f64::from(options.0) / 30.0,
                        muted: options.1,
                        ..Default::default()
                    })
                    .map_err(|e| e.to_string())?;
                let pending = player.set_source(MediaSource::File(path));
                if let Some(old) = state.player.borrow_mut().replace(player.clone()) {
                    old.clear_source();
                }
                *state.source.borrow_mut() = source.clone();
                *state.title.borrow_mut() = title;
                state.publish_media();
                state.audio_options.set(Some(options));
                state.refresh_video_surfaces();
                pending.await.map_err(|e| e.to_string())?;
                Ok::<_, String>(())
            }
            .await;
            if state.media_generation.get() == generation && vm.document_epoch() == epoch {
                if let Err(error) = result {
                    state.set_playing(false);
                    state.status(format!("Playback: {error}"));
                }
            }
        }));
        Ok(())
    }

    pub(super) fn control(self: &Rc<Self>, action: &str) -> Result<(), String> {
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
        if action == "external" {
            let source = self.source.borrow().clone();
            if source.is_empty() {
                return Err("Choose a media item first.".into());
            }
            if let Some(player) = self.player.borrow_mut().take() {
                player.clear_source();
            }
            self.set_playing(false);
            self.refresh_video_surfaces();
            let state = Rc::downgrade(self);
            drop(Dispatcher::current_dispatcher().spawn_local(async move {
                let result = crate::background::run(move || {
                    let (path, _) = crate::media_files::open_media(&source)?;
                    phone_host::native::open_external(path.as_os_str())
                }).await;
                if let Some(state) = state.upgrade() {
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
                self.set_playing(false);
            }
            "stop" => {
                let pending = player.stop();
                self.set_playing(false);
                let state = Rc::downgrade(self);
                drop(Dispatcher::current_dispatcher().spawn_local(async move {
                    if let Err(error) = pending.await {
                        if let Some(state) = state.upgrade() {
                            state.status(error.to_string());
                        }
                    }
                }));
            }
            _ => unreachable!("validated native playback command"),
        }
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
        self.set_playing(player.playback_session().state == MediaPlaybackState::Playing);
        Self::present(&self.video_views);
    }
}
