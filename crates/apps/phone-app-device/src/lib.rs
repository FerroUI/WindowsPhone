//! Native device apps and OS integration, independently registered through SDK.
mod views;
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use phone_host::platform::{HostFeature, PlatformRequest};
use phone_sdk::prelude::*;
use phone_sdk::AppLifecycle;
use std::cell::RefCell;
pub const APP_ID: &str = "org.ferroui.device";
#[derive(Default)]
struct Pending {
    launch: RefCell<Option<PendingLaunch>>,
}
#[derive(Clone)]
struct PendingLaunch {
    id: String,
    epoch: u64,
    started: i64,
    catalog_revision: u64,
}
fn request(cx: &AppContext, request: PlatformRequest) {
    if let Err(error) = cx.request(phone_host::HostRequest::Platform { request }) {
        cx.vm.set_status(error);
    }
}
fn path(id: &str) -> String {
    format!("launch/{}", URL_SAFE_NO_PAD.encode(id))
}
fn decode(path: &str) -> Result<String, String> {
    let encoded = path
        .strip_prefix("launch/")
        .ok_or("Unknown device app route.")?;
    let bytes = URL_SAFE_NO_PAD
        .decode(encoded)
        .map_err(|_| "Invalid device app route.")?;
    let id = String::from_utf8(bytes).map_err(|_| "Invalid device application identity.")?;
    PlatformRequest::LaunchApplication { id: id.clone() }.validate()?;
    Ok(id)
}
pub fn registration() -> AppRegistration {
    AppRegistration::new(
        AppManifest::new(APP_ID, "Device apps", Icon::List)
            .category(AppCategory::System)
            .capabilities(&[Capability::Platform]),
        views::view,
    )
    .with_route_changed(|vm, ui, _, next| {
        ui.state::<Pending>().launch.borrow_mut().take();
        if let phone_domain::Route::Extension { app, path } = next {
            if app == APP_ID && !path.is_empty() {
                if !vm.platform_state().supports(HostFeature::InstalledApps) {
                    vm.set_status("Device application pins are unavailable on this host.".into());
                    return;
                }
                match decode(path) {
                    Ok(id) => {
                        *ui.state::<Pending>().launch.borrow_mut() = Some(PendingLaunch {
                            id,
                            epoch: vm.document_epoch(),
                            started: vm.now(),
                            catalog_revision: vm.device_applications_revision(),
                        });
                        vm.request(phone_host::HostRequest::Platform {
                            request: PlatformRequest::ListApplications,
                        });
                    }
                    Err(error) => vm.set_status(error),
                }
            }
        }
    })
    .with_lifecycle(|cx, event| {
        if event == AppLifecycle::Activated
            && cx.session.state::<Pending>().launch.borrow().is_none()
            && cx.vm.platform_state().supports(HostFeature::InstalledApps)
        {
            request(&cx, PlatformRequest::ListApplications);
        }
    })
    .with_tick(|vm, ui, now| {
        let pending = ui.state::<Pending>();
        let value = pending.launch.borrow().clone();
        if let Some(PendingLaunch {
            id,
            epoch,
            started,
            catalog_revision,
        }) = value
        {
            if epoch != vm.document_epoch()
                || !matches!(vm.route(),phone_domain::Route::Extension{app,..}if app==APP_ID)
            {
                pending.launch.borrow_mut().take();
                return;
            }
            if vm.device_applications_revision() != catalog_revision {
                pending.launch.borrow_mut().take();
                if vm.device_applications().iter().any(|app| app.id == id) {
                    vm.request(phone_host::HostRequest::Platform {
                        request: PlatformRequest::LaunchApplication { id },
                    });
                } else {
                    vm.set_status("This pinned device app is no longer available.".into());
                }
            } else if now.saturating_sub(started) > 15 {
                pending.launch.borrow_mut().take();
                vm.set_status(
                    "This device app is unavailable. Refresh the device application list.".into(),
                );
            }
        }
    })
    .with_reset(|ui| {
        ui.state::<Pending>().launch.borrow_mut().take();
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
            assert_eq!(decode(&path(id)).unwrap(), id);
        }
        assert!(decode("launch/Lw==").is_err());
        assert!(decode("file:///etc/passwd").is_err());
        assert!(decode(&path("bad\nidentity")).is_err());
    }
}
