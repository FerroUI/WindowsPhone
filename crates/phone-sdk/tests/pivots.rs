use ferroui_view::prelude::Element;
use phone_controls::Icon;
use phone_domain::{AppId, Route};
use phone_sdk::{AppManifest, AppRegistration, AppRegistry, session::Session};
use std::rc::Rc;

fn fixture(app: AppRegistration) -> (Rc<AppRegistry>, Session) {
    let registry = Rc::new(AppRegistry::new(vec![app]).unwrap());
    let ui = Session::new(0, registry.clone());
    (registry, ui)
}

fn builtin() -> AppRegistration {
    AppRegistration::builtin(AppId::Settings, |_, _| Element::Empty)
}

fn root_only(route: &Route, _: &Session) -> usize {
    if *route == Route::App(AppId::Settings) {
        2
    } else {
        0
    }
}

#[test]
fn route_policy_overrides_legacy_including_zero_in_either_builder_order() {
    let legacy = |_: &Session| -> usize { panic!("an explicit route policy owns the decision") };
    for app in [
        builtin()
            .with_pivot_count(legacy)
            .with_route_pivot_count(root_only),
        builtin()
            .with_route_pivot_count(root_only)
            .with_pivot_count(legacy),
    ] {
        let (registry, ui) = fixture(app);
        ui.pivot.set(1);
        let root = Route::App(AppId::Settings);
        let detail = Route::Setting("brightness".into());
        assert_eq!(registry.pivot_count(&root, &ui), 2);
        assert!(registry.can_swipe(&root, &ui));
        assert_eq!(registry.pivot_count(&detail, &ui), 0);
        assert!(!registry.can_swipe(&detail, &ui));
        assert_eq!(
            ui.pivot.get_untracked(),
            1,
            "queries do not change hub selection"
        );
    }
}

#[test]
fn legacy_pivot_registration_preserves_its_session_only_route_behavior() {
    let (registry, ui) =
        fixture(builtin().with_pivot_count(|ui| if ui.menu.get_untracked() { 0 } else { 4 }));
    for route in [Route::App(AppId::Settings), Route::Setting("about".into())] {
        assert_eq!(registry.pivot_count(&route, &ui), 4);
        assert!(registry.can_swipe(&route, &ui));
    }
    ui.menu.set(true);
    for route in [Route::App(AppId::Settings), Route::Setting("about".into())] {
        assert_eq!(registry.pivot_count(&route, &ui), 0);
        assert!(!registry.can_swipe(&route, &ui));
    }
    assert_eq!(registry.pivot_count(&Route::App(AppId::People), &ui), 0);
    assert!(!registry.can_swipe(&Route::Start, &ui));
    let (registry, ui) = fixture(builtin());
    assert_eq!(registry.pivot_count(&Route::App(AppId::Settings), &ui), 0);
    assert!(!registry.can_swipe(&Route::Setting("about".into()), &ui));
}

#[test]
fn extension_policy_receives_its_exact_path_and_current_session() {
    let app = AppRegistration::new(
        AppManifest::new("tests.route-pivot", "Route pivot", Icon::Note),
        |_| Element::Empty,
    )
    .with_route_pivot_count(|route, ui| {
        let Route::Extension { app, path } = route else {
            panic!("extension route expected")
        };
        assert_eq!(app, "tests.route-pivot");
        if ui.menu.get_untracked() {
            return 0;
        }
        match path.as_str() {
            "" => 3,
            "collection" => 2,
            _ => 0,
        }
    });
    let (registry, ui) = fixture(app);
    let route = |path: &str| Route::Extension {
        app: "tests.route-pivot".into(),
        path: path.into(),
    };
    for (path, count) in [("", 3), ("collection", 2), ("collection/edit/42", 0)] {
        assert_eq!(registry.pivot_count(&route(path), &ui), count);
        assert_eq!(registry.can_swipe(&route(path), &ui), count > 0);
    }
    ui.menu.set(true);
    assert_eq!(registry.pivot_count(&route(""), &ui), 0);
    assert!(!registry.can_swipe(&route("collection"), &ui));
    assert_eq!(
        registry.pivot_count(
            &Route::Extension {
                app: "tests.unregistered".into(),
                path: "".into(),
            },
            &ui
        ),
        0
    );
}

#[test]
fn explicit_custom_swipes_remain_independent_of_pivot_policy() {
    let (registry, ui) = fixture(
        builtin()
            .with_route_pivot_count(|_, _| 0)
            .with_can_swipe(|ui| !ui.menu.get_untracked()),
    );
    let route = Route::Setting("custom-canvas".into());
    assert_eq!(registry.pivot_count(&route, &ui), 0);
    assert!(registry.can_swipe(&route, &ui));
    ui.menu.set(true);
    assert!(!registry.can_swipe(&route, &ui));
}
