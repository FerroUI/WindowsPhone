use phone_controls::Icon;
use phone_domain::{AppId, Route};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AppCategory {
    System,
    Productivity,
    Communication,
    Media,
    Game,
    Utility,
}

/// Declared host services checked by AppContext::request. Linked Rust code is
/// trusted code; these declarations are not a process or memory sandbox.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Capability {
    All,
    Network,
    Camera,
    Location,
    Notifications,
    Media,
    Contacts,
    Calendar,
    Clipboard,
    Sharing,
    Phone,
    Messaging,
    Email,
    Files,
    Backup,
}

#[derive(Clone, Copy)]
pub struct AppManifest {
    pub id: &'static str,
    pub title: &'static str,
    pub version: &'static str,
    pub icon: Icon,
    pub accent: &'static str,
    pub category: AppCategory,
    pub capabilities: &'static [Capability],
    pub builtin: Option<AppId>,
    pub listed: bool,
}

impl AppManifest {
    pub fn new(id: &'static str, title: &'static str, icon: Icon) -> Self {
        Self {
            id,
            title,
            icon,
            version: "0.1.0",
            accent: "",
            category: AppCategory::Utility,
            capabilities: &[],
            builtin: None,
            listed: true,
        }
    }
    pub fn category(mut self, value: AppCategory) -> Self {
        self.category = value;
        self
    }
    pub fn accent(mut self, value: &'static str) -> Self {
        self.accent = value;
        self
    }
    pub fn version(mut self, value: &'static str) -> Self {
        self.version = value;
        self
    }
    pub fn capabilities(mut self, value: &'static [Capability]) -> Self {
        self.capabilities = value;
        self
    }
    pub fn listed(mut self, value: bool) -> Self {
        self.listed = value;
        self
    }
    pub fn route(&self, path: &str) -> Route {
        self.builtin
            .map(Route::App)
            .unwrap_or_else(|| Route::Extension {
                app: self.id.into(),
                path: path.into(),
            })
    }
    pub(crate) fn builtin(app: AppId) -> Self {
        let id = match app {
            AppId::Phone => "phone.phone",
            AppId::Messaging => "phone.messaging",
            AppId::People => "phone.people",
            AppId::Outlook => "phone.outlook",
            AppId::Calendar => "phone.calendar",
            AppId::Photos => "phone.photos",
            AppId::Camera => "phone.camera",
            AppId::InternetExplorer => "phone.browser",
            AppId::Music => "phone.music",
            AppId::Video => "phone.video",
            AppId::Games => "phone.games",
            AppId::Store => "phone.store",
            AppId::Settings => "phone.settings",
            AppId::Alarms => "phone.alarms",
            AppId::Calculator => "phone.calculator",
            AppId::OneNote => "phone.onenote",
            AppId::Maps => "phone.maps",
            AppId::Cortana => "phone.cortana",
            AppId::Weather => "phone.weather",
            AppId::Files => "phone.files",
            AppId::Office => "phone.office",
            AppId::BatterySaver => "phone.battery-saver",
            AppId::DataSense => "phone.data-sense",
        };
        let mut value = Self::new(id, app.label(), crate::common::app_icon(app));
        value.builtin = Some(app);
        value.accent = app.color();
        value.category = AppCategory::System;
        value.capabilities = &[Capability::All];
        value
    }
}
