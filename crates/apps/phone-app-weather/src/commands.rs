use crate::{
    model::{self, Place, Saved},
    WeatherState,
};
use ferroui_base::threading::Dispatcher;
use phone_sdk::{session::Session, AppContext, AppStorage};
use phone_state::PhoneViewModel;
use std::rc::Rc;
pub fn context(vm: Rc<PhoneViewModel>, session: Rc<Session>) -> AppContext {
    AppContext {
        vm,
        session,
        id: "phone.weather",
    }
}
pub fn load(cx: &AppContext) -> Result<Saved, String> {
    let saved = cx.storage().read::<Saved>()?;
    saved.validate()?;
    Ok(saved)
}
pub fn change(cx: &AppContext, apply: impl FnOnce(&mut Saved) -> Result<(), String>) -> bool {
    let result = load(cx).and_then(|mut saved| {
        apply(&mut saved)?;
        saved.validate()?;
        cx.storage().write(&saved)
    });
    if let Err(error) = result {
        cx.vm.set_status(error);
        false
    } else {
        true
    }
}
pub fn search(cx: AppContext) {
    let state = cx.session.state::<WeatherState>();
    let query = state.query.get_untracked().trim().to_owned();
    let url = match phone_app_support::geography::geocoding_url(&query) {
        Ok(url) => url,
        Err(error) => {
            cx.vm.set_status(error);
            return;
        }
    };
    let task = cx.get(&url, 64 * 1024);
    let epoch = cx.vm.document_epoch();
    let generation = state.generation.get().wrapping_add(1);
    state.generation.set(generation);
    state.busy.set(true);
    state.results.set(Vec::new());
    drop(Dispatcher::current_dispatcher().spawn_local(async move {
        let result = task.await.and_then(|bytes| model::places(&bytes));
        if cx.vm.document_epoch() != epoch || state.generation.get() != generation {
            return;
        }
        state.busy.set(false);
        match result {
            Ok(places) => {
                if places.is_empty() {
                    cx.vm.set_status(
                        "No cities found. Try a nearby city or a different spelling.".into(),
                    );
                }
                state.results.set(places);
            }
            Err(error) => cx.vm.set_status(error),
        }
    }));
}
pub fn add(cx: AppContext, place: Place) {
    if change(&cx, |saved| saved.add(place.clone())) {
        cx.session.state::<WeatherState>().results.set(Vec::new());
        cx.session.pivot.set(0);
        refresh(cx, place);
    }
}
pub fn refresh(cx: AppContext, place: Place) {
    let state = cx.session.state::<WeatherState>();
    let epoch = cx.vm.document_epoch();
    let generation = state.generation.get().wrapping_add(1);
    state.generation.set(generation);
    state.busy.set(true);
    let task = cx.get(&place.forecast_url(), 128 * 1024);
    drop(Dispatcher::current_dispatcher().spawn_local(async move {
        let result = task
            .await
            .and_then(|bytes| model::forecast(&bytes, cx.vm.now()));
        if cx.vm.document_epoch() != epoch || state.generation.get() != generation {
            return;
        }
        state.busy.set(false);
        match result {
            Ok(forecast) => {
                change(&cx, |saved| {
                    let city = saved
                        .cities
                        .iter_mut()
                        .find(|city| city.place.id == place.id)
                        .ok_or("This city was removed while weather was loading.")?;
                    city.forecast = Some(forecast.clone());
                    Ok(())
                });
            }
            Err(error) => cx.vm.set_status(error),
        }
    }));
}
pub fn read_saved(vm: Rc<PhoneViewModel>) -> Option<Saved> {
    let saved = AppStorage::new(vm, "phone.weather").read::<Saved>().ok()?;
    saved.validate().ok()?;
    Some(saved)
}
