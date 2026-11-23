use ferroui_view::prelude::Element;
use phone_controls::Icon;
use phone_domain::{PhoneData, Route, TileSize};
use phone_sdk::{
    session::Session, AppContext, AppLifecycle, AppManifest, AppRegistration, AppRegistry,
};
use phone_state::PhoneViewModel;
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

const APP: &str = "tests.contract";

#[test]
fn builtin_secondary_paths_are_parsed_before_navigation_or_persistence() {
    let registration = AppRegistration::builtin(phone_domain::AppId::Maps, |_, _| Element::Empty)
        .with_launch_path(|path| {
            let id = path
                .strip_prefix("place/")
                .ok_or("Unknown path")?
                .parse::<u32>()
                .map_err(|_| "Invalid place")?;
            if id == 0 {
                return Err("Invalid place".into());
            }
            Ok(serde_json::json!({"place":id}))
        })
        .with_activation(|_, _, _| Ok(()));
    let saves = Rc::new(Cell::new(0));
    let writes = saves.clone();
    let vm = PhoneViewModel::new(
        PhoneData::demo(),
        Rc::new(move |_| writes.set(writes.get() + 1)),
    );
    let registry = Rc::new(AppRegistry::new(vec![registration]).unwrap());
    registry.bind(&vm);
    let session = Rc::new(Session::new(0, registry));
    let cx = AppContext {
        vm: vm.clone(),
        session: session.clone(),
        id: "phone.maps",
    };
    let before = vm.data().to_json().unwrap();
    assert!(cx.pin("place/0", "Missing").is_err());
    assert!(cx.pin("arbitrary/path", "Wrong").is_err());
    cx.navigate("place/not-a-number");
    assert_eq!(vm.route(), Route::Start);
    assert!(session.take_activation("phone.maps").is_none());
    assert_eq!(vm.data().to_json().unwrap(), before);
    assert_eq!(saves.get(), 0);
    cx.pin("place/42", "Saved place").unwrap();
    assert!(vm
        .data()
        .app_links
        .iter()
        .any(|v| v.app == "phone.maps" && v.path == "place/42"));
    cx.navigate("place/42");
    assert_eq!(vm.route(), Route::App(phone_domain::AppId::Maps));
    assert_eq!(
        session.take_activation("phone.maps"),
        Some(serde_json::json!({"place":42}))
    );
    assert_eq!(saves.get(), 1);
}
#[derive(Default)]
struct Observed {
    activations: RefCell<Vec<serde_json::Value>>,
    tiles: RefCell<Vec<(String, TileSize, f64, f64, i64)>>,
    lifecycle: RefCell<Vec<AppLifecycle>>,
    resets: Cell<u32>,
}
fn model() -> (Rc<PhoneViewModel>, Rc<Session>) {
    let app = AppRegistration::new(
        AppManifest::new(APP, "Contract", Icon::Note)
            .capabilities(&[phone_sdk::Capability::Clipboard]),
        |_| Element::Empty,
    )
    .with_activation(|_, ui, payload| {
        ui.state::<Observed>()
            .activations
            .borrow_mut()
            .push(payload);
        Ok(())
    })
    .with_tile(|cx| {
        cx.app
            .session
            .state::<Observed>()
            .tiles
            .borrow_mut()
            .push((cx.path, cx.size, cx.width, cx.height, cx.now));
        Element::Empty
    })
    .with_reset(|ui| {
        let state = ui.state::<Observed>();
        state.resets.set(state.resets.get() + 1);
    })
    .with_lifecycle(|cx, event| {
        cx.session
            .state::<Observed>()
            .lifecycle
            .borrow_mut()
            .push(event)
    });
    let registry = Rc::new(AppRegistry::new(vec![app]).unwrap());
    let vm = PhoneViewModel::new(PhoneData::default(), Rc::new(|_| {}));
    vm.set_registered_apps([APP.to_owned()]);
    let session = Rc::new(Session::new(0, registry));
    (vm, session)
}

#[test]
fn data_activation_rejects_missing_inactive_disabled_and_lossy_payloads() {
    let (vm, ui) = model();
    let registry = ui.registry();
    let payload = serde_json::json!({"document":42});
    assert!(registry
        .activate(&vm, &ui, "missing.app", payload.clone())
        .is_err());
    assert!(registry.activate(&vm, &ui, APP, payload.clone()).is_err());
    vm.navigate(Route::Extension {
        app: APP.into(),
        path: "details".into(),
    });
    registry.activate(&vm, &ui, APP, payload.clone()).unwrap();
    assert_eq!(
        *ui.state::<Observed>().activations.borrow(),
        vec![payload.clone()]
    );
    assert!(registry
        .activate(&vm, &ui, APP, serde_json::json!(u64::MAX))
        .is_err());
    vm.set_extension_enabled(APP, false).unwrap();
    assert!(registry.activate(&vm, &ui, APP, payload).is_err());
    assert_eq!(ui.state::<Observed>().activations.borrow().len(), 1);
}

#[test]
fn tile_rendering_and_context_pinning_use_the_extension_identity_and_path() {
    let (vm, ui) = model();
    let registry = ui.registry();
    let cx = AppContext {
        vm: vm.clone(),
        session: ui.clone(),
        id: APP,
    };
    cx.pin("document/42", "Document 42").unwrap();
    assert_eq!(vm.data().app_links[0].app, APP);
    assert_eq!(vm.data().app_links[0].path, "document/42");
    assert!(registry
        .tile(
            APP,
            &vm,
            &ui,
            TileSize::Wide,
            360.0,
            180.0,
            "document/42".into(),
            1234
        )
        .is_some());
    assert_eq!(
        *ui.state::<Observed>().tiles.borrow(),
        vec![("document/42".into(), TileSize::Wide, 360.0, 180.0, 1234)]
    );
    assert!(registry
        .tile(
            "missing.app",
            &vm,
            &ui,
            TileSize::Small,
            80.0,
            80.0,
            String::new(),
            0
        )
        .is_none());
    vm.set_extension_enabled(APP, false).unwrap();
    assert!(cx.pin("", "Disabled").is_err());
}

#[test]
fn close_and_reset_callbacks_are_distinct_and_do_not_erase_persistent_data() {
    let (vm, ui) = model();
    let registry = ui.registry();
    vm.set_app_data(APP, Some(serde_json::json!({"saved":true})))
        .unwrap();
    registry.close(&vm, &ui, APP);
    assert_eq!(ui.state::<Observed>().resets.get(), 0);
    registry.reset_app(&vm, &ui, APP);
    assert_eq!(ui.state::<Observed>().resets.get(), 1);
    assert_eq!(
        *ui.state::<Observed>().lifecycle.borrow(),
        vec![AppLifecycle::Closing, AppLifecycle::Reset]
    );
    assert_eq!(vm.app_data(APP), Some(serde_json::json!({"saved":true})));
}

#[test]
fn retained_context_rechecks_availability_and_rejects_invalid_navigation() {
    use phone_state::{HostRequest, HostServices};
    let (vm, session) = model();
    let calls = Rc::new(Cell::new(0));
    let count = calls.clone();
    vm.set_services(HostServices {
        request: Rc::new(move |_| count.set(count.get() + 1)),
    });
    let context = AppContext {
        vm: vm.clone(),
        session,
        id: APP,
    };
    context
        .request(HostRequest::CopyText { text: "one".into() })
        .unwrap();
    vm.set_extension_enabled(APP, false).unwrap();
    assert!(context
        .request(HostRequest::CopyText { text: "two".into() })
        .is_err());
    assert_eq!(calls.get(), 1);
    context.navigate("document/3");
    assert_eq!(vm.route(), Route::Start);
    vm.set_extension_enabled(APP, true).unwrap();
    context.navigate("bad\npath");
    assert_eq!(vm.route(), Route::Start);
    context.navigate("document/3");
    assert_eq!(
        vm.route(),
        Route::Extension {
            app: APP.into(),
            path: "document/3".into()
        }
    );
}
