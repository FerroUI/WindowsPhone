//! Explicit replaceable service providers; no first-success fallback that could
//! turn a denied permission into a different provider's side effect.
use crate::HostRequest;
use std::{collections::BTreeMap, rc::Rc};
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum HostService {
    Network,
    Files,
    Navigation,
    Clipboard,
    Media,
    Camera,
    Notifications,
    Location,
    Sharing,
    Environment,
    Updates,
    Platform,
}
impl HostService {
    pub fn for_request(request: &HostRequest) -> Self {
        use HostRequest::*;
        match request {
            HttpGet { .. } => Self::Network,
            ChooseFile { .. }
            | ImportContacts
            | ExportContacts { .. }
            | ShareContacts { .. }
            | ImportCalendar
            | ExportCalendar { .. }
            | ShareCalendar { .. }
            | ImportPhotos
            | ImportMusic
            | ImportVideo
            | DeleteMedia { .. }
            | ExportBackup
            | ImportBackup
            | ImportDocuments { .. }
            | ExportDocument { .. }
            | ShareDocuments { .. } => Self::Files,
            OpenUrl { .. }
            | Browse { .. }
            | ClearBrowser
            | Dial { .. }
            | Sms { .. }
            | Email { .. } => Self::Navigation,
            CopyText { .. } => Self::Clipboard,
            PlayMedia { .. } | MediaControl { .. } | AlarmAlert { .. } => Self::Media,
            CapturePhoto | StartCamera | StopCamera => Self::Camera,
            Notify { .. } | RequestNotificationPermission => Self::Notifications,
            Locate | FetchWeather => Self::Location,
            Share { .. } => Self::Sharing,
            RefreshEnvironment | RequestPersistentStorage => Self::Environment,
            CheckForUpdates => Self::Updates,
            Platform { .. } => Self::Platform,
        }
    }
}
type Handler = Rc<dyn Fn(HostRequest) -> Result<(), String>>;
#[derive(Default)]
pub struct ServiceRegistry {
    providers: BTreeMap<HostService, (&'static str, Handler)>,
}
impl ServiceRegistry {
    pub fn register(
        &mut self,
        id: &'static str,
        service: HostService,
        handler: Handler,
    ) -> Result<(), String> {
        if service == HostService::Platform {
            return Err("Platform services are composed through PlatformPlugin.".into());
        }
        if id.trim().is_empty() || id.len() > 128 {
            return Err("Invalid host service provider identity.".into());
        }
        if self.providers.contains_key(&service) {
            return Err(format!(
                "A provider for {service:?} is already registered; replacement must be explicit."
            ));
        }
        self.providers.insert(service, (id, handler));
        Ok(())
    }
    pub fn replace(
        &mut self,
        id: &'static str,
        service: HostService,
        handler: Handler,
    ) -> Result<(), String> {
        if !self.providers.contains_key(&service) {
            return Err("There is no provider to replace.".into());
        }
        if id.trim().is_empty() || id.len() > 128 {
            return Err("Invalid host service provider identity.".into());
        }
        self.providers.insert(service, (id, handler));
        Ok(())
    }
    pub fn dispatch(&self, request: HostRequest) -> Result<(), String> {
        let service = HostService::for_request(&request);
        let handler = self
            .providers
            .get(&service)
            .map(|entry| entry.1.clone())
            .ok_or_else(|| format!("This host does not provide {service:?}."))?;
        handler(request)
    }
    pub fn providers(&self) -> impl Iterator<Item = (HostService, &'static str)> + '_ {
        self.providers.iter().map(|(kind, (id, _))| (*kind, *id))
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;
    #[test]
    fn replacement_is_explicit_and_denial_never_falls_through() {
        let mut registry = ServiceRegistry::default();
        registry
            .register(
                "denied",
                HostService::Clipboard,
                Rc::new(|_| Err("denied".into())),
            )
            .unwrap();
        let called = Rc::new(Cell::new(false));
        let capture = called.clone();
        let replacement: Handler = Rc::new(move |_| {
            capture.set(true);
            Ok(())
        });
        assert!(registry
            .register("replacement", HostService::Clipboard, replacement.clone())
            .is_err());
        assert_eq!(
            registry
                .dispatch(HostRequest::CopyText { text: "a".into() })
                .unwrap_err(),
            "denied"
        );
        assert!(!called.get());
        registry
            .replace("replacement", HostService::Clipboard, replacement)
            .unwrap();
        registry
            .dispatch(HostRequest::CopyText { text: "a".into() })
            .unwrap();
        assert!(called.get());
    }
}
