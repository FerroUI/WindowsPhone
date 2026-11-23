//! Retained Music and Video libraries over shared playlist and playback services.
pub mod collections;
mod commands;
mod listing;
mod model;
mod player;
mod state;

use ferroui_base::layout::Orientation;
use ferroui_view::prelude::*;
use phone_controls::{action, app_bar, flat, pivot, Icon};
use phone_domain::AppId;
use phone_sdk::{common::*, media, session::Session};
use phone_state::{HostRequest, PhoneViewModel};
use state::{LibraryPage, LibrarySession};
use std::rc::Rc;

pub fn view(vm: Rc<PhoneViewModel>, ui: Rc<Session>, app: AppId) -> Element {
    let library = match model::read(&vm, app) {
        Ok(value) => value,
        Err(error) => {
            return page(
                app.label(),
                scroll(empty("can't read this library", &error)),
                Element::Empty,
            )
        }
    };
    let video = app == AppId::Video;
    let tracks = vm
        .data()
        .tracks
        .iter()
        .filter(|t| (t.media_kind == "video") == video)
        .cloned()
        .collect::<Vec<_>>();
    let selected_pivot = ui.pivot.get().min(1);
    let mut tabs = stack_panel()
        .orientation(Orientation::Horizontal)
        .margin((20.0, 0.0));
    for (index, label) in ["collection", "now playing"].iter().enumerate() {
        let state = ui.clone();
        tabs = tabs.child(pivot(label, index == selected_pivot, move || {
            state.pivot.set(index)
        }));
    }
    let tabs =
        phone_controls::pivot_header(tabs.into(), ["collection", "now playing"][selected_pivot]);
    let content = if selected_pivot == 1 {
        player::view(vm.clone(), ui.clone(), app, &library)
    } else {
        listing::view(vm.clone(), ui.clone(), app, &library, tracks.clone())
    };
    let selecting = matches!(
        ui.state::<LibrarySession>().page(app).get(),
        LibraryPage::Select(_)
    );
    let (prev_vm, prev_ui, next_vm, next_ui, play_vm, play_ui, more) = (
        vm.clone(),
        ui.clone(),
        vm.clone(),
        ui.clone(),
        vm.clone(),
        ui.clone(),
        ui.clone(),
    );
    let actions = if selecting && selected_pivot == 0 {
        let save_vm = vm.clone();
        let save_ui = ui.clone();
        let cancel = ui.clone();
        vec![
            action(Icon::Check, "done", move || {
                commands::save_selection(&save_vm, &save_ui, app)
            }),
            action(Icon::Close, "cancel", move || {
                back_selection(&cancel, app);
            }),
        ]
    } else {
        vec![
            action(Icon::Previous, "previous", move || {
                media::control(&prev_vm, &prev_ui, "previous");
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
                    } else if !play_ui.media_source.get_untracked().is_empty() {
                        play_vm.request(HostRequest::MediaControl {
                            action: "play".into(),
                        });
                    } else if let Some(track) = tracks.first() {
                        commands::play(&play_vm, &play_ui, app, track.id);
                    }
                },
            ),
            action(Icon::Next, "next", move || {
                media::control(&next_vm, &next_ui, "next");
            }),
            action(Icon::More, "more", move || more.menu.update(|v| *v = !*v)),
        ]
    };
    let import = vm.clone();
    let create_vm = vm.clone();
    let create_ui = ui.clone();
    let mut secondary = vec![
        flat(if video { "add videos" } else { "add music" }, move || {
            import.request(if video {
                HostRequest::ImportVideo
            } else {
                HostRequest::ImportMusic
            })
        }),
        flat("new playlist", move || {
            commands::create_playlist(&create_vm, &create_ui, app)
        }),
    ];
    let current = ui.media_source.get();
    if let Some(track) = vm
        .data()
        .tracks
        .iter()
        .find(|track| track.source == current && (track.media_kind == "video") == video)
        .cloned()
    {
        let (edit_vm, edit_ui, id) = (vm.clone(), ui.clone(), track.id);
        secondary.push(flat("edit title and artist", move || {
            commands::edit_track(&edit_vm, &edit_ui, id)
        }));
        if !video {
            let (album_vm, album_ui) = (vm.clone(), ui.clone());
            secondary.push(flat("set album", move || {
                commands::edit_album(&album_vm, &album_ui, app, id)
            }));
        }
        let (remove_vm, remove_ui) = (vm.clone(), ui.clone());
        secondary.push(flat("remove current item", move || {
            commands::remove(&remove_vm, &remove_ui, app, id)
        }));
        if vm.environment().platform != phone_sdk::host::HostPlatform::Browser {
            let external = vm.clone();
            secondary.push(flat("open in device media app", move || {
                external.request(HostRequest::MediaControl {
                    action: "external".into(),
                })
            }));
        }
    }
    page(
        if video { "video" } else { "music" },
        rsx! {Grid{row_definitions:"Auto,*",{tabs}Border{Grid.row:1,{content}}}},
        app_bar(actions, ui.menu.get() && !selecting, secondary),
    )
}

fn back_selection(ui: &Session, app: AppId) {
    let state = ui.state::<LibrarySession>();
    if let LibraryPage::Select(id) = state.page(app).get_untracked() {
        state.page(app).set(LibraryPage::Playlist(id));
    }
    state.selection.set(Default::default());
}
pub fn back(_vm: &Rc<PhoneViewModel>, ui: &Rc<Session>, app: AppId) -> bool {
    let state = ui.state::<LibrarySession>();
    if state.queue_open.get_untracked() {
        state.queue_open.set(false);
        return true;
    }
    if ui.pivot.get_untracked() != 0 {
        return false;
    }
    let parent = match state.page(app).get_untracked() {
        LibraryPage::Songs => return false,
        LibraryPage::Artist(_) => LibraryPage::Artists,
        LibraryPage::Album(_) => LibraryPage::Albums,
        LibraryPage::Playlist(_) => LibraryPage::Playlists,
        LibraryPage::Select(id) => LibraryPage::Playlist(id),
        _ => LibraryPage::Songs,
    };
    state.page(app).set(parent);
    ui.query.set(String::new());
    true
}
pub fn reset(ui: &Rc<Session>, app: AppId) {
    let state = ui.state::<LibrarySession>();
    state.page(app).set(LibraryPage::Songs);
    state.selection.set(Default::default());
    state.queue_open.set(false);
}
