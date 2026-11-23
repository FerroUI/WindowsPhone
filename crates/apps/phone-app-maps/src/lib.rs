//! Saved places and directions share the SDK's persistence and host tasks.
mod commands;
mod details;
mod directions;
mod model;
mod overview;
mod registration;
mod state;
mod tile;
mod viewport;
use ferroui_base::layout::Orientation;
use ferroui_view::prelude::*;
use model::Place;
use phone_controls::{action, app_bar, flat, pivot, Icon};
use phone_sdk::{common::*, session::Session};
use phone_state::PhoneViewModel;
pub use registration::registration;
use state::MapSession;
use std::rc::Rc;

pub fn view(vm: Rc<PhoneViewModel>, ui: Rc<Session>) -> Element {
    let cx = commands::context(vm.clone(), ui.clone());
    let saved = match commands::load(&cx) {
        Ok(saved) => saved,
        Err(error) => {
            return page(
                "maps",
                empty("Map collection unavailable", &error),
                Element::Empty,
            )
        }
    };
    let state = ui.state::<MapSession>();
    if state.directions.get() {
        return directions::view(cx, &saved);
    }
    if let Some(place) = state.selection.get() {
        let current = if place.id == 0 {
            Some(place)
        } else {
            saved
                .places
                .iter()
                .find(|item| item.id == place.id)
                .cloned()
        };
        if let Some(place) = current {
            return details::view(cx, place, &saved);
        }
    }
    let selected = ui.pivot.get().min(2);
    let mut tabs = stack_panel()
        .orientation(Orientation::Horizontal)
        .margin((20.0, 0.0));
    for (index, label) in ["map", "favorites", "recent"].into_iter().enumerate() {
        let session = ui.clone();
        tabs = tabs.child(pivot(label, selected == index, move || {
            commands::cancel_search(&session);
            session.state::<MapSession>().locate.borrow_mut().take();
            session.pivot.set(index);
            session.query.set(String::new());
        }));
    }
    let tabs = phone_controls::pivot_header(tabs.into(), ["map", "favorites", "recent"][selected]);
    let body = if selected == 0 {
        overview::view(cx.clone(), &saved, tabs)
    } else {
        let mut body = stack_panel().spacing(10.0).child(tabs);
        if selected == 1 {
            if saved.places.is_empty() {
                body = body.child(empty(
                    "Keep your places close",
                    "Tap add place to save an address or coordinates.",
                ));
            }
            let mut places = saved.places.clone();
            places.sort_by_key(|place| place.name.to_lowercase());
            for place in places {
                let context = cx.clone();
                let title = place.name.clone();
                let description = place.destination();
                body = body.child(row(&title, &description, move || {
                    commands::select(&context, place.clone())
                }));
            }
        } else {
            if saved.recent.is_empty() {
                body = body.child(empty(
                    "No recent searches",
                    "Places you search for will appear here.",
                ));
            }
            for query in &saved.recent {
                let context = cx.clone();
                let query = query.clone();
                body = body.child(row(&query.clone(), "search again", move || {
                    context.session.query.set(query.clone());
                    context.session.pivot.set(0);
                    commands::search(context.clone());
                }));
            }
        }
        scroll(body.into())
    };
    let search = cx.clone();
    let locate = cx.clone();
    let add = cx.clone();
    let more = ui.clone();
    let route = cx.clone();
    let clear = cx.clone();
    let menu = vec![
        flat("directions", move || {
            commands::directions(&route, route.session.query.get_untracked())
        }),
        flat("clear recent searches", move || {
            let context = clear.clone();
            clear.session.confirm(
                "clear recent searches?",
                Rc::new(move || {
                    if let Err(error) = commands::change(&context, |saved| {
                        saved.recent.clear();
                        Ok(())
                    }) {
                        context.vm.set_status(error)
                    }
                }),
            );
        }),
    ];
    page(
        "maps",
        body,
        app_bar(
            vec![
                action(Icon::Search, "search", move || {
                    search.session.pivot.set(0);
                    commands::search(search.clone());
                }),
                action(Icon::Location, "my location", move || {
                    commands::locate(&locate)
                }),
                action(Icon::Plus, "add place", move || {
                    let query = add.session.query.get_untracked().trim().to_owned();
                    commands::edit(
                        add.clone(),
                        Place {
                            id: 0,
                            name: query.chars().take(100).collect(),
                            address: query,
                            coordinates: None,
                        },
                        None,
                    );
                }),
                action(Icon::More, "more", move || {
                    more.menu.set(!more.menu.get_untracked())
                }),
            ],
            ui.menu.get(),
            menu,
        ),
    )
}
