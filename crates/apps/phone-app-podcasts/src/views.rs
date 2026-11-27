use crate::{
    commands,
    model::{Episode, Show},
    state::State,
};
use phone_sdk::prelude::*;
use phone_state::HostRequest;
pub fn view(cx: AppContext) -> Element {
    let saved = match commands::load(&cx) {
        Ok(saved) => saved,
        Err(error) => {
            return page(
                "podcasts",
                empty("Podcasts unavailable", &error),
                Element::Empty,
            );
        }
    };
    let state = cx.session.state::<State>();
    let show_id = match cx.vm.route() {
        Route::Extension { path, .. } => path
            .strip_prefix("show/")
            .and_then(|id| id.parse::<u32>().ok()),
        _ => None,
    };
    let selected = show_id.and_then(|id| saved.shows.iter().find(|show| show.id == id));
    if let Some(show) = selected {
        if let Some(episode) = state.episode.get().and_then(|source| {
            show.episodes
                .iter()
                .find(|episode| episode.source == source)
        }) {
            return details(cx, show, episode);
        }
        return episodes(cx, show);
    }
    let mut body = stack_panel()
        .margin(20.0)
        .spacing(16.0)
        .child(text("collection", 42.0))
        .child(
            phone_controls::activity_progress("Loading podcast")
                .key("podcast-activity")
                .name("PodcastProgress")
                .is_indeterminate(state.busy),
        );
    if state.busy.get() {
        body = body.child(muted("loading podcast…", 18.0));
    }
    if saved.shows.is_empty() {
        body=body.child(empty("your podcasts","Subscribe using a public HTTPS RSS or Atom feed, or import a saved feed file. Playable episodes join your shared media library."));
    }
    for show in saved.shows {
        let open = cx.clone();
        let id = show.id;
        let unplayed = show
            .episodes
            .iter()
            .filter(|episode| !episode.played)
            .count();
        body = body.child(row(
            &show.title,
            &format!("{unplayed} unplayed · {} episodes", show.episodes.len()),
            move || commands::open(&open, id),
        ));
    }
    let add = cx.clone();
    let import = cx.clone();
    let pin = cx.clone();
    page(
        "podcasts",
        scroll(body.into()),
        app_bar(
            vec![
                action(Icon::Plus, "subscribe", move || {
                    commands::subscribe(add.clone())
                }),
                action(Icon::Download, "import feed", move || {
                    commands::import(import.clone())
                }),
                action(Icon::Pin, "pin to start", move || {
                    commands::report(&pin, pin.pin("", "Podcasts"))
                }),
            ],
            false,
            vec![],
        ),
    )
}
fn episodes(cx: AppContext, show: &Show) -> Element {
    let state = cx.session.state::<State>();
    let query = state.query.get().to_lowercase();
    let mut body = stack_panel()
        .margin(20.0)
        .spacing(12.0)
        .child(text(&show.title, 32.0))
        .child(
            phone_controls::activity_progress("Refreshing podcast")
                .key("podcast-activity")
                .name("PodcastProgress")
                .is_indeterminate(state.busy),
        );
    if !show.description.is_empty() {
        body = body.child(muted(&show.description, 18.0));
    }
    body = body.child(
        input("Search episodes")
            .name("PodcastSearch")
            .text(state.query.two_way()),
    );
    body = body.child(muted(
        "Most recent playable episodes · refresh to check for new episodes",
        13.0,
    ));
    for episode in show.episodes.iter().filter(|episode| {
        format!("{} {}", episode.title, episode.description)
            .to_lowercase()
            .contains(&query)
    }) {
        let state = state.clone();
        let source = episode.source.clone();
        body = body.child(row(
            &episode.title,
            &format!(
                "{}{}{}",
                if episode.played { "played · " } else { "" },
                date(episode.published),
                if episode.duration > 0 {
                    format!(" · {}", time(episode.duration))
                } else {
                    String::new()
                }
            ),
            move || state.episode.set(Some(source.clone())),
        ));
    }
    let mut actions = Vec::new();
    if let Some(url) = &show.feed {
        let refresh = cx.clone();
        let url = url.clone();
        let id = show.id;
        actions.push(action(Icon::Refresh, "refresh", move || {
            commands::fetch(refresh.clone(), url.clone(), Some(id))
        }));
    }
    let pin = cx.clone();
    let path = format!("show/{}", show.id);
    let title = show.title.chars().take(80).collect::<String>();
    actions.push(action(Icon::Pin, "pin podcast", move || {
        commands::report(&pin, pin.pin(&path, &title))
    }));
    let remove = cx.clone();
    let id = show.id;
    let title = show.title.clone();
    actions.push(action(Icon::Delete, "unsubscribe", move || {
        commands::remove(remove.clone(), id, &title)
    }));
    page(
        "podcasts",
        scroll(body.into()),
        app_bar(actions, false, vec![]),
    )
}
fn details(cx: AppContext, show: &Show, episode: &Episode) -> Element {
    let active = cx.session.media_source.get() == episode.source;
    let playing = active && cx.session.media_playing.get();
    let mut body = stack_panel()
        .margin(20.0)
        .spacing(16.0)
        .child(muted(&show.title, 21.0))
        .child(text(&episode.title, 32.0))
        .child(muted(
            &format!("{} · {}", date(episode.published), time(episode.duration)),
            16.0,
        ));
    if episode.video && active {
        body=body.child(rsx!{Border{name:"MediaViewport",height:200.0,background:"Black",{phone_sdk::host_surface(phone_sdk::HostSurface::Video)}}});
    }
    body = body.child(text(&episode.description, 19.0));
    if active {
        let progress = cx.session.state::<phone_sdk::media::PlaybackSession>();
        body = body.child(text(
            &format!(
                "{} / {}",
                time(progress.position.get() as u32),
                time(progress.duration.get() as u32)
            ),
            23.0,
        ));
    } else if episode.position > 0 {
        body = body.child(muted(
            &format!("saved position {}", time(episode.position)),
            17.0,
        ));
    }
    let start = cx.clone();
    let from_start = episode.clone();
    let show_copy = show.clone();
    body = body.child(flat("play from beginning", move || {
        commands::play(&start, &show_copy, &from_start, false)
    }));
    let mark = cx.clone();
    let source = episode.source.clone();
    let played = episode.played;
    body = body.child(flat(
        if played {
            "mark unplayed"
        } else {
            "mark played"
        },
        move || commands::mark(&mark, &source, !played),
    ));
    let play = cx.clone();
    let play_episode = episode.clone();
    let play_show = show.clone();
    let rewind = cx.clone();
    let forward = cx.clone();
    let rewind_source = episode.source.clone();
    let forward_source = episode.source.clone();
    let share = cx.clone();
    let share_episode = episode.clone();
    page(
        "podcasts",
        scroll(body.into()),
        app_bar(
            vec![
                action(
                    if playing { Icon::Pause } else { Icon::Play },
                    if playing {
                        "pause"
                    } else if episode.position > 0 && !active {
                        "resume"
                    } else {
                        "play"
                    },
                    move || {
                        if active {
                            play.vm.request(HostRequest::MediaControl {
                                action: if playing { "pause" } else { "play" }.into(),
                            });
                        } else {
                            commands::play(&play, &play_show, &play_episode, true);
                        }
                    },
                ),
                action(Icon::Previous, "rewind", move || {
                    if rewind.session.media_source.get_untracked() != rewind_source {
                        rewind
                            .vm
                            .set_status("Play this episode before seeking.".into());
                        return;
                    }
                    rewind.vm.request(HostRequest::MediaControl {
                        action: "rewind".into(),
                    })
                }),
                action(Icon::Next, "forward", move || {
                    if forward.session.media_source.get_untracked() != forward_source {
                        forward
                            .vm
                            .set_status("Play this episode before seeking.".into());
                        return;
                    }
                    forward.vm.request(HostRequest::MediaControl {
                        action: "forward".into(),
                    })
                }),
                action(Icon::Share, "share", move || {
                    commands::report(
                        &share,
                        share.request(HostRequest::Share {
                            title: share_episode.title.clone(),
                            text: String::new(),
                            url: share_episode.source.clone(),
                        }),
                    )
                }),
            ],
            false,
            vec![],
        ),
    )
}
fn time(seconds: u32) -> String {
    format!("{}:{:02}", seconds / 60, seconds % 60)
}
fn date(timestamp: i64) -> String {
    chrono::DateTime::from_timestamp(timestamp, 0)
        .filter(|_| timestamp != 0)
        .map(|date| date.format("%e %b %Y").to_string())
        .unwrap_or("episode".into())
}
