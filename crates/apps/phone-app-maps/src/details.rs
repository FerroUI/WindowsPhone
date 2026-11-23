use crate::{
    commands,
    model::{Place, Saved},
    state::MapSession,
};
use ferroui_view::prelude::*;
use phone_controls::{action, app_bar, flat, Icon};
use phone_sdk::{common::*, AppContext};
use phone_state::HostRequest;
use std::rc::Rc;
pub fn view(cx: AppContext, place: Place, saved: &Saved) -> Element {
    let coordinates = place
        .coordinates
        .map(|value| value.label())
        .unwrap_or_default();
    let shortcut = if saved.home == Some(place.id) {
        "home"
    } else if saved.work == Some(place.id) {
        "work"
    } else {
        ""
    };
    let body = rsx! {StackPanel{margin:20.0,spacing:20.0,
        {text(&place.name,38.0)}
        if !shortcut.is_empty(){{muted(shortcut,18.0)}}
        if !place.address.is_empty(){{text(&place.address,23.0)}}
        if !coordinates.is_empty()&&place.name!=coordinates{{muted(&coordinates,18.0)}}
        if place.coordinates.is_some(){{crate::viewport::view(cx.clone(),saved.clone(),Some(place.clone()),Some(240.0))}}
        {muted(if place.coordinates.is_some(){"Drag to explore. Tap the map to choose another place, or open directions in your maps app."}else{"Open the map to find this address, or add coordinates when editing to show it here."},18.0)}
    }};
    let open = cx.clone();
    let open_place = place.clone();
    let route = cx.clone();
    let route_place = place.clone();
    let edit = cx.clone();
    let edit_place = place.clone();
    let more = cx.session.clone();
    let mut menu = Vec::new();
    if place.id != 0 {
        let pin = cx.clone();
        let pin_place = place.clone();
        menu.push(flat("pin place to Start", move || {
            commands::pin(&pin, &pin_place)
        }));
        for (home, label) in [(true, "set as home"), (false, "set as work")] {
            let context = cx.clone();
            let id = place.id;
            menu.push(flat(label, move || {
                commands::set_shortcut(&context, id, home)
            }));
        }
        let context = cx.clone();
        let id = place.id;
        menu.push(flat("delete place", move || {
            let action = context.clone();
            context.session.confirm(
                "delete this saved place?",
                Rc::new(move || match commands::remove(&action, id) {
                    Ok(()) => action.session.state::<MapSession>().selection.set(None),
                    Err(error) => action.vm.set_status(error),
                }),
            );
        }));
    }
    let share = cx.clone();
    let share_place = place.clone();
    menu.push(flat("share place", move || match share_place.map_url() {
        Ok(url) => share.vm.request(HostRequest::Share {
            title: share_place.name.clone(),
            text: share_place.destination(),
            url,
        }),
        Err(error) => share.vm.set_status(error),
    }));
    page(
        "maps",
        scroll(body),
        app_bar(
            vec![
                action(Icon::Maps, "open map", move || {
                    commands::open_map(&open, &open_place)
                }),
                action(Icon::Forward, "directions", move || {
                    commands::directions(&route, route_place.destination())
                }),
                action(
                    if place.id == 0 {
                        Icon::Star
                    } else {
                        Icon::Edit
                    },
                    if place.id == 0 { "save" } else { "edit" },
                    move || commands::edit(edit.clone(), edit_place.clone(), None),
                ),
                action(Icon::More, "more", move || {
                    more.menu.set(!more.menu.get_untracked())
                }),
            ],
            cx.session.menu.get(),
            menu,
        ),
    )
}
