use super::{cadence, live};
use crate::{common, images, session::Session};
use ferroui_base::layout::{HorizontalAlignment as H, VerticalAlignment as V};
use ferroui_base::media::{FontWeight, TextWrapping};
use ferroui_view::prelude::*;
use phone_controls::{icon, Icon};
use phone_domain::{AppId, Tile, TileSize, TileTarget};
use phone_state::PhoneViewModel;
use std::rc::Rc;

pub(super) fn view(
    vm: Rc<PhoneViewModel>,
    ui: Rc<Session>,
    tile: &Tile,
    w: f64,
    h: f64,
    badge: usize,
) -> Element {
    let small = tile.size == TileSize::Small;
    let wide = tile.size == TileSize::Wide;
    let data = vm.data();
    let cycling = data.settings.live_tiles_enabled();
    let animate = cycling && ui.edit_tile.get().is_none();
    if let Some(target) = tile.target {
        return match target {
            TileTarget::Contact(id) => {
                let initials = data
                    .contact(id)
                    .map(|contact| contact.initials())
                    .unwrap_or_default();
                rsx! {TextBlock{text:initials,font_size:if small{31.0}else{62.0},font_weight:FontWeight::Light,foreground:"White",horizontal_alignment:H::Center,vertical_alignment:V::Center,margin:if small{(0.0,0.0,0.0,0.0)}else{(0.0,0.0,0.0,20.0)}}}
            }
            TileTarget::Note(id) if !small => {
                let body = data
                    .note(id)
                    .map(|note| {
                        note.body
                            .chars()
                            .take(if wide { 160 } else { 90 })
                            .collect::<String>()
                    })
                    .unwrap_or_default();
                rsx! {StackPanel{margin:(12.0,12.0,12.0,38.0),spacing:7.0,{icon(Icon::Note,29.0,"White")},TextBlock{text:body,font_size:19.0,foreground:"White",text_wrapping:TextWrapping::Wrap}}}
            }
            TileTarget::Document(id) if !small => {
                let document = data.document(id);
                let snippet = document
                    .map(|document| live::document_preview(document, if wide { 140 } else { 75 }))
                    .unwrap_or_default();
                let letter = document
                    .map(|document| document.kind().letter())
                    .unwrap_or("O");
                rsx! {StackPanel{margin:(12.0,10.0,12.0,38.0),spacing:7.0,
                    TextBlock{text:letter,font_size:32.0,font_weight:FontWeight::SemiBold,foreground:"White"}
                    TextBlock{text:snippet,font_size:18.0,foreground:"White",text_wrapping:TextWrapping::Wrap}
                }}
            }
            TileTarget::Bookmark(id) if !small => {
                let url = data
                    .bookmarks
                    .iter()
                    .find(|bookmark| bookmark.id == id)
                    .map(|bookmark| {
                        bookmark
                            .url
                            .split("://")
                            .last()
                            .unwrap_or(&bookmark.url)
                            .trim_end_matches('/')
                            .to_owned()
                    })
                    .unwrap_or_default();
                rsx! {StackPanel{margin:(12.0,14.0,12.0,38.0),spacing:10.0,{icon(Icon::Explorer,40.0,"White")},TextBlock{text:url,font_size:17.0,foreground:"White",text_wrapping:TextWrapping::Wrap}}}
            }
            _ => icon(
                common::app_icon(tile.app),
                if small { 31.0 } else { 56.0 },
                "White",
            ),
        };
    }
    match tile.app {
        AppId::Photos if !data.photos.is_empty() => {
            let now = if cycling { ui.live_clock.get() } else { 0 };
            let index = cadence::cycle(now, 17, tile.id, data.photos.len(), cycling);
            let photo = &data.photos[index];
            live::frame(
                format!("photo-tile-{}-{}", tile.id, photo.id),
                rsx! {Grid{{images::photo(&photo.source)},Border{background:"#11000000"}}},
                h * 0.65,
                animate,
            )
        }
        AppId::Calendar if !small => {
            let date = chrono::DateTime::from_timestamp(vm.local_now(), 0).unwrap_or_default();
            let today = date.format("%Y-%m-%d").to_string();
            let next = phone_domain::next_calendar_occurrence(&data.events, vm.local_now())
                .and_then(|occurrence| {
                    data.events
                        .iter()
                        .find(|event| event.id == occurrence.event_id)
                        .map(|event| {
                            let time = if event.all_day {
                                "all day".into()
                            } else {
                                phone_domain::time_text(occurrence.start, data.settings.time_24h)
                            };
                            (
                                event.title.clone(),
                                format!(
                                    "{}{}  {}",
                                    if occurrence.date == today {
                                        String::new()
                                    } else {
                                        format!("{}  ", occurrence.date)
                                    },
                                    time,
                                    event.location
                                ),
                            )
                        })
                });
            rsx! {Grid{
                StackPanel{margin:(12.0,7.0),spacing:0.0,horizontal_alignment:H::Left,
                    TextBlock{text:date.format("%A").to_string(),font_size:16.0,foreground:"White"},
                    TextBlock{text:date.format("%-d").to_string(),font_size:70.0,font_weight:FontWeight::Light,foreground:"White"}
                },
                if wide{StackPanel{margin:(133.0,23.0,14.0,36.0),spacing:8.0,
                    TextBlock{text:next.as_ref().map(|(title,_)|title.clone()).unwrap_or_else(||"No more events".into()),font_size:22.0,foreground:"White",text_wrapping:TextWrapping::Wrap},
                    TextBlock{text:next.as_ref().map(|(_,detail)|detail.clone()).unwrap_or_default(),font_size:16.0,foreground:"White",text_wrapping:TextWrapping::Wrap}
                }}
            }}
        }
        AppId::People if !small && !data.contacts.is_empty() => {
            let now = if cycling { ui.live_clock.get() } else { 0 };
            let mut mosaic = grid()
                .row_definitions("*,*")
                .column_definitions("*,*,*")
                .margin((0.0, 0.0, 0.0, 28.0));
            for i in 0..6 {
                let person = &data.contacts[cadence::person(now, i, data.contacts.len(), cycling)];
                let content = rsx! {Border{background:person.color.as_str(),TextBlock{name:format!("PeopleCell-{}-{i}",tile.id),text:person.initials(),font_size:if wide{28.0}else{22.0},font_weight:FontWeight::Light,foreground:"#EEFFFFFF",horizontal_alignment:H::Center,vertical_alignment:V::Center}}};
                mosaic = mosaic.child(
                    border()
                        .key(format!("people-cell-{i}"))
                        .grid_row((i / 3) as i32)
                        .grid_column((i % 3) as i32)
                        .clip_to_bounds(true)
                        .child(live::frame(
                            format!("person-{}", person.id),
                            content,
                            28.0,
                            animate,
                        )),
                );
            }
            mosaic.into()
        }
        AppId::Messaging
            if wide
                && data
                    .conversations
                    .iter()
                    .any(|conversation| conversation.unread) =>
        {
            let message = data
                .conversations
                .iter()
                .filter(|conversation| conversation.unread)
                .max_by_key(|conversation| conversation.timestamp())
                .unwrap();
            preview(
                Icon::Message,
                &data.contact_name(message),
                &message.preview(),
                badge,
            )
        }
        AppId::Outlook
            if wide
                && data
                    .mail
                    .iter()
                    .any(|mail| !mail.read && mail.folder == "inbox") =>
        {
            let mail = data
                .mail
                .iter()
                .filter(|mail| !mail.read && mail.folder == "inbox")
                .max_by_key(|mail| mail.timestamp)
                .unwrap();
            preview(Icon::Mail, &mail.from, &mail.subject, badge)
        }
        AppId::Weather if !small => {
            let weather = ui.weather.get();
            rsx! {Grid{margin:(14.0,12.0,14.0,32.0),column_definitions:"70,*",
                {icon(Icon::Weather,55.0,"White")},
                StackPanel{Grid.column:1,spacing:3.0,vertical_alignment:V::Center,
                    TextBlock{text:weather.as_ref().map(|(_,degrees,_)|format!("{degrees:.0}°")).unwrap_or_else(||"Weather".into()),font_size:if weather.is_some(){42.0}else{29.0},foreground:"White",font_weight:FontWeight::Light},
                    TextBlock{text:weather.as_ref().map(|(location,_,description)|format!("{location}\n{description}")).unwrap_or_else(||"tap for your forecast".into()),font_size:14.0,foreground:"White",text_wrapping:TextWrapping::Wrap}
                }
            }}
        }
        AppId::OneNote if !small && !data.notes.is_empty() => {
            let note = data.notes.iter().max_by_key(|note| note.modified).unwrap();
            rsx! {StackPanel{margin:(12.0,12.0,12.0,35.0),spacing:8.0,{icon(Icon::Note,32.0,"White")},
                TextBlock{text:note.title.clone(),font_size:19.0,foreground:"White",text_wrapping:TextWrapping::Wrap}
            }}
        }
        _ => {
            let mut inner = grid().child(icon(
                common::app_icon(tile.app),
                if small { 31.0 } else { 56.0 },
                "White",
            ));
            if badge > 0 {
                inner = inner.child(
                    text_block()
                        .text(badge.to_string())
                        .font_size(if small { 19.0 } else { 34.0 })
                        .font_weight(FontWeight::Light)
                        .foreground("White")
                        .horizontal_alignment(H::Right)
                        .vertical_alignment(V::Top)
                        .margin((
                            0.0,
                            if small { 3.0 } else { 12.0 },
                            if small { 5.0 } else { 14.0 },
                            0.0,
                        )),
                );
            }
            let _ = (w, h);
            inner.into()
        }
    }
}
fn preview(kind: Icon, title: &str, body: &str, badge: usize) -> Element {
    rsx! {Grid{margin:(12.0,13.0,14.0,34.0),column_definitions:"52,*",
        StackPanel{spacing:5.0,vertical_alignment:V::Top,{icon(kind,34.0,"White")},TextBlock{text:badge.to_string(),font_size:27.0,font_weight:FontWeight::Light,foreground:"White",horizontal_alignment:H::Center}},
        StackPanel{Grid.column:1,spacing:6.0,margin:(10.0,0.0,0.0,0.0),TextBlock{text:title.to_owned(),font_size:23.0,foreground:"White"},TextBlock{text:body.to_owned(),font_size:18.0,foreground:"White",text_wrapping:TextWrapping::Wrap}}
    }}
}
