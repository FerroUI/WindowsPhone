use crate::state::MapSession;
use phone_sdk::{session::Session, AppLifecycle, AppRegistration};
use std::rc::Rc;
pub fn registration() -> AppRegistration {
    AppRegistration::builtin(phone_domain::AppId::Maps, crate::view)
        .with_launch_path(|path| Ok(serde_json::json!({"place":place_id(path)?})))
        .with_activation(|vm, ui, value| {
            #[derive(serde::Deserialize)]
            #[serde(deny_unknown_fields)]
            struct Activation {
                place: u32,
            }
            let activation: Activation =
                serde_json::from_value(value).map_err(|_| "Invalid Maps activation.".to_owned())?;
            let cx = crate::commands::context(vm.clone(), ui.clone());
            let saved = crate::commands::load(&cx)?;
            let place = saved
                .places
                .into_iter()
                .find(|place| place.id == activation.place && activation.place != 0)
                .ok_or("This pinned place is no longer saved.")?;
            crate::commands::select(&cx, place);
            Ok(())
        })
        .with_tile(crate::tile::view)
        .with_tick(|vm, ui, now| {
            let cx = crate::commands::context(vm.clone(), ui.clone());
            crate::viewport::tick(&cx);
            crate::commands::tick_location(&cx, now);
        })
        .with_lifecycle(|cx, event| match event {
            AppLifecycle::Activated | AppLifecycle::Resumed => {
                crate::viewport::suspend(&cx.session, false)
            }
            AppLifecycle::Suspended | AppLifecycle::Deactivated | AppLifecycle::Closing => {
                cancel(&cx.session);
                crate::viewport::suspend(&cx.session, true);
            }
            AppLifecycle::Reset => {}
        })
        .with_pivot_count(|ui| {
            let state = ui.state::<MapSession>();
            if state.selection.get_untracked().is_some() || state.directions.get_untracked() {
                0
            } else {
                3
            }
        })
        .with_back(|_, ui| {
            let state = ui.state::<MapSession>();
            if state.directions.get_untracked() {
                state.directions.set(false);
                true
            } else if state.selection.get_untracked().is_some() {
                state.selection.set(None);
                true
            } else {
                false
            }
        })
        .with_overlay(|ui| {
            let state = ui.state::<MapSession>();
            state.selection.get_untracked().is_some() || state.directions.get_untracked()
        })
        .with_deactivate(cancel)
        .with_reset(|ui| {
            cancel(ui);
            crate::viewport::reset(ui);
            let state = ui.state::<MapSession>();
            state.selection.set(None);
            state.directions.set(false);
            state.results.set(Vec::new());
            state.origin.set(String::new());
            state.destination.set(String::new());
        })
}
fn cancel(ui: &Rc<Session>) {
    crate::commands::cancel_search(ui);
    let state = ui.state::<MapSession>();
    state.locate.borrow_mut().take();
}
pub(crate) fn place_id(path: &str) -> Result<u32, String> {
    let text = path
        .strip_prefix("place/")
        .ok_or("Unknown Maps launch path.")?;
    let id = text
        .parse::<u32>()
        .map_err(|_| "Invalid saved place identifier.")?;
    if id == 0 || id == u32::MAX || id.to_string() != text {
        return Err("Invalid saved place identifier.".into());
    }
    Ok(id)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn saved_place_routes_require_canonical_nonzero_identity() {
        assert_eq!(place_id("place/7").unwrap(), 7);
        for path in [
            "place/0",
            "place/01",
            "place/+1",
            "place/1/next",
            "place/4294967295",
            "geo:1,2",
        ] {
            assert!(place_id(path).is_err());
        }
    }
}
