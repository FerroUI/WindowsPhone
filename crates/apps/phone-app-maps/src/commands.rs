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
    cancel_search(&cx.session);
    cx.session.menu.set(false);
    let state = cx.session.state::<MapSession>();
    state.locate.borrow_mut().take();
    state.directions.set(false);
    if let Some(coordinates) = place.coordinates {
        crate::viewport::focus(cx, coordinates);
    }
    state.selection.set(Some(place));
}
pub fn cancel_search(ui: &Session) {
    let state = ui.state::<MapSession>();
    if let Some(abort) = state.search_abort.borrow_mut().take() {
        abort();
    }
    state
        .generation
        .set(state.generation.get().saturating_add(1));
    state.busy.set(false);
}
pub fn search(cx: AppContext) {
    let state = cx.session.state::<MapSession>();
    state.locate.borrow_mut().take();
    cancel_search(&cx.session);
    state.results.set(Vec::new());
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
    let Some(generation) = state.generation.get().checked_add(1) else {
        cx.vm
            .set_status("The map search session is exhausted. Restart Maps.".into());
        return;
    };
    state.generation.set(generation);
    state.busy.set(true);
    state.results.set(Vec::new());
    let epoch = cx.vm.document_epoch();
    let task = cx.get(&url, 64 * 1024);
    let searched = query;
    let weak_vm = Rc::downgrade(&cx.vm);
    let weak_ui = Rc::downgrade(&cx.session);
    let weak_state = Rc::downgrade(&state);
    let task = Dispatcher::current_dispatcher().spawn_local(async move {
        let result = task
            .await
            .and_then(|bytes| geography::geocoding_places(&bytes));
        let (Some(vm), Some(ui), Some(state)) =
            (weak_vm.upgrade(), weak_ui.upgrade(), weak_state.upgrade())
        else {
            return;
        };
        if vm.document_epoch() != epoch
            || state.generation.get() != generation
            || vm.route() != phone_domain::Route::App(phone_domain::AppId::Maps)
        {
            return;
        }
        state.search_abort.borrow_mut().take();
        state.busy.set(false);
        if ui.query.get_untracked().trim() != searched {
            return;
        }
        match result {
            Ok(places) => {
                if places.is_empty() {
                    vm.set_status(
                        "No cities found. Use search on map for an address or point of interest."
                            .into(),
                    );
                }
                state.results.set(places);
            }
            Err(error) => vm.set_status(error),
        }
        drop(ui);
    });
    let abort = task.abort_handle();
    drop(task);
    *state.search_abort.borrow_mut() = Some(Box::new(move || abort.abort()));
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
        Ok(())
            if cx
                .vm
                .platform_state()
                .supports(phone_sdk::host::platform::HostFeature::Maps) =>
        {
            cx.vm
                .request_device(phone_sdk::host::platform::DeviceRequest::OpenMap {
                    query,
                    latitude: None,
                    longitude: None,
                    destination: false,
                    mode: phone_sdk::host::platform::MapTravelMode::Driving,
                })
        }
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
    cancel_search(&cx.session);
    let state = cx.session.state::<MapSession>();
    state.locate.borrow_mut().take();
    state.destination.set(destination);
    state.origin.set(String::new());
    state.directions.set(true);
    cx.session.menu.set(false);
}
pub fn pin(cx: &AppContext, place: &Place) {
    let result = (|| {
        let saved = load(cx)?;
        let place = saved
            .places
            .iter()
            .find(|saved| saved.id == place.id && place.id != 0)
            .ok_or("Save this place before pinning it.")?;
        cx.pin(
            &format!("place/{}", place.id),
            &place.name.chars().take(80).collect::<String>(),
        )
    })();
    match result {
        Ok(()) => {
            cx.session.menu.set(false);
            cx.vm.set_status("Pinned place to Start.".into());
        }
        Err(error) => cx.vm.set_status(error),
    }
}
pub fn remove(cx: &AppContext, id: u32) -> Result<(), String> {
    change(cx, |saved| {
        saved.remove(id);
        Ok(())
    })?;
    let data = cx.vm.data();
    let path = format!("place/{id}");
    let links = data
        .app_links
        .iter()
        .filter(|link| link.app == cx.id && link.path == path)
        .map(|link| link.id)
        .collect::<std::collections::BTreeSet<_>>();
    let tiles=data.all_tiles().filter(|tile|matches!(tile.target,Some(phone_domain::TileTarget::AppLink(link))if links.contains(&link))).map(|tile|tile.id).collect::<Vec<_>>();
    drop(data);
    for tile in tiles {
        cx.vm.unpin(tile);
    }
    Ok(())
}
pub fn locate(cx: &AppContext) {
    let state = cx.session.state::<MapSession>();
    if state.locate.borrow().is_some() {
        cx.vm
            .set_status("A location request is already in progress.".into());
        return;
    }
    *state.locate.borrow_mut() = Some(crate::state::Locate {
        revision: cx.vm.device_state().location_revision,
        started: cx.vm.now(),
        epoch: cx.vm.document_epoch(),
    });
    if let Err(error) = cx.request(HostRequest::Platform {
        request: phone_sdk::host::platform::PlatformRequest::Locate,
    }) {
        state.locate.borrow_mut().take();
        cx.vm.set_status(error);
    }
}
pub fn cancel_location(cx: &AppContext) {
    cx.session.state::<MapSession>().locate.borrow_mut().take();
}
pub fn tick_location(cx: &AppContext, now: i64) {
    let state = cx.session.state::<MapSession>();
    let pending = *state.locate.borrow();
    let Some(pending) = pending else {
        return;
    };
    if pending.epoch != cx.vm.document_epoch() {
        state.locate.borrow_mut().take();
        return;
    }
    let observed = cx.vm.device_state();
    if observed.location_revision != pending.revision {
        state.locate.borrow_mut().take();
        if let Some((latitude, longitude)) = observed.location {
            if let Ok(coordinates) = Coordinates::new(latitude, longitude) {
                select(
                    cx,
                    Place {
                        id: 0,
                        name: "current location".into(),
                        address: String::new(),
                        coordinates: Some(coordinates),
                    },
                );
            }
        }
    } else if now.saturating_sub(pending.started) > 30 {
        state.locate.borrow_mut().take();
        cx.vm
            .set_status("Location was not returned. Check location access and try again.".into());
    }
}
pub fn launch_directions(cx: &AppContext) {
    let state = cx.session.state::<MapSession>();
    if state.origin.get_untracked().trim().is_empty()
        && cx
            .vm
            .platform_state()
            .supports(phone_sdk::host::platform::HostFeature::Maps)
    {
        let saved = match load(cx) {
            Ok(value) => value,
            Err(error) => {
                cx.vm.set_status(error);
                return;
            }
        };
        let query = state.destination.get_untracked();
        let coordinates = Coordinates::parse(&query).ok();
        cx.vm
            .request_device(phone_sdk::host::platform::DeviceRequest::OpenMap {
                query,
                latitude: coordinates.map(|v| v.latitude),
                longitude: coordinates.map(|v| v.longitude),
                destination: true,
                mode: travel_mode(saved.travel_mode),
            });
        return;
    }
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
pub fn open_map(cx: &AppContext, place: &Place) {
    if cx
        .vm
        .platform_state()
        .supports(phone_sdk::host::platform::HostFeature::Maps)
    {
        cx.vm
            .request_device(phone_sdk::host::platform::DeviceRequest::OpenMap {
                query: if place.address.is_empty() {
                    place.name.clone()
                } else {
                    place.address.clone()
                },
                latitude: place.coordinates.map(|v| v.latitude),
                longitude: place.coordinates.map(|v| v.longitude),
                destination: false,
                mode: phone_sdk::host::platform::MapTravelMode::Driving,
            });
    } else {
        match place.map_url() {
            Ok(url) => cx.vm.request(HostRequest::OpenUrl { url }),
            Err(error) => cx.vm.set_status(error),
        }
    }
}
fn travel_mode(value: model::TravelMode) -> phone_sdk::host::platform::MapTravelMode {
    use phone_sdk::host::platform::MapTravelMode as M;
    match value {
        model::TravelMode::Driving => M::Driving,
        model::TravelMode::Walking => M::Walking,
        model::TravelMode::Bicycling => M::Cycling,
        model::TravelMode::Transit => M::Transit,
    }
}
fn value(values: &[String], index: usize) -> &str {
    values.get(index).map(String::as_str).unwrap_or("")
}

#[cfg(test)]
mod tests {
    use super::*;
    use phone_domain::{AppId, PhoneData, Route, TileTarget};
    use phone_sdk::{AppLifecycle, AppRegistry};
    use std::cell::Cell;

    fn fixture() -> AppContext {
        let vm = PhoneViewModel::new(PhoneData::default(), Rc::new(|_| {}));
        let registry = Rc::new(AppRegistry::new(vec![crate::registration()]).unwrap());
        registry.bind(&vm);
        let ui = Rc::new(Session::new(vm.now(), registry));
        vm.navigate(Route::App(AppId::Maps));
        context(vm, ui)
    }
    fn place(name: &str) -> Place {
        Place {
            id: 0,
            name: name.into(),
            address: String::new(),
            coordinates: Some(Coordinates::new(50.091, 18.219).unwrap()),
        }
    }

    #[test]
    fn saved_place_pins_reactivate_restored_content_and_deletion_removes_nested_tiles() {
        let cx = fixture();
        pin(&cx, &place("Unsaved"));
        assert!(cx.vm.data().app_links.is_empty());
        let first = change(&cx, |saved| saved.save(place("Home"))).unwrap();
        let second = change(&cx, |saved| saved.save(place("Work"))).unwrap();
        let saved = load(&cx).unwrap();
        for item in &saved.places {
            pin(&cx, item);
        }
        let first_link = cx
            .vm
            .data()
            .app_links
            .iter()
            .find(|link| link.path == format!("place/{first}"))
            .unwrap()
            .id;
        let second_link = cx
            .vm
            .data()
            .app_links
            .iter()
            .find(|link| link.path == format!("place/{second}"))
            .unwrap()
            .id;
        let tile_for = |id| {
            cx.vm
                .data()
                .all_tiles()
                .find(|tile| tile.target == Some(TileTarget::AppLink(id)))
                .unwrap()
                .id
        };
        cx.vm
            .create_folder(tile_for(first_link), tile_for(second_link), "Places".into());
        let exported = cx.vm.export_json().unwrap();
        cx.vm.import_json(&exported).unwrap();
        cx.vm.navigate(Route::App(AppId::Maps));
        let registry = cx.session.registry();
        let payload = registry
            .by_id(cx.id)
            .unwrap()
            .launch_payload(&format!("place/{first}"))
            .unwrap()
            .unwrap();
        registry
            .activate(&cx.vm, &cx.session, cx.id, payload.clone())
            .unwrap();
        assert_eq!(
            cx.session
                .state::<MapSession>()
                .selection
                .get_untracked()
                .unwrap()
                .name,
            "Home"
        );
        remove(&cx, first).unwrap();
        assert!(!cx
            .vm
            .data()
            .app_links
            .iter()
            .any(|link| link.id == first_link));
        assert!(!cx
            .vm
            .data()
            .all_tiles()
            .any(|tile| tile.target == Some(TileTarget::AppLink(first_link))));
        assert!(cx
            .vm
            .data()
            .all_tiles()
            .any(|tile| tile.target == Some(TileTarget::AppLink(second_link))));
        assert!(registry
            .activate(&cx.vm, &cx.session, cx.id, payload)
            .is_err());
        assert!(registry
            .activate(
                &cx.vm,
                &cx.session,
                cx.id,
                serde_json::json!({"place":second,"extra":true})
            )
            .is_err());
    }

    #[test]
    fn location_is_explicit_fresh_one_shot_and_does_not_persist_camera_or_gps() {
        let cx = fixture();
        cx.vm.set_device_location(1.0, 2.0).unwrap();
        let before = cx.vm.export_json().unwrap();
        locate(&cx);
        tick_location(&cx, cx.vm.now());
        assert!(
            cx.session
                .state::<MapSession>()
                .selection
                .get_untracked()
                .is_none(),
            "Cached coordinates cannot acknowledge a fresh locate request"
        );
        cx.vm
            .set_device_response(phone_sdk::host::platform::DeviceResponse::Notifications(
                Vec::new(),
            ))
            .unwrap();
        tick_location(&cx, cx.vm.now());
        assert!(
            cx.session
                .state::<MapSession>()
                .selection
                .get_untracked()
                .is_none(),
            "Unrelated observations are not location results"
        );
        cx.vm.set_device_location(50.091, 18.219).unwrap();
        tick_location(&cx, cx.vm.now());
        let selected = cx
            .session
            .state::<MapSession>()
            .selection
            .get_untracked()
            .unwrap();
        assert_eq!(selected.id, 0);
        assert_eq!(
            selected.coordinates.unwrap(),
            Coordinates::new(50.091, 18.219).unwrap()
        );
        assert!(cx.session.state::<MapSession>().locate.borrow().is_none());
        cx.vm.set_device_location(-20.0, 30.0).unwrap();
        tick_location(&cx, cx.vm.now());
        assert_eq!(
            cx.session.state::<MapSession>().selection.get_untracked(),
            Some(selected)
        );
        assert_eq!(
            cx.vm.export_json().unwrap(),
            before,
            "GPS and camera state stay outside persistent phone data until Save place"
        );
    }

    #[test]
    fn navigation_restore_manual_interaction_and_timeout_cancel_location_intent() {
        let cx = fixture();
        let state = cx.session.state::<MapSession>();
        locate(&cx);
        cancel_location(&cx); // The input handlers cancel before changing camera.
        cx.vm.set_device_location(1.0, 2.0).unwrap();
        tick_location(&cx, cx.vm.now());
        assert!(state.selection.get_untracked().is_none());
        locate(&cx);
        let cancelled = Rc::new(Cell::new(0));
        let capture = cancelled.clone();
        *state.search_abort.borrow_mut() = Some(Box::new(move || capture.set(capture.get() + 1)));
        cx.session
            .registry()
            .lifecycle(&cx.vm, &cx.session, AppLifecycle::Suspended);
        assert_eq!(cancelled.get(), 1);
        assert!(state.locate.borrow().is_none());
        cx.session
            .registry()
            .lifecycle(&cx.vm, &cx.session, AppLifecycle::Resumed);
        locate(&cx);
        let now = cx.vm.now();
        tick_location(&cx, now + 31);
        assert!(state.locate.borrow().is_none());
        assert!(cx.vm.status().contains("not returned"));
        locate(&cx);
        cx.vm.import_json(&cx.vm.export_json().unwrap()).unwrap();
        cx.vm.set_device_location(40.0, 40.0).unwrap();
        tick_location(&cx, cx.vm.now());
        assert!(state.locate.borrow().is_none());
        assert!(state.selection.get_untracked().is_none());
        cx.vm.navigate(Route::App(AppId::Maps));
        locate(&cx);
        cx.vm.home();
        cx.session.registry().route_changed(
            &cx.vm,
            &cx.session,
            &Route::App(AppId::Maps),
            &Route::Start,
        );
        assert!(state.locate.borrow().is_none());
    }
}
