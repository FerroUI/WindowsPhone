use super::{action, common, Page, State};
use crate::session::Session;
use ferroui_view::prelude::*;
use phone_domain::{Id, Tile, TileBackground, TileBadge, TileNotification, TileTemplate};
use phone_state::PhoneViewModel;
use std::rc::Rc;

pub(super) fn view(vm: Rc<PhoneViewModel>, ui: Rc<Session>, tile: &Tile) -> Element {
    let id = tile.id;
    let mut body=stack_panel().spacing(4.0).child(border().margin((14.0,6.0)).child(common::muted("Up to five cards. A new sixth card replaces the oldest. Cards change while Start is visible.",16.0)));
    for card in tile.notifications.iter().rev() {
        let state = if card.starts_at > vm.now() {
            "scheduled"
        } else if card.active_at(vm.now()) {
            "active"
        } else {
            "expired"
        };
        let title = match &card.template {
            TileTemplate::Text { title, .. }
            | TileTemplate::Flip { title, .. }
            | TileTemplate::Photo { title, .. }
            | TileTemplate::Cycle { title, .. }
            | TileTemplate::List { title, .. }
            | TileTemplate::Progress { title, .. } => title,
        };
        body = body.child(border().margin((14.0, 9.0, 14.0, 0.0)).child(common::text(
            &format!(
                "{} · {state}",
                if title.is_empty() { &card.tag } else { title }
            ),
            21.0,
        )));
        let schedule_vm = vm.clone();
        let schedule_ui = ui.clone();
        let tag = card.tag.clone();
        body = body.child(action(
            &format!("schedule / badge · {}", card.tag),
            move || schedule(schedule_vm.clone(), schedule_ui.clone(), id, &tag),
        ));
        let clear = vm.clone();
        let tag = card.tag.clone();
        body = body.child(action(&format!("remove · {}", card.tag), move || {
            if let Err(error) = clear.clear_tile_notifications(id, Some(&tag)) {
                clear.set_status(error);
            }
        }));
    }
    for (label, kind) in [
        ("add text card", 0),
        ("add two-sided card", 1),
        ("add list card", 2),
        ("add progress card", 3),
        ("add photo card", 4),
        ("cycle favorite photos", 5),
    ] {
        let vm = vm.clone();
        let ui = ui.clone();
        body = body.child(action(label, move || {
            edit(vm.clone(), ui.clone(), id, kind)
        }));
    }
    if !tile.notifications.is_empty() {
        let clear = vm.clone();
        let confirm = ui.clone();
        body = body.child(action("clear all cards", move || {
            let vm = clear.clone();
            confirm.confirm(
                "Clear this tile's saved live cards?",
                Rc::new(move || {
                    if let Err(error) = vm.clear_tile_notifications(id, None) {
                        vm.set_status(error);
                    }
                }),
            );
        }));
    }
    body.into()
}
fn edit(vm: Rc<PhoneViewModel>, ui: Rc<Session>, id: Id, kind: u8) {
    let data = vm.data();
    let Some(tile) = data.tile(id) else {
        return;
    };
    let title = tile.display_label().to_owned();
    let epoch = vm.document_epoch();
    let tag = (1..=6)
        .map(|index| format!("custom.{index}"))
        .find(|tag| tile.notifications.iter().all(|card| card.tag != *tag))
        .unwrap();
    let photo = match tile.appearance.background {
        TileBackground::Photo(id) => Some(id),
        _ => None,
    };
    let favorites = data
        .photos
        .iter()
        .filter(|photo| photo.favorite)
        .take(9)
        .map(|photo| photo.id)
        .collect::<Vec<_>>();
    drop(data);
    if kind == 4 && photo.is_none() {
        vm.set_status(
            "Choose a photo background in Customize first, then add its photo card.".into(),
        );
        return;
    }
    if kind == 5 && favorites.is_empty() {
        vm.set_status(
            "Mark photos as favorites in Photos first. A cycle shows up to nine favorites.".into(),
        );
        return;
    }
    let (heading, fields) = match kind {
        0 => ("text card", vec![("title", title), ("body", String::new())]),
        1 => (
            "two-sided card",
            vec![
                ("front title", title),
                ("front text", String::new()),
                ("back title", String::new()),
                ("back text", String::new()),
            ],
        ),
        2 => (
            "list card",
            vec![
                ("title", title),
                ("first line", String::new()),
                ("second line", String::new()),
                ("third line", String::new()),
            ],
        ),
        3 => (
            "progress card",
            vec![
                ("title", title),
                ("body", String::new()),
                ("percent (0–100)", "0".into()),
            ],
        ),
        4 => (
            "photo card",
            vec![("title", title), ("body", String::new())],
        ),
        _ => ("favorite photos card", vec![("title", title)]),
    };
    let save_ui = ui.clone();
    ui.form(
        heading,
        fields,
        Rc::new(move |fields| {
            let result = (|| {
                if vm.document_epoch() != epoch {
                    return Err("This phone document has changed. Open tile actions again.".into());
                }
                let template = match kind {
                    0 => TileTemplate::Text {
                        title: fields[0].clone(),
                        body: fields[1].clone(),
                    },
                    1 => TileTemplate::Flip {
                        title: fields[0].clone(),
                        body: fields[1].clone(),
                        back_title: fields[2].clone(),
                        back_body: fields[3].clone(),
                    },
                    2 => TileTemplate::List {
                        title: fields[0].clone(),
                        lines: fields[1..]
                            .iter()
                            .map(|value| value.trim().to_owned())
                            .filter(|value| !value.is_empty())
                            .collect(),
                    },
                    3 => TileTemplate::Progress {
                        title: fields[0].clone(),
                        body: fields[1].clone(),
                        percent: fields[2]
                            .trim()
                            .parse::<u8>()
                            .map_err(|_| "Enter progress from 0 to 100.")?,
                    },
                    4 => TileTemplate::Photo {
                        photo: photo.unwrap(),
                        title: fields[0].clone(),
                        body: fields[1].clone(),
                    },
                    _ => TileTemplate::Cycle {
                        photos: favorites.clone(),
                        title: fields[0].clone(),
                    },
                };
                vm.publish_tile_notification(
                    id,
                    TileNotification::new(tag.clone(), template, vm.now()),
                )
            })();
            finish(&save_ui, result);
        }),
    );
}
fn schedule(vm: Rc<PhoneViewModel>, ui: Rc<Session>, id: Id, tag: &str) {
    let Some(card) = vm
        .data()
        .tile(id)
        .and_then(|tile| tile.notifications.iter().find(|card| card.tag == tag))
        .cloned()
    else {
        return;
    };
    let epoch = vm.document_epoch();
    let delay = card.starts_at.saturating_sub(vm.now()).max(0) / 60;
    let duration = card
        .expires_at
        .map(|end| end.saturating_sub(card.starts_at) / 60)
        .unwrap_or(0);
    let badge = match card.badge {
        Some(TileBadge::Count(count)) => count.to_string(),
        _ => String::new(),
    };
    let save_ui = ui.clone();
    ui.form(
        "card schedule + badge",
        vec![
            ("start in minutes (0 = now)", delay.to_string()),
            ("visible minutes (0 = no expiry)", duration.to_string()),
            ("badge count (blank = app badge)", badge),
        ],
        Rc::new(move |fields| {
            let result = (|| {
                if vm.document_epoch() != epoch {
                    return Err("This phone document has changed. Open tile actions again.".into());
                }
                let delay = minutes(&fields[0])?;
                let duration = minutes(&fields[1])?;
                let mut card = card.clone();
                card.starts_at = vm.now().saturating_add(delay * 60);
                card.expires_at =
                    (duration > 0).then(|| card.starts_at.saturating_add(duration * 60));
                card.badge = if fields[2].trim().is_empty() {
                    None
                } else {
                    Some(TileBadge::Count(
                        fields[2]
                            .trim()
                            .parse::<u32>()
                            .map_err(|_| "Enter a whole nonnegative badge count.")?,
                    ))
                };
                vm.publish_tile_notification(id, card)
            })();
            finish(&save_ui, result);
        }),
    );
}
fn minutes(value: &str) -> Result<i64, String> {
    let value = value
        .trim()
        .parse::<i64>()
        .map_err(|_| "Enter a whole number of minutes.")?;
    if !(0..=43_200).contains(&value) {
        return Err("Enter 0 to 43,200 minutes (30 days).".into());
    }
    Ok(value)
}
fn finish(ui: &Session, result: Result<(), String>) {
    match result {
        Ok(()) => {
            ui.state::<State>().page.set(Page::Notifications);
            ui.menu.set(true);
        }
        Err(error) => ui.form_error.set(error),
    }
}
