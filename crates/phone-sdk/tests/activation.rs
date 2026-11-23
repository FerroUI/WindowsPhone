use ferroui_reactive::ObservableExt;
use ferroui_reactiveui::IReactiveNotifyPropertyChanged;
use ferroui_view::prelude::*;
use phone_controls::Icon;
use phone_domain::{AppId, PhoneData, Route};
use phone_sdk::{session::Session, AppContext, AppManifest, AppRegistration, AppRegistry};
use phone_state::PhoneViewModel;
use std::{cell::RefCell, rc::Rc};

#[test]
fn activation_wakes_an_already_active_view_without_a_route_change() {
    let registry = Rc::new(
        AppRegistry::new(vec![AppRegistration::new(
            AppManifest::new("tests.activation", "Activation", Icon::Note),
            |_| Element::Empty,
        )])
        .unwrap(),
    );
    let session = Rc::new(Session::new(0, registry));
    let received = Rc::new(RefCell::new(Vec::new()));
    let render = Owner::new_root();
    render.with(|| {
        let session = session.clone();
        let received = received.clone();
        effect(move || {
            if let Some(payload) = session.take_activation("tests.activation") {
                received.borrow_mut().push(payload);
            }
        });
    });
    session
        .queue_activation("tests.activation", serde_json::json!({"note": 1}))
        .unwrap();
    session
        .queue_activation("tests.activation", serde_json::json!({"note": 2}))
        .unwrap();
    assert_eq!(
        *received.borrow(),
        vec![
            serde_json::json!({"note": 1}),
            serde_json::json!({"note": 2})
        ]
    );
    assert!(session.take_activation("tests.activation").is_none());
    assert!(session
        .queue_activation("tests.missing", serde_json::Value::Null)
        .is_err());
    assert_eq!(received.borrow().len(), 2);
    render.dispose();
}

#[test]
fn context_typed_navigation_preserves_synchronous_children_and_document_restore() {
    #[derive(Default)]
    struct Received(RefCell<Vec<String>>);
    struct EffectOwner(Owner);
    impl Drop for EffectOwner {
        fn drop(&mut self) {
            self.0.dispose();
        }
    }

    let registry = Rc::new(
        AppRegistry::new(vec![AppRegistration::builtin(AppId::Settings, |_, _| {
            Element::Empty
        })
        .with_launch_path(|path| match path {
            "device" => Ok(serde_json::json!({"page": "device services"})),
            "communications" => Ok(serde_json::json!({"page": "phone+messages"})),
            "restore" => Ok(serde_json::json!({"restore": true})),
            _ => Err("Unknown typed Settings path.".into()),
        })
        .with_activation(|vm, session, payload| {
            if payload["restore"].as_bool() == Some(true) {
                vm.replace_data(PhoneData::default())?;
                vm.set_status("Restored document.".into());
            } else {
                let page = payload["page"].as_str().unwrap().to_owned();
                session
                    .state::<Received>()
                    .0
                    .borrow_mut()
                    .push(page.clone());
                vm.navigate(Route::Setting(page));
            }
            Ok(())
        })])
        .unwrap(),
    );
    let vm = PhoneViewModel::new(PhoneData::default(), Rc::new(|_| {}));
    registry.bind(&vm);
    let session = Rc::new(Session::new(vm.now(), registry));
    let context = AppContext {
        vm: vm.clone(),
        session: session.clone(),
        id: "phone.settings",
    };
    vm.navigate(Route::App(AppId::Settings));
    let owner = EffectOwner(Owner::new_root());
    owner.0.with(|| {
        // A standalone SDK host uses the same reactive bridge as the shell.
        // Queueing activation can run this effect before navigate() returns.
        let revision = signal(vm.revision());
        let observed = vm.clone();
        let subscription = vm
            .changed()
            .subscribe_next(move |_| revision.set(observed.revision()));
        on_cleanup(move || subscription.dispose());
        let vm = vm.clone();
        let session = session.clone();
        effect(move || {
            revision.get();
            let registry = session.registry();
            if let Some(app) = registry.for_route(&vm.route()) {
                if let Some(payload) = session.take_activation(app.manifest.id) {
                    registry
                        .activate(&vm, &session, app.manifest.id, payload)
                        .unwrap();
                }
            }
        });
    });
    for (path, page) in [
        ("device", "device services"),
        ("communications", "phone+messages"),
        ("communications", "phone+messages"),
    ] {
        context.navigate(path);
        assert_eq!(vm.route(), Route::Setting(page.into()));
    }
    assert_eq!(
        *session.state::<Received>().0.borrow(),
        vec!["device services", "phone+messages", "phone+messages"]
    );
    context.navigate("unknown");
    assert_eq!(vm.route(), Route::Setting("phone+messages".into()));
    assert_eq!(vm.status(), "Unknown typed Settings path.");
    assert_eq!(session.state::<Received>().0.borrow().len(), 3);

    let epoch = vm.document_epoch();
    context.navigate("restore");
    assert_eq!(vm.document_epoch(), epoch + 1);
    assert_eq!(vm.route(), Route::Start);
    assert_eq!(vm.status(), "Restored document.");
    assert!(session.take_activation("phone.settings").is_none());
}
