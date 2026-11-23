use super::*;
use ferroui_media_video::{MediaPlayerOptions, MediaSource};

impl NativeMedia {
    pub(super) fn alarm(self: &Rc<Self>, active: bool, volume: u8) {
        if !active || volume == 0 {
            self.alarm_generation
                .set(self.alarm_generation.get().wrapping_add(1));
            if let Some(player) = self.alarm_player.borrow_mut().take() {
                player.clear_source();
            }
            return;
        }
        if let Some(player) = self.alarm_player.borrow().as_ref() {
            let mut options = player.options();
            options.volume = f64::from(volume.min(10)) / 10.0;
            if let Err(error) = player.set_options(options) {
                self.status(error.to_string());
            }
            return;
        }
        self.alarm_generation
            .set(self.alarm_generation.get().wrapping_add(1));
        let generation = self.alarm_generation.get();
        let weak = Rc::downgrade(self);
        drop(Dispatcher::current_dispatcher().spawn_local(async move {
            let Some(state) = weak.upgrade() else {
                return;
            };
            let result = async {
                let interop = state.interop().await?;
                if state.alarm_generation.get() != generation {
                    return Ok(());
                }
                let player = MediaPlayer::new(
                    providers::video(&*interop, state.budget.clone()).map_err(|e| e.to_string())?,
                );
                player
                    .set_options(MediaPlayerOptions {
                        autoplay: true,
                        looping: true,
                        volume: f64::from(volume.min(10)) / 10.0,
                        ..Default::default()
                    })
                    .map_err(|e| e.to_string())?;
                let pending = player.set_source(MediaSource::Resource {
                    name: "phone-alarm.wav".into(),
                    content_type: "audio/wav".into(),
                    encoded: tone().into(),
                });
                *state.alarm_player.borrow_mut() = Some(player);
                pending.await.map_err(|e| e.to_string())?;
                Ok::<_, String>(())
            }
            .await;
            if state.alarm_generation.get() == generation {
                if let Err(error) = result {
                    state.status(format!("Alarm audio: {error}"));
                }
            }
        }));
    }

    pub(super) fn poll_alarm(self: &Rc<Self>) {
        let player = self.alarm_player.borrow().clone();
        if let Some(player) = player {
            match player.poll() {
                Ok(Some(pending)) => {
                    let weak = Rc::downgrade(self);
                    let generation = self.alarm_generation.get();
                    drop(Dispatcher::current_dispatcher().spawn_local(async move {
                        if let Err(error) = pending.await {
                            if let Some(state) = weak
                                .upgrade()
                                .filter(|state| state.alarm_generation.get() == generation)
                            {
                                state.status(format!("Alarm audio: {error}"));
                            }
                        }
                    }));
                }
                Ok(None) => {}
                Err(error) => {
                    player.clear_source();
                    self.status(format!("Alarm audio: {error}"));
                }
            }
        }
    }
}

/// A bounded two-second PCM16 alert. The player owns user volume and looping.
fn tone() -> Vec<u8> {
    const RATE: u32 = 22_050;
    let samples = RATE * 2;
    let length = samples * 2;
    let mut data = Vec::with_capacity(length as usize + 44);
    data.extend_from_slice(b"RIFF");
    data.extend_from_slice(&(length + 36).to_le_bytes());
    data.extend_from_slice(b"WAVEfmt \x10\0\0\0\x01\0\x01\0");
    data.extend_from_slice(&RATE.to_le_bytes());
    data.extend_from_slice(&(RATE * 2).to_le_bytes());
    data.extend_from_slice(b"\x02\0\x10\0data");
    data.extend_from_slice(&length.to_le_bytes());
    for sample in 0..samples {
        let t = f64::from(sample) / f64::from(RATE);
        let local = t % 0.5;
        let envelope = if local < 0.3 {
            (local / 0.008).min(1.0) * ((0.3 - local) / 0.02).min(1.0)
        } else {
            0.0
        };
        let frequency = if (t / 0.5) as u32 % 2 == 0 {
            660.0
        } else {
            880.0
        };
        let value = ((t * frequency * std::f64::consts::TAU).sin() * envelope * 12_000.0) as i16;
        data.extend_from_slice(&value.to_le_bytes());
    }
    data
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn generated_alert_is_bounded_pcm_with_a_silent_gap() {
        let wav = tone();
        assert_eq!(&wav[..4], b"RIFF");
        assert_eq!(&wav[8..12], b"WAVE");
        assert_eq!(wav.len(), 44 + 22_050 * 2 * 2);
        assert!(wav[44 + 8_000 * 2..44 + 9_000 * 2]
            .iter()
            .all(|sample| *sample == 0));
        assert!(wav[44..1_000].iter().any(|sample| *sample != 0));
    }
}
