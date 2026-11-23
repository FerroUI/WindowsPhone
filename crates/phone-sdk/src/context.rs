use crate::{session::Session, AppStorage, Capability};
use phone_state::{HostRequest, PhoneViewModel};
use std::rc::Rc;

#[derive(Clone)]
pub struct AppContext {
    pub vm: Rc<PhoneViewModel>,
    pub session: Rc<Session>,
    pub id: &'static str,
}
impl AppContext {
    pub fn storage(&self) -> AppStorage {
        AppStorage::new(self.vm.clone(), self.id)
    }
    pub fn navigate(&self, path: &str) {
        let result = (|| {
            phone_domain::validate_app_path(path)?;
            let registry = self.session.registry();
            let app = registry
                .by_id(self.id)
                .ok_or("Application is not registered.")?;
            require_available(&app.manifest, &self.vm)?;
            if app.manifest.builtin.is_some() && !path.is_empty() {
                return Err("This built-in app does not support a custom launch path.".into());
            }
            self.vm.navigate(app.manifest.route(path));
            Ok(())
        })();
        if let Err(error) = result {
            self.vm.set_status(error);
        }
    }
    pub fn pin(&self, path: &str, title: &str) -> Result<(), String> {
        phone_domain::validate_app_link(self.id, path, title)?;
        let registry = self.session.registry();
        let entry = registry
            .by_id(self.id)
            .ok_or("Application is not registered.")?;
        require_available(&entry.manifest, &self.vm)?;
        if let Some(app) = entry.manifest.builtin {
            if !path.is_empty() {
                return Err("This built-in app does not support a custom launch path.".into());
            }
            self.vm.pin(app);
            if self.vm.data().is_pinned(app) {
                Ok(())
            } else {
                Err(self.vm.status())
            }
        } else {
            self.vm.pin_app(self.id, path, title)
        }
    }
    pub fn choose_file(&self, options: crate::FilePickerOptions) -> crate::FileTask {
        crate::tasks::choose_file(self, options)
    }
    pub fn get(&self, url: &str, max_bytes: u32) -> crate::tasks::HttpTask {
        crate::tasks::get(self, url, max_bytes)
    }
    pub fn request(&self, request: HostRequest) -> Result<(), String> {
        let registry = self.session.registry();
        let app = registry
            .by_id(self.id)
            .ok_or("Application is not registered.")?;
        require_available(&app.manifest, &self.vm)?;
        let required = capability(&request);
        if !app.manifest.capabilities.contains(&Capability::All)
            && !app.manifest.capabilities.contains(&required)
        {
            return Err(format!(
                "Application {} does not declare {required:?}.",
                self.id
            ));
        }
        self.vm.request(request);
        Ok(())
    }
}

fn require_available(app: &crate::AppManifest, vm: &PhoneViewModel) -> Result<(), String> {
    if app.builtin.map_or_else(
        || vm.extension_enabled(app.id),
        |id| vm.data().installed.contains(&id),
    ) {
        Ok(())
    } else {
        Err("Application is unavailable or disabled.".into())
    }
}

fn capability(request: &HostRequest) -> Capability {
    use HostRequest::*;
    match request {
        StartCamera | CapturePhoto | StopCamera => Capability::Camera,
        Locate | FetchWeather => Capability::Location,
        Notify { .. } | RequestNotificationPermission | AlarmAlert { .. } => {
            Capability::Notifications
        }
        ImportContacts | ExportContacts { .. } | ShareContacts { .. } => Capability::Contacts,
        ImportCalendar | ExportCalendar { .. } | ShareCalendar { .. } => Capability::Calendar,
        Dial { .. } => Capability::Phone,
        Sms { .. } => Capability::Messaging,
        Email { .. } => Capability::Email,
        CopyText { .. } => Capability::Clipboard,
        Share { .. } | ShareDocuments { .. } => Capability::Sharing,
        PlayMedia { .. }
        | MediaControl { .. }
        | DeleteMedia { .. }
        | ImportPhotos
        | ImportMusic
        | ImportVideo => Capability::Media,
        ExportBackup | ImportBackup => Capability::Backup,
        ChooseFile { .. }
        | ImportDocuments { .. }
        | ExportDocument { .. }
        | RequestPersistentStorage
        | RefreshEnvironment => Capability::Files,
        HttpGet { .. } | CheckForUpdates | OpenUrl { .. } | Browse { .. } | ClearBrowser => {
            Capability::Network
        }
    }
}
