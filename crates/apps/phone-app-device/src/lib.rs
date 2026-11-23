//! Native device apps and OS integration, independently registered through SDK.
mod tiles;
mod views;
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use phone_host::platform::*;
use phone_sdk::prelude::*;
use phone_sdk::AppLifecycle;
use std::cell::RefCell;
pub const APP_ID: &str = "org.ferroui.device";

#[derive(Default)]
struct Pending {
    launch: RefCell<Option<PendingLaunch>>,
}
#[derive(Clone, Debug, PartialEq, Eq)]
enum Launch {
    App(String),
    Shortcut { application: String, id: String },
}
impl Launch {
    fn application(&self) -> &str {
        match self {
            Self::App(id) => id,
            Self::Shortcut { application, .. } => application,
        }
    }
}
#[derive(Clone)]
enum Stage {
    Catalog(u64),
    Shortcuts(u64),
}
#[derive(Clone)]
struct PendingLaunch {
    target: Launch,
    epoch: u64,
    started: i64,
    stage: Stage,
}
fn request(cx: &AppContext, request: PlatformRequest) {
    if let Err(error) = cx.request(phone_host::HostRequest::Platform { request }) {
        cx.vm.set_status(error);
    }
}
fn path(id: &str) -> String {
    format!("launch/{}", URL_SAFE_NO_PAD.encode(id))
}
fn shortcut_path(application: &str, id: &str) -> String {
    format!(
        "shortcut/{}/{}",
        URL_SAFE_NO_PAD.encode(application),
        URL_SAFE_NO_PAD.encode(id)
    )
}
fn decode(path: &str) -> Result<Launch, String> {
    let decode = |value: &str| phone_sdk::device::decode_route_value(value, 1024);
    let value = if let Some(encoded) = path.strip_prefix("launch/") {
        let id = decode(encoded)?;
        PlatformRequest::LaunchApplication { id: id.clone() }.validate()?;
        Launch::App(id)
    } else if let Some(encoded) = path.strip_prefix("shortcut/") {
        let (application, id) = encoded
            .split_once('/')
            .ok_or("Invalid device shortcut route.")?;
        let application = decode(application)?;
        let id = decode(id)?;
        DeviceRequest::LaunchShortcut {
            application: application.clone(),
            id: id.clone(),
        }
        .validate()?;
        Launch::Shortcut { application, id }
    } else {
        return Err("Unknown device app route.".into());
    };
    Ok(value)
}
fn shortcuts_available(vm: &phone_state::PhoneViewModel) -> bool {
    let snapshot = vm.device_state().snapshot;
    vm.platform_state().supports(HostFeature::AppShortcuts)
        && snapshot.supports(DeviceOperation::AppShortcuts)
        && snapshot.role(DeviceRole::Home) == HostRole::Active
}
fn tick(
    vm: &std::rc::Rc<phone_state::PhoneViewModel>,
    ui: &std::rc::Rc<phone_sdk::session::Session>,
    now: i64,
) {
    views::tick(vm, ui, now);
    let pending = ui.state::<Pending>();
    let value = pending.launch.borrow().clone();
    let Some(value) = value else {
        return;
    };
    if value.epoch != vm.document_epoch()
        || !matches!(vm.route(),phone_domain::Route::Extension{app,..}if app==APP_ID)
    {
        pending.launch.borrow_mut().take();
        return;
    }
    if now.saturating_sub(value.started) > 15 {
        pending.launch.borrow_mut().take();
        vm.set_status("This device app or shortcut could not be verified. Refresh Device apps and check its Home role.".into());
        return;
    }
    match value.stage {
        Stage::Catalog(revision) if vm.device_applications_revision() != revision => {
            if !vm
                .device_applications()
                .iter()
                .any(|app| app.id == value.target.application())
            {
                pending.launch.borrow_mut().take();
                vm.set_status("This pinned device app is no longer available.".into());
                return;
            }
            match &value.target {
                Launch::App(id) => {
                    pending.launch.borrow_mut().take();
                    vm.request(phone_host::HostRequest::Platform {
                        request: PlatformRequest::LaunchApplication { id: id.clone() },
                    });
                }
                Launch::Shortcut { application, .. } if shortcuts_available(vm) => {
                    let revision = vm.device_state().shortcut_revision;
                    *pending.launch.borrow_mut() = Some(PendingLaunch {
                        stage: Stage::Shortcuts(revision),
                        ..value.clone()
                    });
                    vm.request_device(DeviceRequest::Shortcuts {
                        application: application.clone(),
                    });
                }
                _ => {} // A concurrent fresh snapshot can still confirm the role.
            }
        }
        Stage::Shortcuts(revision) => {
            let state = vm.device_state();
            let Launch::Shortcut { application, id } = &value.target else {
                return;
            };
            if state.shortcut_revision == revision
                || state.shortcut_application.as_deref() != Some(application)
            {
                return;
            }
            pending.launch.borrow_mut().take();
            if shortcuts_available(vm)
                && state
                    .shortcuts
                    .iter()
                    .any(|shortcut| shortcut.id == *id && shortcut.enabled)
            {
                vm.request_device(DeviceRequest::LaunchShortcut {
                    application: application.clone(),
                    id: id.clone(),
                });
            } else {
                vm.set_status(
                    "This shortcut is disabled or no longer available to the current Home app."
                        .into(),
                );
            }
        }
        _ => {}
    }
}
pub fn registration() -> AppRegistration {
    AppRegistration::new(
        AppManifest::new(APP_ID, "Device apps", Icon::List)
            .category(AppCategory::System)
            .capabilities(&[Capability::Platform]),
        views::view,
    )
    .with_tile(tiles::view)
    .with_activation(|vm,ui,value|{
        if value.get("kind").and_then(|v|v.as_str())!=Some("shortcuts"){return Err("Unknown device app activation.".into());}
        let application=value.get("application").and_then(|v|v.as_str()).ok_or("A device app is required.")?;
        DeviceRequest::Shortcuts{application:application.into()}.validate()?;
        if !shortcuts_available(vm){return Err("App shortcuts require the active Home role and shortcut access.".into());}
        let app=vm.device_applications().into_iter().find(|app|app.id==application).ok_or("This device application is no longer available.")?;
        views::choose_shortcuts(&AppContext{vm:vm.clone(),session:ui.clone(),id:APP_ID},app);
        Ok(())
    })
    .with_route_changed(|vm, ui, _, next| {
        ui.state::<Pending>().launch.borrow_mut().take();
        if let phone_domain::Route::Extension { app, path } = next {
            if app == APP_ID && !path.is_empty() {
                if !vm.platform_state().supports(HostFeature::InstalledApps) {
                    vm.set_status("Device application pins are unavailable on this host.".into());
                    return;
                }
                match decode(path) {
                    Ok(target) => {
                        let shortcut = matches!(target, Launch::Shortcut { .. });
                        *ui.state::<Pending>().launch.borrow_mut() = Some(PendingLaunch {
                            target,
                            epoch: vm.document_epoch(),
                            started: vm.now(),
                            stage: Stage::Catalog(vm.device_applications_revision()),
                        });
                        vm.request(phone_host::HostRequest::Platform {
                            request: PlatformRequest::ListApplications,
                        });
                        if shortcut {
                            vm.refresh_device();
                        }
                    }
                    Err(error) => vm.set_status(error),
                }
            }
        }
    })
    .with_lifecycle(|cx, event| {
        if event == AppLifecycle::Activated {
            cx.vm.refresh_device();
            if cx.session.state::<Pending>().launch.borrow().is_none()
                && cx.vm.platform_state().supports(HostFeature::InstalledApps)
            {
                request(&cx, PlatformRequest::ListApplications);
            }
        }
        if event == AppLifecycle::Closing {
            cx.session.state::<Pending>().launch.borrow_mut().take();
            views::reset(&cx.session);
        }
        if event == AppLifecycle::Deactivated {
            views::dismiss(&cx.session);
        }
    })
    .with_tick(tick)
    .with_back(|_, ui| views::back(ui))
    .with_reset(|ui| {
        ui.state::<Pending>().launch.borrow_mut().take();
        views::reset(ui);
    })
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn pins_roundtrip_opaque_os_identities_without_commands() {
        for id in [
            "org.example/.Main",
            "mac:/Applications/Editor.app",
            "windows:Vendor.App!Application",
            "linux:org.example.Editor.desktop",
        ] {
            assert_eq!(decode(&path(id)).unwrap(), Launch::App(id.into()));
        }
        assert!(decode("launch/Lw==").is_err());
        assert!(decode("file:///etc/passwd").is_err());
        assert!(decode(&path("bad\nidentity")).is_err());
    }
    #[test]
    fn shortcut_routes_preserve_boundaries_and_validate_each_identity() {
        let app = "com.example/.Main";
        let id = "message/a&b%done";
        assert_eq!(
            decode(&shortcut_path(app, id)).unwrap(),
            Launch::Shortcut {
                application: app.into(),
                id: id.into()
            }
        );
        for value in ["shortcut/a", "shortcut//", "shortcut/YQ/Yg/extra"] {
            assert!(decode(value).is_err());
        }
        assert!(decode(&shortcut_path(app, "bad\nshortcut")).is_err());
        assert!(decode(&format!("launch/{}", "a".repeat(4097))).is_err());
    }
    #[test]
    fn shortcut_pin_requires_fresh_catalog_and_matching_shortcut_response() {
        use std::rc::Rc;
        let vm =
            phone_state::PhoneViewModel::new(phone_domain::PhoneData::default(), Rc::new(|_| {}));
        vm.set_registered_apps([APP_ID.into()]);
        vm.set_platform_state(PlatformState {
            platform: phone_host::HostPlatform::Android,
            mode: HostMode::Launcher,
            launcher: HostRole::Active,
            features: vec![
                HostFeature::InstalledApps,
                HostFeature::LauncherRole,
                HostFeature::AppShortcuts,
            ],
            ..Default::default()
        })
        .unwrap();
        vm.set_device_response(DeviceResponse::Snapshot(DeviceSnapshot {
            roles: vec![DeviceRoleState {
                role: DeviceRole::Home,
                status: HostRole::Active,
                detail: String::new(),
            }],
            operations: vec![DeviceOperation::AppShortcuts],
            ..Default::default()
        }))
        .unwrap();
        let application = "com.example/.Main";
        let shortcut = DeviceShortcut {
            id: "compose".into(),
            title: "Compose".into(),
            enabled: true,
            pinned: false,
        };
        vm.set_device_response(DeviceResponse::Shortcuts {
            application: application.into(),
            shortcuts: vec![shortcut.clone()],
        })
        .unwrap();
        let registry = Rc::new(phone_sdk::AppRegistry::new(vec![registration()]).unwrap());
        let ui = Rc::new(phone_sdk::session::Session::new(vm.now(), registry.clone()));
        let requests = Rc::new(RefCell::new(Vec::new()));
        let capture = requests.clone();
        vm.set_services(phone_state::HostServices {
            request: Rc::new(move |request| capture.borrow_mut().push(request)),
        });
        let route = phone_domain::Route::Extension {
            app: APP_ID.into(),
            path: shortcut_path(application, "compose"),
        };
        vm.navigate(route.clone());
        registry.route_changed(&vm, &ui, &phone_domain::Route::Start, &route);
        let launched = || {
            requests
                .borrow()
                .iter()
                .filter(|request| {
                    matches!(
                        request,
                        phone_host::HostRequest::Platform {
                            request: PlatformRequest::Device {
                                request: DeviceRequest::LaunchShortcut { .. }
                            }
                        }
                    )
                })
                .count()
        };
        tick(&vm, &ui, vm.now());
        assert_eq!(launched(), 0);
        vm.set_device_applications(vec![DeviceApplication {
            id: application.into(),
            title: "Example".into(),
        }])
        .unwrap();
        tick(&vm, &ui, vm.now());
        assert!(matches!(
            ui.state::<Pending>()
                .launch
                .borrow()
                .as_ref()
                .unwrap()
                .stage,
            Stage::Shortcuts(_)
        ));
        vm.set_device_response(DeviceResponse::Notifications(Vec::new()))
            .unwrap();
        tick(&vm, &ui, vm.now());
        assert_eq!(
            launched(),
            0,
            "Unrelated observations cannot acknowledge cached shortcuts"
        );
        vm.set_device_response(DeviceResponse::Shortcuts {
            application: "another/.Main".into(),
            shortcuts: vec![shortcut.clone()],
        })
        .unwrap();
        tick(&vm, &ui, vm.now());
        assert_eq!(
            launched(),
            0,
            "Another app's query cannot authorize this route"
        );
        vm.set_device_response(DeviceResponse::Shortcuts {
            application: application.into(),
            shortcuts: vec![shortcut],
        })
        .unwrap();
        tick(&vm, &ui, vm.now());
        tick(&vm, &ui, vm.now());
        assert_eq!(
            launched(),
            1,
            "The matching enabled shortcut is dispatched exactly once"
        );
    }
    #[test]
    fn pending_native_launch_does_not_survive_restore_or_timeout() {
        use std::rc::Rc;
        let vm =
            phone_state::PhoneViewModel::new(phone_domain::PhoneData::default(), Rc::new(|_| {}));
        vm.set_registered_apps([APP_ID.into()]);
        let ui = Rc::new(phone_sdk::session::Session::new(
            vm.now(),
            Rc::new(phone_sdk::AppRegistry::new(vec![registration()]).unwrap()),
        ));
        let route = phone_domain::Route::Extension {
            app: APP_ID.into(),
            path: path("com.example/.Main"),
        };
        vm.navigate(route);
        *ui.state::<Pending>().launch.borrow_mut() = Some(PendingLaunch {
            target: Launch::App("com.example/.Main".into()),
            epoch: vm.document_epoch(),
            started: vm.now(),
            stage: Stage::Catalog(vm.device_applications_revision()),
        });
        tick(&vm, &ui, vm.now() + 16);
        assert!(ui.state::<Pending>().launch.borrow().is_none());
        assert!(vm.status().contains("could not be verified"));
        *ui.state::<Pending>().launch.borrow_mut() = Some(PendingLaunch {
            target: Launch::App("com.example/.Main".into()),
            epoch: vm.document_epoch(),
            started: vm.now(),
            stage: Stage::Catalog(0),
        });
        vm.import_json(&vm.export_json().unwrap()).unwrap();
        tick(&vm, &ui, vm.now());
        assert!(ui.state::<Pending>().launch.borrow().is_none());
    }
}
