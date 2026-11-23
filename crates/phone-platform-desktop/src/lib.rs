//! Native desktop app catalogs and session policy, independent of the UI tree.
//! OS identities come from a bounded, observed catalog. Requests never contain
//! executable command lines, shell fragments, arbitrary paths, or arguments.

use phone_host::platform::{
    ready, validate_applications, DeviceApplication, HostFeature, HostLifecycle, HostMode,
    HostRole, PlatformPlugin, PlatformRequest, PlatformResponse, PlatformState, PlatformTask,
    MAX_DEVICE_APPLICATIONS,
};
use std::collections::BTreeMap;
use std::sync::{
    atomic::{AtomicBool, AtomicU64, Ordering},
    Arc, Mutex,
};

#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "linux")]
use linux as backend;
#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "macos")]
use macos as backend;
#[cfg(target_os = "windows")]
mod windows;
#[cfg(target_os = "windows")]
use windows as backend;
#[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
compile_error!("phone-platform-desktop supports Linux, macOS and Windows only");

#[derive(Clone, Debug)]
struct Application {
    id: String,
    title: String,
    /// Private native identity. Never accepted directly from a request.
    target: String,
}

#[derive(Default)]
struct Catalog {
    targets: BTreeMap<String, String>,
}

impl Catalog {
    fn replace(
        &mut self,
        applications: Vec<Application>,
    ) -> Result<Vec<DeviceApplication>, String> {
        if applications.len() > MAX_DEVICE_APPLICATIONS {
            return Err("The installed application catalog exceeds its limit.".into());
        }
        let mut targets = BTreeMap::new();
        let mut visible = Vec::with_capacity(applications.len());
        for application in applications {
            if targets
                .insert(application.id.clone(), application.target)
                .is_some()
            {
                return Err(
                    "The installed application catalog contains an ambiguous identity.".into(),
                );
            }
            visible.push(DeviceApplication {
                id: application.id,
                title: application.title,
            });
        }
        validate_applications(&visible)?;
        visible.sort_by(|left, right| {
            left.title
                .to_lowercase()
                .cmp(&right.title.to_lowercase())
                .then_with(|| left.id.cmp(&right.id))
        });
        self.targets = targets;
        Ok(visible)
    }

    fn target(&self, id: &str) -> Result<String, String> {
        self.targets.get(id).cloned().ok_or_else(|| {
            "That application is not in the current device catalog. Refresh the applications list.".into()
        })
    }
}

/// A desktop app/launcher provider. Creating it does not change the person's
/// shell, login session, file associations, or default applications.
pub struct DesktopPlatform {
    mode: HostMode,
    state: Arc<Mutex<PlatformState>>,
    catalog: Arc<Mutex<Catalog>>,
    catalog_request: Arc<AtomicU64>,
    closed: Arc<AtomicBool>,
}

impl DesktopPlatform {
    pub fn new(mode: HostMode) -> Result<Self, String> {
        let state = backend::state(mode);
        state.validate()?;
        Ok(Self {
            mode,
            state: Arc::new(Mutex::new(state)),
            catalog: Arc::new(Mutex::new(Catalog::default())),
            catalog_request: Arc::new(AtomicU64::new(0)),
            closed: Arc::new(AtomicBool::new(false)),
        })
    }

    fn available(&self) -> Result<(), String> {
        if self.closed.load(Ordering::Acquire) {
            Err("The desktop host has closed.".into())
        } else {
            Ok(())
        }
    }
}

impl PlatformPlugin for DesktopPlatform {
    fn id(&self) -> &'static str {
        "ferroui.desktop"
    }

    fn state(&self) -> PlatformState {
        self.state
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .clone()
    }

    fn dispatch(&self, request: PlatformRequest) -> PlatformTask {
        if let Err(error) = self.available().and_then(|_| request.validate()) {
            return ready(Err(error));
        }
        if let Some(feature) = request.feature() {
            if !self.state().supports(feature) {
                return ready(Err(match feature {
                    HostFeature::LauncherRole => "This desktop has no default HOME-app chooser. Use launcher mode, or install the documented opt-in desktop session.".into(),
                    _ => "This service is not provided by the desktop platform plugin.".into(),
                }));
            }
        }
        match request {
            PlatformRequest::Device {
                request: phone_host::platform::DeviceRequest::Snapshot,
            } => {
                use phone_host::platform::{
                    DeviceResponse, DeviceSnapshot, SystemSettingsPage as P,
                };
                let state = self.state();
                ready(Ok(PlatformResponse::Device(DeviceResponse::Snapshot(DeviceSnapshot{
                    settings:if state.supports(HostFeature::SystemSettings){vec![P::Application,P::Network,P::Notifications,P::DefaultApps,P::Wifi,P::Bluetooth,P::Display,P::Brightness,P::Sound,P::AudioOutput,P::Battery,P::BatterySaver,P::Storage,P::Security,P::Accounts,P::Sync,P::DateTime,P::Language,P::Keyboard,P::Accessibility,P::Privacy,P::Location,P::Vpn,P::SystemUpdate,P::About]}else{vec![]},
                    detail:"Desktop hardware changes use your operating system's settings application. The host does not claim unobserved radio or mixer values.".into(),
                    ..Default::default()
                }))))
            }
            PlatformRequest::Refresh => {
                let (mode, state, closed) = (self.mode, self.state.clone(), self.closed.clone());
                Box::pin(async move {
                    if closed.load(Ordering::Acquire) {
                        return Err("The desktop host has closed.".into());
                    }
                    let pending = closed.clone();
                    let observed = phone_host::native::background::run_system(move || {
                        if pending.load(Ordering::Acquire) {
                            return Err("The desktop host has closed.".into());
                        }
                        Ok(backend::state(mode))
                    })
                    .await?;
                    if closed.load(Ordering::Acquire) {
                        return Err("The desktop host has closed.".into());
                    }
                    observed.validate()?;
                    *state.lock().unwrap_or_else(|error| error.into_inner()) = observed.clone();
                    Ok(PlatformResponse::State(observed))
                })
            }
            PlatformRequest::ListApplications => {
                let (catalog, closed) = (self.catalog.clone(), self.closed.clone());
                let requests = self.catalog_request.clone();
                let current = match requests.try_update(
                    Ordering::AcqRel,
                    Ordering::Acquire,
                    |value| value.checked_add(1),
                ) {
                    Ok(value) => value + 1,
                    Err(_) => return ready(Err(
                        "The application catalog request counter is exhausted. Restart the host."
                            .into(),
                    )),
                };
                Box::pin(async move {
                    if closed.load(Ordering::Acquire) {
                        return Err("The desktop host has closed.".into());
                    }
                    let pending = closed.clone();
                    let applications = phone_host::native::background::run_system(move || {
                        if pending.load(Ordering::Acquire) {
                            return Err("The desktop host has closed.".into());
                        }
                        backend::applications()
                    })
                    .await?;
                    if closed.load(Ordering::Acquire) {
                        return Err("The desktop host has closed.".into());
                    }
                    if requests.load(Ordering::Acquire) != current {
                        return Err("A newer application catalog request replaced this one.".into());
                    }
                    let applications = catalog
                        .lock()
                        .unwrap_or_else(|error| error.into_inner())
                        .replace(applications)?;
                    Ok(PlatformResponse::Applications(applications))
                })
            }
            PlatformRequest::LaunchApplication { id } => {
                let target = self
                    .catalog
                    .lock()
                    .unwrap_or_else(|error| error.into_inner())
                    .target(&id);
                let target = match target {
                    Ok(target) => target,
                    Err(error) => return ready(Err(error)),
                };
                #[cfg(target_os = "macos")]
                {
                    let task = backend::launch(target);
                    let closed = self.closed.clone();
                    Box::pin(async move {
                        let result = task.await;
                        if closed.load(Ordering::Acquire) {
                            return Err("The desktop host has closed.".into());
                        }
                        result
                    })
                }
                #[cfg(not(target_os = "macos"))]
                {
                    let closed = self.closed.clone();
                    Box::pin(async move {
                        let pending = closed.clone();
                        phone_host::native::background::run_system(move || {
                            if pending.load(Ordering::Acquire) {
                                return Err("The desktop host has closed.".into());
                            }
                            backend::launch(&target)
                        })
                        .await?;
                        if closed.load(Ordering::Acquire) {
                            return Err("The desktop host has closed.".into());
                        }
                        Ok(PlatformResponse::Accepted {
                            message:
                                "The operating system accepted the application launch request."
                                    .into(),
                        })
                    })
                }
            }
            PlatformRequest::OpenSettings { page } => {
                let task = backend::open_settings(page);
                let closed = self.closed.clone();
                Box::pin(async move {
                    if closed.load(Ordering::Acquire) {
                        return Err("The desktop host has closed.".into());
                    }
                    let result = task.await;
                    if closed.load(Ordering::Acquire) {
                        return Err("The desktop host has closed.".into());
                    }
                    result
                })
            }
            _ => ready(Err(
                "This service is owned by another native host adapter.".into()
            )),
        }
    }

    fn lifecycle(&self, event: HostLifecycle) {
        if event == HostLifecycle::Closing {
            self.closed.store(true, Ordering::Release);
            self.catalog
                .lock()
                .unwrap_or_else(|error| error.into_inner())
                .targets
                .clear();
        }
    }
}

fn application(prefix: &str, identity: &str, title: &str, target: &str) -> Option<Application> {
    let id = format!("{prefix}:{identity}");
    if id.len() > 1024
        || identity.trim().is_empty()
        || id.chars().any(char::is_control)
        || target.is_empty()
        || target.len() > 16_384
        || target.contains('\0')
    {
        return None;
    }
    let title = title
        .chars()
        .filter(|character| !character.is_control())
        .collect::<String>();
    let mut end = title.len().min(512);
    while !title.is_char_boundary(end) {
        end -= 1;
    }
    let title = title[..end].trim().to_owned();
    if title.is_empty() {
        return None;
    }
    Some(Application {
        id,
        title,
        target: target.to_owned(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn launch_accepts_only_identities_from_the_last_successful_catalog() {
        let mut catalog = Catalog::default();
        assert!(catalog.target("linux:editor.desktop").is_err());
        catalog
            .replace(vec![application(
                "linux",
                "editor.desktop",
                "Editor",
                "editor.desktop",
            )
            .unwrap()])
            .unwrap();
        assert_eq!(
            catalog.target("linux:editor.desktop").unwrap(),
            "editor.desktop"
        );
        for injected in [
            "/bin/sh",
            "linux:../../tmp/a.desktop",
            "linux:editor.desktop --argument",
        ] {
            assert!(catalog.target(injected).is_err());
        }
        catalog.replace(vec![]).unwrap();
        assert!(catalog.target("linux:editor.desktop").is_err());
    }

    #[test]
    fn malformed_replacement_does_not_partially_replace_a_catalog() {
        let mut catalog = Catalog::default();
        let original = application("linux", "ok.desktop", "OK", "ok.desktop").unwrap();
        catalog.replace(vec![original.clone()]).unwrap();
        assert!(catalog.replace(vec![original.clone(), original]).is_err());
        assert_eq!(catalog.target("linux:ok.desktop").unwrap(), "ok.desktop");
    }

    #[test]
    fn catalog_text_is_bounded_without_breaking_unicode() {
        let app = application(
            "macos",
            "identity",
            &"⛄".repeat(500),
            "/Applications/Example.app",
        )
        .unwrap();
        assert_eq!(app.title.len(), 510);
        assert!(application("linux", "bad\nidentity", "Title", "ok").is_none());
        assert!(application("linux", "", "Title", "ok").is_none());
        assert!(application("linux", "valid", "\r\n", "ok").is_none());
    }

    #[test]
    fn fullscreen_intent_never_substitutes_for_a_configured_session() {
        let state = PlatformState {
            mode: HostMode::Shell,
            shell: HostRole::Available,
            features: vec![HostFeature::ShellSession],
            ..Default::default()
        };
        assert!(state.validate().is_err());
    }

    #[test]
    fn closing_revokes_admitted_targets_and_unpolled_discovery() {
        let platform = DesktopPlatform {
            mode: HostMode::App,
            state: Arc::new(Mutex::new(PlatformState {
                features: vec![HostFeature::InstalledApps],
                ..Default::default()
            })),
            catalog: Arc::new(Mutex::new(Catalog::default())),
            catalog_request: Arc::new(AtomicU64::new(0)),
            closed: Arc::new(AtomicBool::new(false)),
        };
        platform
            .catalog
            .lock()
            .unwrap()
            .replace(vec![application(
                "fixture",
                "app",
                "Example",
                "private-target",
            )
            .unwrap()])
            .unwrap();
        let mut pending = platform.dispatch(PlatformRequest::ListApplications);
        platform.lifecycle(HostLifecycle::Closing);
        let mut context = std::task::Context::from_waker(std::task::Waker::noop());
        assert!(matches!(
            pending.as_mut().poll(&mut context),
            std::task::Poll::Ready(Err(_))
        ));
        assert!(platform
            .catalog
            .lock()
            .unwrap()
            .target("fixture:app")
            .is_err());
        let mut launch = platform.dispatch(PlatformRequest::LaunchApplication {
            id: "fixture:app".into(),
        });
        assert!(matches!(
            launch.as_mut().poll(&mut context),
            std::task::Poll::Ready(Err(_))
        ));
    }
}
