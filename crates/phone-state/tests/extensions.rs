use phone_domain::{Action, AppId, PhoneData, Route, TileSize, TileTarget};
use phone_state::{PhoneViewModel, RecentApp};
use std::{cell::Cell, rc::Rc};

const APP: &str = "tests.extension";
fn model(data: PhoneData) -> (Rc<PhoneViewModel>, Rc<Cell<usize>>) {
    let saves = Rc::new(Cell::new(0));
    let count = saves.clone();
    let vm = PhoneViewModel::new(data, Rc::new(move |_| count.set(count.get() + 1)));
    vm.set_registered_apps([APP.to_string(), "tests.second".to_string()]);
    (vm, saves)
}
fn route(app: &str, path: &str) -> Route {
    Route::Extension {
        app: app.into(),
        path: path.into(),
    }
}
fn pinned(vm: &PhoneViewModel, path: &str) -> (u64, u64) {
    let data = vm.data();
    let link = data
        .app_links
        .iter()
        .find(|link| link.app == APP && link.path == path)
        .unwrap();
    let tile = data
        .all_tiles()
        .find(|tile| tile.target == Some(TileTarget::AppLink(link.id)))
        .unwrap();
    (link.id, tile.id)
}

#[test]
fn extension_links_preserve_identity_and_deep_links_through_folders_and_backup() {
    let (vm, saves) = model(PhoneData::demo());
    vm.pin_app(APP, "documents/42", "My document").unwrap();
    let (link, tile) = pinned(&vm, "documents/42");
    assert_ne!(link, tile);
    vm.set_tile_size(tile, TileSize::Wide);
    let original = vm.data().tile(tile).unwrap().clone();
    vm.create_folder(tile, 100, "work".into());
    let folder = vm.data().parent_folder(tile).unwrap();
    assert_eq!(vm.data().tile(tile), Some(&original));
    let count = saves.get();
    vm.pin_app(APP, "documents/42", "Duplicate").unwrap();
    assert_eq!(saves.get(), count);
    assert_eq!(vm.data().app_links.len(), 1);
    let json = vm.export_json().unwrap();
    let restored = PhoneData::from_json(&json).unwrap();
    assert_eq!(restored.tile(tile), Some(&original));
    assert_eq!(restored.parent_folder(tile), Some(folder));
    let (loaded, _) = model(restored);
    loaded.open_tile(tile);
    assert_eq!(loaded.route(), route(APP, "documents/42"));
    loaded.extract_folder_tile(folder, tile);
    assert!(loaded.data().tile(folder).is_none());
    assert_eq!(loaded.data().tile(tile), Some(&original));
    loaded.unpin(tile);
    assert!(loaded.data().app_links.is_empty());
    assert!(loaded.data().tile(100).is_some());
}

#[test]
fn disabling_removes_launch_points_and_journals_but_keeps_the_namespace() {
    let (vm, _) = model(PhoneData::demo());
    let saved = serde_json::json!({"version":1,"score":73});
    vm.set_app_data(APP, Some(saved.clone())).unwrap();
    vm.pin_app(APP, "", "Extension").unwrap();
    let (_, tile) = pinned(&vm, "");
    vm.create_folder(tile, 100, "apps".into());
    vm.navigate(Route::App(AppId::OneNote));
    vm.navigate(route(APP, "game/5"));
    vm.home();
    vm.resume_extension(APP);
    assert_eq!(vm.route(), route(APP, "game/5"));
    vm.set_extension_enabled(APP, false).unwrap();
    assert!(!vm.extension_enabled(APP));
    assert_ne!(vm.route(), route(APP, "game/5"));
    assert!(!vm
        .recent_entries()
        .contains(&RecentApp::Extension(APP.into())));
    assert!(vm.data().tile(tile).is_none());
    assert!(vm.data().tile(100).is_some());
    assert!(vm.data().app_links.is_empty());
    assert_eq!(vm.app_data(APP), Some(saved.clone()));
    let current = vm.route();
    vm.navigate(route(APP, ""));
    assert_eq!(vm.route(), current);
    assert!(vm.pin_app(APP, "", "Disabled").is_err());
    let (loaded, _) = model(PhoneData::from_json(&vm.export_json().unwrap()).unwrap());
    assert!(!loaded.extension_enabled(APP));
    assert_eq!(loaded.app_data(APP), Some(saved));
    loaded.set_extension_enabled(APP, true).unwrap();
    loaded.resume_extension(APP);
    assert_eq!(loaded.route(), route(APP, ""));
    assert!(loaded.data().app_links.is_empty());
}

#[test]
fn recent_entries_interleave_builtin_apps_and_extensions_and_close_resets_resume() {
    let (vm, _) = model(PhoneData::default());
    vm.navigate(route(APP, "first"));
    vm.navigate(Route::App(AppId::Calculator));
    vm.navigate(route("tests.second", "second"));
    assert_eq!(
        vm.recent_entries(),
        vec![
            RecentApp::Extension("tests.second".into()),
            RecentApp::Builtin(AppId::Calculator),
            RecentApp::Extension(APP.into())
        ]
    );
    vm.resume_extension(APP);
    assert_eq!(vm.route(), route(APP, "first"));
    vm.close_extension(APP);
    assert_eq!(vm.route(), Route::Start);
    assert!(!vm
        .recent_entries()
        .contains(&RecentApp::Extension(APP.into())));
    vm.resume_extension(APP);
    assert_eq!(vm.route(), route(APP, ""));
    vm.set_registered_apps(["tests.second".to_string()]);
    assert_eq!(vm.route(), Route::Start);
    assert!(!vm
        .recent_entries()
        .contains(&RecentApp::Extension(APP.into())));
}

#[test]
fn registry_unknowns_invalid_paths_and_exhausted_ids_never_partially_pin() {
    let (vm, saves) = model(PhoneData::default());
    let original = vm.data().clone();
    vm.dispatch(Action::PinApp {
        app: "missing.app".into(),
        path: String::new(),
        title: "Missing".into(),
    });
    vm.dispatch(Action::SetExtensionEnabled {
        app: "missing.app".into(),
        enabled: false,
    });
    vm.navigate(route("missing.app", ""));
    vm.navigate(route(APP, "invalid\npath"));
    assert_eq!(vm.route(), Route::Start);
    for (path, title) in [("bad\npath", "Valid"), ("", "  "), ("", "bad\ttitle")] {
        assert!(vm.pin_app(APP, path, title).is_err());
    }
    assert!(vm.pin_app(APP, &"x".repeat(2049), "Long").is_err());
    assert_eq!(*vm.data(), original);
    assert_eq!(saves.get(), 0);
    let mut exhausted = PhoneData::default();
    exhausted.next_id = u64::MAX - 2;
    let (vm, saves) = model(exhausted);
    let original = vm.data().clone();
    assert!(vm.pin_app(APP, "", "No room").is_err());
    assert_eq!(*vm.data(), original);
    assert_eq!(saves.get(), 0);
}
