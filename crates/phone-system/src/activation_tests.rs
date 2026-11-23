use super::*;
use phone_domain::{AppId, PhoneData, Route};
use phone_sdk::{AppManifest, AppRegistration, AppRegistry};
use std::cell::Cell;

#[derive(Clone, Copy, Default)]
enum Delivery {
    #[default]
    Detail,
    AnotherApp,
    Restore,
    Close,
    ReplaceSession,
}

#[derive(Default)]
struct ActivationState {
    delivery: Cell<Delivery>,
    calls: Cell<usize>,
    restore_on_deactivate: Cell<bool>,
    model: RefCell<Weak<PhoneViewModel>>,
    replacement: RefCell<Option<Rc<session::Session>>>,
}

struct Mounted {
    vm: Rc<PhoneViewModel>,
    ui: Rc<session::Session>,
    owner: Owner,
}

impl Mounted {
    fn new() -> Self {
        let settings = AppRegistration::builtin(AppId::Settings, |_, _| Element::Empty)
            .with_launch_path(|path| match path {
                "device" => Ok(serde_json::json!({"page": "device services"})),
                "communications" => Ok(serde_json::json!({"page": "phone+messages"})),
                _ => Err("Unknown Settings route.".into()),
            })
            .with_activation(|vm, ui, payload| {
                let state = ui.state::<ActivationState>();
                state.calls.set(state.calls.get() + 1);
                match state.delivery.get() {
                    Delivery::Detail => {}
                    Delivery::AnotherApp => {
                        vm.navigate(Route::App(AppId::Phone));
                        return Ok(());
                    }
                    Delivery::Restore => vm.replace_data(PhoneData::default())?,
                    Delivery::Close => host_closing(vm),
                    Delivery::ReplaceSession => {
                        let replacement = Rc::new(session::Session::new(vm.now(), ui.registry()));
                        SESSION.with(|slot| *slot.borrow_mut() = Rc::downgrade(&replacement));
                        *state.replacement.borrow_mut() = Some(replacement);
                    }
                }
                vm.navigate(Route::Setting(payload["page"].as_str().unwrap().into()));
                Ok(())
            })
            .with_deactivate(|ui| {
                let state = ui.state::<ActivationState>();
                if state.restore_on_deactivate.replace(false) {
                    let vm = state.model.borrow().upgrade().unwrap();
                    vm.replace_data(PhoneData::default()).unwrap();
                    // App callbacks may synchronously reconcile a restored document.
                    ui.reconcile_document(&vm);
                }
            });
        let registry = Rc::new(
            AppRegistry::new(vec![
                settings,
                AppRegistration::builtin(AppId::Phone, |_, _| Element::Empty),
                AppRegistration::new(
                    AppManifest::new("tests.launch", "Launch", phone_controls::Icon::Note),
                    |_| Element::Empty,
                ),
            ])
            .unwrap(),
        );
        let vm = PhoneViewModel::new(PhoneData::default(), Rc::new(|_| {}));
        registry.bind(&vm);
        let ui = Rc::new(session::Session::new(vm.now(), registry));
        *ui.state::<ActivationState>().model.borrow_mut() = Rc::downgrade(&vm);
        MODEL.with(|slot| *slot.borrow_mut() = Rc::downgrade(&vm));
        SESSION.with(|slot| *slot.borrow_mut() = Rc::downgrade(&ui));
        let owner = Owner::new_root();
        owner.with(|| {
            // Use the production Revision-to-Signal bridge and activation ordering,
            // so a queue write can really rerender during launch_app itself.
            let revision = signal(vm.revision());
            let observed = vm.clone();
            let subscription = vm
                .changed()
                .subscribe_next(move |_| revision.set(observed.revision()));
            on_cleanup(move || subscription.dispose());
            let vm = vm.clone();
            let ui = ui.clone();
            effect(move || {
                revision.get();
                ui.reconcile_document(&vm);
                let route = vm.route();
                ui.route_changed(&vm, &route);
                let registry = ui.registry();
                if let Some(app) = registry.for_route(&route) {
                    if let Some(payload) = ui.take_activation(app.manifest.id) {
                        registry
                            .activate(&vm, &ui, app.manifest.id, payload)
                            .unwrap();
                    }
                }
            });
        });
        Self { vm, ui, owner }
    }

    fn state(&self) -> Rc<ActivationState> {
        self.ui.state::<ActivationState>()
    }
}

impl Drop for Mounted {
    fn drop(&mut self) {
        self.owner.dispose();
        MODEL.with(|slot| *slot.borrow_mut() = Weak::new());
        SESSION.with(|slot| *slot.borrow_mut() = Weak::new());
    }
}

#[test]
fn typed_launch_accepts_an_owned_synchronous_detail_and_preserves_back_history() {
    let shell = Mounted::new();
    assert!(launch_app(&shell.vm, "phone.settings", "device"));
    assert_eq!(shell.vm.route(), Route::Setting("device services".into()));
    assert_eq!(shell.state().calls.get(), 1);
    shell.vm.back();
    assert_eq!(shell.vm.route(), Route::App(AppId::Settings));
    shell.vm.back();
    assert_eq!(shell.vm.route(), Route::Start);
}

#[test]
fn typed_launch_batches_payload_and_navigation_in_an_already_active_app() {
    let shell = Mounted::new();
    shell.vm.navigate(Route::App(AppId::Settings));
    for (path, page) in [
        ("device", "device services"),
        ("communications", "phone+messages"),
        ("communications", "phone+messages"),
    ] {
        let accepted = launch_app(&shell.vm, "phone.settings", path);
        assert_eq!(shell.vm.route(), Route::Setting(page.into()));
        assert!(accepted);
    }
    assert_eq!(shell.state().calls.get(), 3);
}

#[test]
fn launch_rejects_a_synchronous_redirect_to_another_application() {
    let shell = Mounted::new();
    shell.state().delivery.set(Delivery::AnotherApp);
    assert!(!launch_app(&shell.vm, "phone.settings", "device"));
    assert_eq!(shell.vm.route(), Route::App(AppId::Phone));
    assert_eq!(shell.state().calls.get(), 1);
}

#[test]
fn owned_route_does_not_survive_document_or_shell_identity_replacement() {
    for delivery in [Delivery::Restore, Delivery::Close, Delivery::ReplaceSession] {
        let shell = Mounted::new();
        shell.state().delivery.set(delivery);
        assert!(!launch_app(&shell.vm, "phone.settings", "device"));
        assert_eq!(shell.vm.route(), Route::Setting("device services".into()));
        assert_eq!(shell.state().calls.get(), 1);
    }
}

#[test]
fn overlay_callback_restore_stops_launch_before_payload_or_navigation() {
    let shell = Mounted::new();
    shell.vm.navigate(Route::App(AppId::Settings));
    shell.state().restore_on_deactivate.set(true);
    assert!(!launch_app(&shell.vm, "phone.settings", "device"));
    assert_eq!(shell.vm.route(), Route::Start);
    assert_eq!(shell.state().calls.get(), 0);
    assert!(shell.ui.take_activation("phone.settings").is_none());
}

#[test]
fn launch_requires_the_mounted_model_enabled_app_and_valid_path() {
    let shell = Mounted::new();
    let other = PhoneViewModel::new(PhoneData::default(), Rc::new(|_| {}));
    shell.ui.registry().bind(&other);
    assert!(!launch_app(&other, "phone.settings", ""));
    assert_eq!(other.route(), Route::Start);
    assert!(!launch_app(&shell.vm, "phone.settings", "unknown"));
    assert!(!launch_app(&shell.vm, "phone.missing", ""));
    assert!(!launch_app(&shell.vm, "phone.settings", "device\n"));
    shell
        .vm
        .set_extension_enabled("tests.launch", false)
        .unwrap();
    assert!(!launch_app(&shell.vm, "tests.launch", "detail"));
    assert_eq!(shell.vm.route(), Route::Start);
    assert_eq!(shell.state().calls.get(), 0);
}
