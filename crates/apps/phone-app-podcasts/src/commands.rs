use crate::{
    model::{Episode, Saved, Show},
    state::State,
};
use ferroui_base::threading::Dispatcher;
use phone_sdk::prelude::*;
use phone_sdk::{
    media::{self, PlaybackEvent, RepeatMode},
    FilePickerOptions,
};
use phone_state::HostRequest;
#[cfg(test)]
mod tests;
pub fn load(cx: &AppContext) -> Result<Saved, String> {
    let saved = cx.storage().read::<Saved>()?;
    saved.validate()?;
    Ok(saved)
}
pub fn change(
    cx: &AppContext,
    apply: impl FnOnce(&mut Saved) -> Result<(), String>,
) -> Result<(), String> {
    let mut saved = load(cx)?;
    apply(&mut saved)?;
    saved.validate()?;
    cx.storage().write(&saved)
}
pub fn report(cx: &AppContext, result: Result<(), String>) {
    if let Err(error) = result {
        cx.vm.set_status(error);
    }
}

struct FeedOrigin {
    route: Route,
    episode: Option<String>,
}
impl FeedOrigin {
    fn capture(cx: &AppContext) -> Self {
        Self {
            route: cx.vm.route(),
            episode: cx.session.state::<State>().episode.get_untracked(),
        }
    }
    fn is_current(&self, cx: &AppContext) -> bool {
        matches!(&self.route, Route::Extension { app, .. } if app == crate::APP_ID)
            && cx.vm.route() == self.route
            && cx.session.state::<State>().episode.get_untracked() == self.episode
    }
}

pub fn open(cx: &AppContext, id: u32) {
    cx.session.state::<State>().episode.set(None);
    cx.navigate(&format!("show/{id}"));
}
pub fn subscribe(cx: AppContext) {
    let entered = cx.clone();
    cx.session.form_with_action(
        "subscribe to a podcast",
        vec![("HTTPS feed address", String::new())],
        "subscribe",
        Rc::new(move |fields| {
            let value = fields.first().cloned().unwrap_or_default();
            match phone_host::http::validate_url(&value) {
                Ok(url) => {
                    entered.session.dialog.set(None);
                    fetch(entered.clone(), url, None);
                }
                Err(error) => entered.session.form_error.set(error),
            }
        }),
    );
}
pub fn fetch(cx: AppContext, url: String, id: Option<u32>) {
    let origin = FeedOrigin::capture(&cx);
    let state = cx.session.state::<State>();
    let generation = state.generation.get().wrapping_add(1);
    state.generation.set(generation);
    state.busy.set(true);
    let epoch = cx.vm.document_epoch();
    let task = cx.get(&url, phone_host::http::MAX_RESPONSE_BYTES);
    drop(Dispatcher::current_dispatcher().spawn_local(async move {
        let result = task
            .await
            .and_then(|bytes| crate::feed::parse(&bytes, Some(url)));
        if cx.vm.document_epoch() != epoch || state.generation.get() != generation {
            return;
        }
        state.busy.set(false);
        match result {
            Ok(mut show) => {
                show.id = id.unwrap_or(0);
                save_show(&cx, show, &origin);
            }
            Err(error) => cx.vm.set_status(error),
        }
    }));
}
pub fn import(cx: AppContext) {
    let origin = FeedOrigin::capture(&cx);
    let task = cx.choose_file(FilePickerOptions {
        extensions: vec![".xml".into(), ".rss".into(), ".atom".into()],
        max_bytes: phone_host::http::MAX_RESPONSE_BYTES,
    });
    let state = cx.session.state::<State>();
    let generation = state.generation.get().wrapping_add(1);
    state.generation.set(generation);
    // The picker supersedes a pending fetch. Its stale completion deliberately
    // does nothing, so clear that fetch's loading indicator here.
    state.busy.set(false);
    let epoch = cx.vm.document_epoch();
    drop(Dispatcher::current_dispatcher().spawn_local(async move {
        let result = task.await;
        if cx.vm.document_epoch() != epoch || state.generation.get() != generation {
            return;
        }
        match result {
            Ok(Some(file)) => match crate::feed::parse(&file.bytes, None) {
                Ok(show) => save_show(&cx, show, &origin),
                Err(error) => cx.vm.set_status(error),
            },
            Ok(None) => {}
            Err(error) => cx.vm.set_status(error),
        }
    }));
}
fn save_show(cx: &AppContext, show: Show, origin: &FeedOrigin) {
    let mut id = 0;
    let count = show.episodes.len();
    let result = change(cx, |saved| {
        id = saved.upsert(show)?;
        Ok(())
    });
    match result {
        Ok(()) => {
            cx.vm
                .set_status(format!("Saved {count} recent playable episodes."));
            if origin.is_current(cx) {
                open(cx, id);
            }
        }
        Err(error) => cx.vm.set_status(error),
    }
}
pub fn remove(cx: AppContext, id: u32, title: &str) {
    let confirmed = cx.clone();
    cx.session.confirm(
        &format!(
            "Unsubscribe from {title}? Episodes added to your media library will remain there."
        ),
        Rc::new(move || {
            let result = change(&confirmed, |saved| {
                saved.shows.retain(|show| show.id != id);
                Ok(())
            });
            match result {
                Ok(()) => {
                    confirmed.session.dialog.set(None);
                    confirmed.navigate("");
                }
                Err(error) => confirmed.session.form_error.set(error),
            }
        }),
    );
}
pub fn play(cx: &AppContext, show: &Show, episode: &Episode, resume: bool) {
    let already_current = cx.session.media_source.get_untracked() == episode.source;
    let tracks = show
        .episodes
        .iter()
        .filter(|item| item.video == episode.video)
        .map(|item| phone_domain::MediaTrack {
            id: 0,
            title: item.title.clone(),
            artist: show.title.clone(),
            source: item.source.clone(),
            duration_seconds: item.duration.into(),
            media_kind: if item.video { "video" } else { "audio" }.into(),
        })
        .collect::<Vec<_>>();
    let sources = tracks
        .iter()
        .map(|track| track.source.clone())
        .collect::<Vec<_>>();
    let result = cx.vm.import_media_tracks(tracks).and_then(|_| {
        media::start(
            &cx.vm,
            &cx.session,
            sources,
            &episode.source,
            RepeatMode::Off,
            false,
        )
    });
    match result {
        Ok(()) => {
            if !resume && already_current {
                cx.vm.request(HostRequest::MediaControl {
                    action: "restart".into(),
                });
            }
            cx.session
                .state::<State>()
                .pending_seek
                .replace(if resume && episode.position > 0 {
                    Some((episode.source.clone(), episode.position))
                } else {
                    None
                });
        }
        Err(error) => cx.vm.set_status(error),
    }
}
pub fn media_event(cx: AppContext, event: &PlaybackEvent) {
    let observation = event.snapshot();
    if !observation.is_current(&cx.vm, &cx.session) {
        return;
    }
    let state = cx.session.state::<State>();
    let source = &observation.source;
    let position = observation.position;
    let duration = observation.duration;
    let ended = matches!(event, PlaybackEvent::Ended(_));
    let checkpoint = !matches!(event, PlaybackEvent::Progress(_));
    let seek = state
        .pending_seek
        .borrow()
        .as_ref()
        .filter(|(target, _)| target == source)
        .map(|(_, seconds)| *seconds);
    if let Some(seconds) = seek {
        if ended {
            state.pending_seek.borrow_mut().take();
        } else if !checkpoint && duration > 0.0 {
            state.pending_seek.borrow_mut().take();
            if position < 2.0 && f64::from(seconds) < duration - 1.0 {
                cx.vm.request(HostRequest::MediaControl {
                    action: format!("seek:{seconds}"),
                });
                return;
            }
        } else if position < 2.0 {
            // A pause before the host has loaded metadata must not overwrite
            // the saved resume point with its temporary zero position.
            return;
        } else {
            state.pending_seek.borrow_mut().take();
        }
    }
    let now = cx.vm.now();
    if !checkpoint && now.saturating_sub(state.last_save.get()) < 15 {
        return;
    }
    state.last_save.set(now);
    let Ok(mut saved) = load(&cx) else {
        return;
    };
    let seconds = position.clamp(0.0, 31_536_000.0) as u32;
    let mut changed = false;
    for episode in saved
        .shows
        .iter_mut()
        .flat_map(|show| &mut show.episodes)
        .filter(|episode| &episode.source == source)
    {
        if ended {
            changed |= !episode.played || episode.position != 0;
            episode.played = true;
            episode.position = 0;
        } else if episode.position != seconds
            && (checkpoint || episode.position.abs_diff(seconds) >= 5)
        {
            episode.position = seconds;
            changed = true;
        }
    }
    if changed {
        report(&cx, cx.storage().write(&saved));
    }
}
pub fn mark(cx: &AppContext, source: &str, played: bool) {
    report(
        cx,
        change(cx, |saved| {
            for episode in saved
                .shows
                .iter_mut()
                .flat_map(|show| &mut show.episodes)
                .filter(|episode| episode.source == source)
            {
                episode.played = played;
                if played {
                    episode.position = 0;
                }
            }
            Ok(())
        }),
    );
}
