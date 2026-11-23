use crate::{
    model::{self, Place, Saved},
    state::MapSession,
};
use ferroui_base::threading::Dispatcher;
use phone_app_support::geography::{self, Coordinates, PlaceMatch};
use phone_sdk::{session::Session, AppContext};
use phone_state::{HostRequest, PhoneViewModel};
use std::rc::Rc;
pub fn context(vm: Rc<PhoneViewModel>, session: Rc<Session>) -> AppContext {
    AppContext {
        vm,
        session,
        id: "phone.maps",
    }
}
pub fn load(cx: &AppContext) -> Result<Saved, String> {
    let saved = cx.storage().read::<Saved>()?;
    saved.validate()?;
    Ok(saved)
}
pub fn change<T>(
    cx: &AppContext,
    apply: impl FnOnce(&mut Saved) -> Result<T, String>,
) -> Result<T, String> {
    let mut saved = load(cx)?;
    let result = apply(&mut saved)?;
    saved.validate()?;
    cx.storage().write(&saved)?;
    Ok(result)
}
pub fn select(cx: &AppContext, place: Place) {
    cx.session.menu.set(false);
    cx.session.state::<MapSession>().selection.set(Some(place));
}
pub fn search(cx: AppContext) {
    let state = cx.session.state::<MapSession>();
    let query = cx.session.query.get_untracked().trim().to_owned();
    if query.is_empty() {
        cx.vm
            .set_status("Enter a city, address or coordinates.".into());
        return;
    }
    let coordinates = if model::looks_like_coordinates(&query) {
        match Coordinates::parse(&query) {
            Ok(value) => Some(value),
            Err(error) => {
                cx.vm.set_status(error);
                return;
            }
        }
    } else {
        None
    };
    if let Err(error) = change(&cx, |saved| saved.remember(&query)) {
        cx.vm.set_status(error);
        return;
    }
    if let Some(coordinates) = coordinates {
        select(
            &cx,
            Place {
                id: 0,
                name: coordinates.label(),
                address: String::new(),
                coordinates: Some(coordinates),
            },
        );
        return;
    }
    let url = match geography::geocoding_url(&query) {
        Ok(url) => url,
        Err(error) => {
            cx.vm.set_status(error);
            return;
        }
    };
    let generation = state.generation.get().wrapping_add(1);
    state.generation.set(generation);
    state.busy.set(true);
    state.results.set(Vec::new());
    let epoch = cx.vm.document_epoch();
    let task = cx.get(&url, 64 * 1024);
    drop(Dispatcher::current_dispatcher().spawn_local(async move {
        let result = task
            .await
            .and_then(|bytes| geography::geocoding_places(&bytes));
        if cx.vm.document_epoch() != epoch || state.generation.get() != generation {
            return;
        }
        state.busy.set(false);
        match result {
            Ok(places) => {
                if places.is_empty() {
                    cx.vm.set_status(
                        "No cities found. Use search on map for an address or point of interest."
                            .into(),
                    );
                }
                state.results.set(places);
            }
            Err(error) => cx.vm.set_status(error),
        }
    }));
}
pub fn from_match(place: PlaceMatch) -> Place {
    Place {
        id: 0,
        name: place.name.chars().take(100).collect(),
        address: place.name,
        coordinates: Some(place.coordinates),
    }
}
pub fn external_search(cx: &AppContext) {
    let query = cx.session.query.get_untracked().trim().to_owned();
    match change(cx, |saved| saved.remember(&query)) {
        Ok(()) => cx.vm.request(HostRequest::OpenUrl {
            url: format!(
                "https://www.openstreetmap.org/search?query={}",
                phone_app_support::urls::encode(&query)
            ),
        }),
        Err(error) => cx.vm.set_status(error),
    }
}
pub fn edit(cx: AppContext, place: Place, shortcut: Option<bool>) {
    let ui = cx.session.clone();
    let epoch = cx.vm.document_epoch();
    let id = place.id;
    ui.form(
        if id == 0 { "save place" } else { "edit place" },
        vec![
            ("name", place.name),
            ("address", place.address),
            (
                "coordinates (optional)",
                place
                    .coordinates
                    .map(Coordinates::label)
                    .unwrap_or_default(),
            ),
        ],
        Rc::new(move |values| {
            let result = (|| {
                if cx.vm.document_epoch() != epoch {
                    return Err("The phone data changed. Reopen the place.".into());
                }
                let coordinates = value(&values, 2).trim();
                let place = Place {
                    id,
                    name: value(&values, 0).trim().into(),
                    address: value(&values, 1).trim().into(),
                    coordinates: if coordinates.is_empty() {
                        None
                    } else {
                        Some(Coordinates::parse(coordinates)?)
                    },
                };
                change(&cx, |saved| {
                    let id = saved.save(place)?;
                    if let Some(home) = shortcut {
                        if home {
                            saved.home = Some(id)
                        } else {
                            saved.work = Some(id)
                        }
                    }
                    Ok(id)
                })
            })();
            match result {
                Ok(id) => {
                    if let Ok(saved) = load(&cx) {
                        if let Some(place) = saved.places.into_iter().find(|place| place.id == id) {
                            select(&cx, place);
                        }
                    }
                }
                Err(error) => cx.session.form_error.set(error),
            }
        }),
    );
}
pub fn set_shortcut(cx: &AppContext, id: u32, home: bool) {
    let result = change(cx, |saved| {
        if !saved.places.iter().any(|p| p.id == id) {
            return Err("Save this place first.".into());
        }
        if home {
            saved.home = Some(id);
        } else {
            saved.work = Some(id);
        }
        Ok(())
    });
    match result {
        Ok(()) => {
            cx.session.menu.set(false);
            cx.vm.set_status(
                if home {
                    "Home updated."
                } else {
                    "Work updated."
                }
                .into(),
            );
        }
        Err(error) => cx.vm.set_status(error),
    }
}
pub fn directions(cx: &AppContext, destination: String) {
    let state = cx.session.state::<MapSession>();
    state.destination.set(destination);
    state.origin.set(String::new());
    state.directions.set(true);
    cx.session.menu.set(false);
}
pub fn launch_directions(cx: &AppContext) {
    let state = cx.session.state::<MapSession>();
    let result = load(cx).and_then(|saved| {
        model::directions_url(
            &state.origin.get_untracked(),
            &state.destination.get_untracked(),
            saved.travel_mode,
        )
    });
    match result {
        Ok(url) => cx.vm.request(HostRequest::OpenUrl { url }),
        Err(error) => cx.vm.set_status(error),
    }
}
fn value(values: &[String], index: usize) -> &str {
    values.get(index).map(String::as_str).unwrap_or("")
}
