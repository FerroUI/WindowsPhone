use crate::{common::*, session::Session};
use ferroui_base::layout::{HorizontalAlignment as H, Orientation, VerticalAlignment as V};
use ferroui_view::prelude::*;
use phone_controls::{action, app_bar, flat, icon, pivot, Icon};
use phone_domain::{Action, AppId, MediaTrack};
use phone_state::{HostRequest, PhoneViewModel};
use std::rc::Rc;

pub fn view(vm: Rc<PhoneViewModel>, ui: Rc<Session>, app: AppId) -> Element {
    let video = app == AppId::Video;
    let tracks = vm
        .data()
        .tracks
        .iter()
        .filter(|t| (t.media_kind == "video") == video)
        .cloned()
        .collect::<Vec<_>>();
    let current = ui.media_source.get();
    let selected = tracks.iter().find(|t| t.source == current).cloned();
    let mut body = stack_panel();
    let selected_pivot = ui.pivot.get().min(1);
    let mut tabs = stack_panel()
        .orientation(Orientation::Horizontal)
        .margin((20.0, 0.0));
    for (i, label) in ["collection", "now playing"].iter().enumerate() {
        let switch = ui.clone();
        tabs = tabs.child(pivot(label, i == selected_pivot, move || {
            switch.pivot.set(i)
        }));
    }
    body = body.child(tabs);
    if selected_pivot == 1 {
        if let Some(track) = selected.clone() {
            body = body.child(now_playing(vm.clone(), ui.clone(), &track, video));
        } else {
            body = body.child(empty(
                "nothing playing",
                "Choose something from your collection.",
            ));
        }
    } else if tracks.is_empty() {
        body = body.child(empty(
            if video { "your videos" } else { "your music" },
            if video {
                "Add a video from your device to watch it here."
            } else {
                "Add songs from your device to build your collection."
            },
        ));
        let import = vm.clone();
        body = body.child(border().margin(20.0).child(phone_controls::command(
            if video { "add videos" } else { "add music" },
            move || {
                import.request(if video {
                    HostRequest::ImportVideo
                } else {
                    HostRequest::ImportMusic
                })
            },
        )));
    } else {
        for track in &tracks {
            let vm = vm.clone();
            let ui = ui.clone();
            let chosen = track.clone();
            body = body.child(row(
                &track.title,
                if track.artist.is_empty() {
                    "unknown artist"
                } else {
                    &track.artist
                },
                move || play(&vm, &ui, &chosen),
            ));
        }
    }
    let import = vm.clone();
    let prev_vm = vm.clone();
    let next_vm = vm.clone();
    let play_vm = vm.clone();
    let prev_ui = ui.clone();
    let next_ui = ui.clone();
    let play_ui = ui.clone();
    let prev_tracks = tracks.clone();
    let next_tracks = tracks.clone();
    let play_tracks = tracks.clone();
    let menu = ui.clone();
    let remove_vm = vm.clone();
    let remove_ui = ui.clone();
    let secondary = vec![
        flat(if video { "add videos" } else { "add music" }, move || {
            import.request(if video {
                HostRequest::ImportVideo
            } else {
                HostRequest::ImportMusic
            })
        }),
        flat("remove current item", move || {
            let source = remove_ui.media_source.get_untracked();
            let track = remove_vm
                .data()
                .tracks
                .iter()
                .find(|t| t.source == source)
                .cloned();
            if let Some(track) = track {
                remove_vm.request(HostRequest::MediaControl {
                    action: "stop".into(),
                });
                remove_vm.dispatch(Action::DeleteTrack(track.id));
                remove_vm.request(HostRequest::DeleteMedia {
                    source: track.source,
                });
                remove_ui.media_source.set(String::new());
                remove_ui.media_playing.set(false);
            }
        }),
    ];
    page(
        if video { "video" } else { "music" },
        scroll(body.into()),
        app_bar(
            vec![
                action(Icon::Previous, "previous", move || {
                    step(&prev_vm, &prev_ui, &prev_tracks, -1)
                }),
                action(
                    if ui.media_playing.get() {
                        Icon::Pause
                    } else {
                        Icon::Play
                    },
                    if ui.media_playing.get() {
                        "pause"
                    } else {
                        "play"
                    },
                    move || {
                        if play_ui.media_playing.get_untracked() {
                            play_vm.request(HostRequest::MediaControl {
                                action: "pause".into(),
                            });
                            play_ui.media_playing.set(false);
                        } else if let Some(track) = play_tracks
                            .iter()
                            .find(|t| t.source == play_ui.media_source.get_untracked())
                            .or(play_tracks.first())
                        {
                            play(&play_vm, &play_ui, track);
                        }
                    },
                ),
                action(Icon::Next, "next", move || {
                    step(&next_vm, &next_ui, &next_tracks, 1)
                }),
                action(Icon::More, "more", move || menu.menu.update(|v| *v = !*v)),
            ],
            ui.menu.get(),
            secondary,
        ),
    )
}

fn now_playing(
    vm: Rc<PhoneViewModel>,
    ui: Rc<Session>,
    track: &MediaTrack,
    video: bool,
) -> Element {
    let accent = accent(&vm);
    let graphic = if video {
        rsx! {Border{name:"MediaViewport",height:230.0,background:"Black",{icon(Icon::Video,60.0,"#666666")}}}
    } else {
        rsx! {Border{height:240.0,background:"#F15A24",{icon(Icon::Music,105.0,"White")}}}
    };
    let mut player = stack_panel()
        .margin(20.0)
        .spacing(15.0)
        .child(graphic)
        .child(text(&track.title, 30.0))
        .child(muted(
            if track.artist.is_empty() {
                "unknown artist"
            } else {
                &track.artist
            },
            20.0,
        ));
    player = player.child(rsx! {Border{height:3.0,background:accent.as_str()}});
    if track.duration_seconds > 0.0 {
        player = player.child(muted(
            &format!(
                "{}:{:02}",
                (track.duration_seconds as u64) / 60,
                (track.duration_seconds as u64) % 60
            ),
            14.0,
        ));
    }
    let pause = vm.clone();
    let state = ui.clone();
    player=player.child(rsx!{Button{class:"phone-flat",min_height:44.0,horizontal_alignment:H::Center,vertical_alignment:V::Center,
        on_click:move |_|{pause.request(HostRequest::MediaControl{action:"stop".into()});state.media_playing.set(false);},
        {muted("stop",16.0)}
    }});
    player.into()
}

fn play(vm: &PhoneViewModel, ui: &Session, track: &MediaTrack) {
    ui.media_title.set(track.title.clone());
    ui.media_source.set(track.source.clone());
    ui.media_playing.set(true);
    ui.pivot.set(1);
    vm.request(HostRequest::PlayMedia {
        source: track.source.clone(),
    });
}
fn step(vm: &PhoneViewModel, ui: &Session, tracks: &[MediaTrack], direction: isize) {
    if tracks.is_empty() {
        return;
    }
    let current = ui.media_source.get_untracked();
    let index = tracks
        .iter()
        .position(|t| t.source == current)
        .map(|i| (i as isize + direction).rem_euclid(tracks.len() as isize) as usize)
        .unwrap_or(0);
    play(vm, ui, &tracks[index]);
}
