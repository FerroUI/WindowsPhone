//! Exercise the real retained list hooks across empty and matching searches.
use super::*;
use ferroui_base::Ref;
use ferroui_base::interactivity::RoutedEventArgs;
use ferroui_base::threading::{CancellationToken, Dispatcher};
use ferroui_controls::{
    Application, Control, ListBox, TextBlock, Window, automation::AutomationProperties,
};
use ferroui_headless::{FerroHeadlessPlatform, FerroTestAssembly, HeadlessUnitTestSession};
use ferroui_reactive::ObservableExt;
use ferroui_reactiveui::IReactiveNotifyPropertyChanged;
use phone_domain::PhoneData;
use phone_sdk::{AppRegistry, host::platform::*};
use std::time::Duration;

fn frame() {
    for _ in 0..4 {
        Dispatcher::ui_thread().run_jobs(None);
        FerroHeadlessPlatform::force_render_timer_tick(1);
        Dispatcher::ui_thread().run_jobs(None);
    }
}

fn native_contact() -> DeviceContact {
    DeviceContact {
        id: "device-ada".into(),
        name: "Ada Device".into(),
        phones: vec!["+15550100001".into()],
        ..Default::default()
    }
}

struct Mounted {
    vm: Rc<PhoneViewModel>,
    ui: Rc<Session>,
    root: Ref<Control>,
    window: Ref<Window>,
    owner: Owner,
}

impl Mounted {
    fn new(section: usize) -> Self {
        let mut data = PhoneData::default();
        data.contacts = vec![Contact {
            id: 1,
            name: "Ada Local".into(),
            favorite: true,
            ..Default::default()
        }];
        data.settings.lock_timeout = 0;
        let vm = PhoneViewModel::new(data, Rc::new(|_| {}));
        vm.set_platform_state(PlatformState {
            features: vec![HostFeature::Contacts],
            ..Default::default()
        })
        .unwrap();
        vm.set_device_response(DeviceResponse::Snapshot(DeviceSnapshot {
            permissions: vec![DevicePermissionState {
                permission: DevicePermission::Contacts,
                status: PermissionState::Granted,
            }],
            operations: vec![DeviceOperation::ReadContacts],
            ..Default::default()
        }))
        .unwrap();
        vm.set_device_response(DeviceResponse::Contacts(vec![native_contact()]))
            .unwrap();
        let registry = Rc::new(AppRegistry::new(vec![crate::registration()]).unwrap());
        registry.bind(&vm);
        let ui = Rc::new(Session::new(vm.now(), registry));
        ui.pivot.set(section);
        phone_sdk::set_current_session(&ui);
        vm.navigate(Route::App(AppId::People));
        let owner = Owner::new_root();
        let root = owner.with(|| {
            let revision = signal(vm.revision());
            let observed = vm.clone();
            let subscription = vm
                .changed()
                .subscribe_next(move |_| revision.set(observed.revision()));
            on_cleanup(move || subscription.dispose());
            let vm = vm.clone();
            let ui = ui.clone();
            rsx! {Grid {{rerender(move || {
                revision.get();
                crate::view(vm.clone(), ui.clone())
            })}}}
            .mount()
        });
        let window = Window::new();
        window.set_width(320.0);
        window.set_height(640.0);
        window.set_content(Some(Control::boxed(root.clone())));
        let mounted = Self {
            vm,
            ui,
            root,
            window,
            owner,
        };
        mounted.window.show();
        frame();
        mounted
    }

    fn click(&self, label: &str) {
        let button = self
            .root
            .find_descendant_of_type_where::<Button>(false, |button| {
                AutomationProperties::get_name(button).as_deref() == Some(label)
            })
            .unwrap_or_else(|| panic!("missing native command {label}"));
        assert!(button.is_effectively_visible() && button.is_effectively_enabled());
        button.raise_event(&RoutedEventArgs::with_event(Button::click_event()));
        frame();
    }

    fn editor(&self, device: bool) -> Ref<TextBox> {
        let name = if device {
            "DevicePeopleSearch"
        } else {
            "PeopleSearchInput"
        };
        self.root
            .find_descendant_of_type_where::<TextBox>(false, |editor| editor.has_name(name))
            .expect("native search editor")
    }

    fn list(&self, device: bool) -> Option<Ref<ListBox>> {
        let name = if device {
            "DevicePeopleSearchList"
        } else {
            "PeopleSearchList"
        };
        self.root
            .find_descendant_of_type_where::<ListBox>(false, |list| list.has_name(name))
    }

    fn has_text(&self, text: &str) -> bool {
        self.root
            .find_descendant_of_type_where::<TextBlock>(false, |part| {
                part.text().as_deref() == Some(text)
            })
            .is_some()
    }
}

impl Drop for Mounted {
    fn drop(&mut self) {
        self.window.close();
        self.window.set_content(None);
        self.owner.dispose();
        frame();
        phone_controls::set_motion_enabled(true);
    }
}

#[test]
fn local_favorite_and_device_search_cross_empty_results_without_losing_hooks_or_focus() {
    // Native manipulation/material resources have dispatcher-thread lifetime;
    // keep that documented assembly lifetime with fresh per-dispatch apps.
    // Each mounted view still closes and disposes on success or panic.
    static ASSEMBLY: FerroTestAssembly = FerroTestAssembly {
        name: "phone-app-people.hub",
        test_application: None,
        test_isolation: None,
    };
    let session = HeadlessUnitTestSession::get_or_start_for_assembly(Some(&ASSEMBLY));
    session
        .dispatch(
            || {
                phone_controls::set_motion_enabled(true);
                phone_controls::install_theme(&Application::current().unwrap());
                for section in 0..3 {
                    let app = Mounted::new(section);
                    let original = serde_json::to_value(&*app.vm.data()).unwrap();
                    app.click("more");
                    app.click("search");
                    let device = section == 2;
                    let editor = app.editor(device);
                    assert!(editor.is_focused());
                    let count = if device { 1 } else { 2 }; // local rows include their letter header
                    assert_eq!(app.list(device).unwrap().item_count(), count);
                    for query in ["no matching person", "Ada", "another absent name", ""] {
                        editor.set_text(Some(query));
                        frame();
                        assert!(editor.ptr_eq(&app.editor(device)) && editor.is_focused());
                        if query.is_empty() || query == "Ada" {
                            assert_eq!(app.list(device).unwrap().item_count(), count);
                        } else {
                            assert!(app.list(device).is_none());
                            assert!(app.has_text("No contacts"));
                        }
                    }
                    if device {
                        app.vm
                            .set_device_response(DeviceResponse::Contacts(Vec::new()))
                            .unwrap();
                        frame();
                        assert!(app.list(true).is_none());
                        assert!(app.has_text("No device contacts"));
                        app.vm
                            .set_device_response(DeviceResponse::Contacts(vec![native_contact()]))
                            .unwrap();
                        frame();
                        assert_eq!(app.list(true).unwrap().item_count(), 1);
                        assert!(editor.ptr_eq(&app.editor(true)) && editor.is_focused());
                    }
                    assert!(app.ui.registry().back(&app.vm, &app.ui));
                    frame();
                    assert_eq!(app.ui.pivot.get_untracked(), section);
                    assert!(app.ui.state::<State>().search.get_untracked().is_none());
                    assert!(
                        app.root
                            .find_descendant_of_type_where::<ferroui_controls::Carousel>(
                                false,
                                |part| part.has_name("PeoplePanorama")
                            )
                            .is_some()
                    );
                    assert_eq!(serde_json::to_value(&*app.vm.data()).unwrap(), original);
                }
            },
            CancellationToken::none(),
        )
        .wait_timeout(Duration::from_secs(30))
        .expect("People search and provider empty-state regression completes");
}
