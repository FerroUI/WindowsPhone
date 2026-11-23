use crate::{
    commands,
    model::{self, Library},
    state::LibrarySession,
};
use ferroui_base::{
    layout::{HorizontalAlignment as H, Orientation},
    BoxedValue,
};
use ferroui_controls::ItemsSource;
use ferroui_view::prelude::*;
use phone_controls::{flat, icon, Icon};
use phone_domain::AppId;
use phone_sdk::{
    common::*,
    media::{self, PlaybackSession},
    session::Session,
};
use phone_state::{HostRequest, PhoneViewModel};
use std::{cell::RefCell, rc::Rc};

pub fn view(vm: Rc<PhoneViewModel>, ui: Rc<Session>, app: AppId, library: &Library) -> Element {
    let video = app == AppId::Video;
    let current = ui.media_source.get();
    let Some(track) = vm
        .data()
        .tracks
        .iter()
        .find(|track| track.source == current && (track.media_kind == "video") == video)
        .cloned()
    else {
        return scroll(empty(
            "nothing playing",
            "Choose something from your collection.",
        ));
    };
    let playback = ui.state::<PlaybackSession>();
    let queue = playback.queue.get();
    if ui.state::<LibrarySession>().queue_open.get() {
        return queue_view(vm, ui, queue);
    }
    let graphic = if video {
        rsx! {Border{name:"MediaViewport",height:230.0,background:"Black",Grid{{icon(Icon::Video,60.0,"#666666")}{phone_sdk::host_surface(phone_sdk::HostSurface::Video)}}}}
    } else {
        rsx! {Border{height:210.0,background:"#F15A24",{icon(Icon::Music,96.0,"White")}}}
    };
    let duration = playback.duration.get();
    let position = playback.position.get();
    let width = (vm.viewport().0.min(400.0) - 40.0).max(80.0);
    let progress = if duration > 0.0 {
        (position / duration).clamp(0.0, 1.0)
    } else {
        0.0
    };
    let mut body = stack_panel()
        .margin(20.0)
        .spacing(12.0)
        .child(graphic)
        .child(text(&track.title, 29.0))
        .child(muted(commands::artist_name(&track), 20.0));
    if !video {
        body = body.child(muted(library.album(track.id), 16.0));
    }
    body=body.child(rsx!{Grid{height:3.0,background:phone_controls::secondary_color(),Border{width:width*progress,horizontal_alignment:H::Left,background:accent(&vm).as_str()}}})
        .child(rsx!{Grid{column_definitions:"*,*",{muted(&time(position),14.0)}Border{Grid.column:1,horizontal_alignment:H::Right,{muted(&time(duration),14.0)}}}});
    let rewind = vm.clone();
    let forward = vm.clone();
    body=body.child(rsx!{Grid{column_definitions:"*,*",{flat("−10 seconds",move||rewind.request(HostRequest::MediaControl{action:"rewind".into()}))}Border{Grid.column:1,{flat("+10 seconds",move||forward.request(HostRequest::MediaControl{action:"forward".into()}))}}}});
    let (shuffle_vm, shuffle_ui, repeat_vm, repeat_ui) =
        (vm.clone(), ui.clone(), vm.clone(), ui.clone());
    let mut options = stack_panel()
        .orientation(Orientation::Horizontal)
        .spacing(20.0);
    options = options.child(flat(
        if library.shuffle {
            "shuffle on"
        } else {
            "shuffle off"
        },
        move || {
            let mut value = false;
            let result = model::change(&shuffle_vm, app, |library| {
                library.shuffle = !library.shuffle;
                value = library.shuffle;
                Ok(())
            });
            match result {
                Ok(()) => shuffle_ui
                    .state::<PlaybackSession>()
                    .queue
                    .update(|queue| queue.set_shuffle(value, shuffle_vm.now() as u64)),
                Err(error) => shuffle_vm.set_status(error),
            }
        },
    ));
    options = options.child(flat(library.repeat.label(), move || {
        let mut value = media::RepeatMode::Off;
        let result = model::change(&repeat_vm, app, |library| {
            library.repeat = library.repeat.next();
            value = library.repeat;
            Ok(())
        });
        match result {
            Ok(()) => repeat_ui
                .state::<PlaybackSession>()
                .queue
                .update(|queue| queue.repeat = value),
            Err(error) => repeat_vm.set_status(error),
        }
    }));
    let queue_ui = ui.clone();
    let stop = vm.clone();
    body = body
        .child(options)
        .child(flat(
            &format!("queue · {} items", queue.len()),
            move || queue_ui.state::<LibrarySession>().queue_open.set(true),
        ))
        .child(flat("stop", move || {
            stop.request(HostRequest::MediaControl {
                action: "stop".into(),
            })
        }));
    scroll(body.into())
}

#[derive(Clone, PartialEq)]
struct QueueRow {
    title: String,
    subtitle: String,
    source: String,
}
struct QueueRows {
    items: Vec<QueueRow>,
    source: ItemsSource,
}
impl QueueRows {
    fn new(items: Vec<QueueRow>) -> Self {
        let source = ItemsSource::from_items(
            items
                .iter()
                .cloned()
                .map(|row| Some(Rc::new(row) as BoxedValue)),
        );
        Self { items, source }
    }
}
fn queue_view(vm: Rc<PhoneViewModel>, ui: Rc<Session>, queue: media::MediaQueue) -> Element {
    let data = vm.data();
    let tracks = data
        .tracks
        .iter()
        .map(|track| (track.source.as_str(), track))
        .collect::<std::collections::HashMap<_, _>>();
    let items = queue
        .sources()
        .enumerate()
        .filter_map(|(index, source)| {
            tracks.get(source).map(|track| QueueRow {
                title: track.title.clone(),
                subtitle: format!(
                    "{}{}",
                    if index == queue.position() {
                        "playing · "
                    } else {
                        ""
                    },
                    commands::artist_name(track)
                ),
                source: source.into(),
            })
        })
        .collect::<Vec<_>>();
    drop(data);
    let list = rerender(move || {
        let cached = use_hook(|| Rc::new(RefCell::new(QueueRows::new(items.clone()))));
        let source = {
            let mut cached = cached.borrow_mut();
            if cached.items != items {
                *cached = QueueRows::new(items.clone());
            }
            cached.source.clone()
        };
        let template = use_hook({
            let vm = vm.clone();
            let ui = ui.clone();
            move || {
                template(move |item: QueueRow| {
                    let model = vm.clone();
                    let state = ui.clone();
                    let source = item.source;
                    row(&item.title, &item.subtitle, move || {
                        state.state::<PlaybackSession>().queue.update(|queue| {
                            queue.select(&source);
                        });
                        model.request(HostRequest::PlayMedia {
                            source: source.clone(),
                        });
                        state.state::<LibrarySession>().queue_open.set(false);
                    })
                })
            }
        });
        rsx! {ListBox{class:"phone-list",items_source:source,item_template:template}}
    });
    rsx! {Grid{row_definitions:"Auto,*",Border{margin:20.0,{text("now playing queue",30.0)}}Border{Grid.row:1,{list}}}}
}
fn time(seconds: f64) -> String {
    if !seconds.is_finite() || seconds < 0.0 {
        return "0:00".into();
    }
    let seconds = seconds as u64;
    format!("{}:{:02}", seconds / 60, seconds % 60)
}
