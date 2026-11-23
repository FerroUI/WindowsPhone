use ferroui_view::prelude::*;
use phone_controls::Icon;
use phone_domain::{AppId, PhoneData, Route};
use phone_sdk::{
    session::Session, AppContext, AppLifecycle, AppManifest, AppRegistration, AppRegistry,
    Capability,
};
use phone_state::{HostRequest, HostServices, PhoneViewModel};
use std::{cell::RefCell, rc::Rc};

#[derive(Default)]
struct Events(RefCell<Vec<String>>);

struct OwnedState {
    count: Signal<u32>,
    lifetime: Signal<Rc<()>>,
}
impl Default for OwnedState {
    fn default() -> Self {
        Self {
            count: signal(7),
            lifetime: signal(Rc::new(())),
        }
    }
}

fn empty(_: AppContext) -> Element {
    Element::Empty
}
fn builtin(_: Rc<PhoneViewModel>, _: Rc<Session>) -> Element {
    Element::Empty
}
fn registration(id: &'static str) -> AppRegistration {
    AppRegistration::new(AppManifest::new(id, id, Icon::Note), empty)
}
fn route(id: &str, path: &str) -> Route {
    Route::Extension {
        app: id.into(),
        path: path.into(),
    }
}
fn fixture(entries: Vec<AppRegistration>) -> (Rc<AppRegistry>, Rc<Session>, Rc<PhoneViewModel>) {
    let registry = Rc::new(AppRegistry::new(entries).unwrap());
    let vm = PhoneViewModel::new(PhoneData::default(), Rc::new(|_| {}));
    vm.set_registered_apps(registry.manifests().iter().map(|app| app.id.to_string()));
    let session = Rc::new(Session::new(0, registry.clone()));
    (registry, session, vm)
}

#[test]
fn typed_state_survives_the_render_owner_and_is_disposed_with_its_session() {
    let (_, session, _) = fixture(vec![registration("tests.owner")]);
    let render = Owner::new_root();
    let state = render.with(|| session.state::<OwnedState>());
    let witness = Rc::downgrade(&state.lifetime.get_untracked());
    state.count.set(42);
    render.dispose();
    assert_eq!(state.count.get_untracked(), 42);
    let again = session.state::<OwnedState>();
    assert!(Rc::ptr_eq(&state, &again));
    assert!(witness.upgrade().is_some());
    drop(session);
    // Holding the typed state object does not leak the reactive nodes after
    // the session that owns them ends.
    assert!(witness.upgrade().is_none());
}

#[test]
fn catalog_rejects_duplicate_identity_and_duplicate_builtin_ownership() {
    let repeated = AppRegistry::new(vec![registration("tests.one"), registration("tests.one")]);
    assert!(repeated.err().unwrap().contains("Duplicate application"));
    let first = AppRegistration::builtin(AppId::Games, builtin);
    let mut second = first.clone();
    second.manifest.id = "tests.another-games";
    let repeated = AppRegistry::new(vec![first, second]);
    assert!(repeated.err().unwrap().contains("Duplicate built-in"));
    assert!(AppRegistry::new(vec![registration("../invalid")]).is_err());
}

fn record(session: &Session, value: impl Into<String>) {
    session.state::<Events>().0.borrow_mut().push(value.into());
}
fn lifecycle(context: AppContext, event: AppLifecycle) {
    record(&context.session, format!("{}:{event:?}", context.id));
}

#[test]
fn lifecycle_orders_deactivation_before_activation_and_preserves_same_app_navigation() {
    let first = registration("tests.a")
        .with_route_changed(|_, ui, _, _| record(ui, "a:route"))
        .with_deactivate(|ui| record(ui, "a:deactivate"))
        .with_reset(|ui| record(ui, "a:reset"))
        .with_lifecycle(lifecycle);
    let second = registration("tests.b")
        .with_route_changed(|_, ui, _, _| record(ui, "b:route"))
        .with_deactivate(|ui| record(ui, "b:deactivate"))
        .with_reset(|ui| record(ui, "b:reset"))
        .with_lifecycle(lifecycle);
    let (registry, ui, vm) = fixture(vec![first, second]);
    let a = route("tests.a", "");
    let detail = route("tests.a", "detail/7");
    let b = route("tests.b", "");
    vm.navigate(a.clone());
    registry.route_changed(&vm, &ui, &Route::Start, &a);
    vm.navigate(detail.clone());
    registry.route_changed(&vm, &ui, &a, &detail);
    vm.navigate(b.clone());
    registry.route_changed(&vm, &ui, &detail, &b);
    registry.lifecycle(&vm, &ui, AppLifecycle::Suspended);
    registry.lifecycle(&vm, &ui, AppLifecycle::Resumed);
    registry.reset(&vm, &ui);
    assert_eq!(
        *ui.state::<Events>().0.borrow(),
        vec![
            "a:route",
            "tests.a:Activated",
            "a:route",
            "a:route",
            "a:deactivate",
            "tests.a:Deactivated",
            "b:route",
            "tests.b:Activated",
            "tests.b:Suspended",
            "tests.b:Resumed",
            "a:reset",
            "tests.a:Reset",
            "b:reset",
            "tests.b:Reset",
        ]
    );
}

#[test]
fn undeclared_capabilities_are_rejected_before_reaching_the_host() {
    let app = AppRegistration::new(
        AppManifest::new("tests.permissions", "Permissions", Icon::Note)
            .capabilities(&[Capability::Clipboard]),
        empty,
    );
    let (_, session, vm) = fixture(vec![app]);
    let requests = Rc::new(RefCell::new(Vec::new()));
    let recorded = requests.clone();
    vm.set_services(HostServices {
        request: Rc::new(move |request| recorded.borrow_mut().push(request)),
    });
    let context = AppContext {
        vm: vm.clone(),
        session: session.clone(),
        id: "tests.permissions",
    };
    let before = vm.data().clone();
    assert!(context
        .request(HostRequest::StartCamera)
        .unwrap_err()
        .contains("Camera"));
    assert!(requests.borrow().is_empty());
    context
        .request(HostRequest::CopyText {
            text: "local test".into(),
        })
        .unwrap();
    assert!(
        matches!(requests.borrow().as_slice(), [HostRequest::CopyText {text}] if text == "local test")
    );
    let missing = AppContext {
        id: "tests.unregistered",
        ..context.clone()
    };
    assert!(missing
        .request(HostRequest::CopyText {
            text: "blocked".into()
        })
        .is_err());
    assert_eq!(requests.borrow().len(), 1);
    assert_eq!(*vm.data(), before);
    vm.set_extension_enabled(context.id, false).unwrap();
    let disabled = vm.data().clone();
    assert!(context
        .request(HostRequest::CopyText {
            text: "blocked".into()
        })
        .unwrap_err()
        .contains("disabled"));
    assert_eq!(requests.borrow().len(), 1);
    assert_eq!(*vm.data(), disabled);
}
