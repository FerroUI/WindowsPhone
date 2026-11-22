use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum AppId {
    Phone,
    Messaging,
    People,
    Outlook,
    Calendar,
    Photos,
    Camera,
    InternetExplorer,
    Music,
    Video,
    Games,
    Store,
    Settings,
    Alarms,
    Calculator,
    OneNote,
    Maps,
    Cortana,
    Weather,
    Files,
    Office,
    BatterySaver,
    DataSense,
}

impl AppId {
    pub const ALL: [Self; 23] = [
        Self::Alarms,
        Self::BatterySaver,
        Self::Calculator,
        Self::Calendar,
        Self::Camera,
        Self::Cortana,
        Self::DataSense,
        Self::Files,
        Self::Games,
        Self::InternetExplorer,
        Self::Maps,
        Self::Messaging,
        Self::Music,
        Self::Office,
        Self::OneNote,
        Self::Outlook,
        Self::People,
        Self::Phone,
        Self::Photos,
        Self::Settings,
        Self::Store,
        Self::Video,
        Self::Weather,
    ];
    pub fn label(self) -> &'static str {
        match self {
            Self::Phone => "Phone",
            Self::Messaging => "Messaging",
            Self::People => "People",
            Self::Outlook => "Outlook",
            Self::Calendar => "Calendar",
            Self::Photos => "Photos",
            Self::Camera => "Camera",
            Self::InternetExplorer => "Internet Explorer",
            Self::Music => "Music",
            Self::Video => "Video",
            Self::Games => "Games",
            Self::Store => "Store",
            Self::Settings => "Settings",
            Self::Alarms => "Alarms",
            Self::Calculator => "Calculator",
            Self::OneNote => "OneNote",
            Self::Maps => "Maps",
            Self::Cortana => "Cortana",
            Self::Weather => "Weather",
            Self::Files => "Files",
            Self::Office => "Office",
            Self::BatterySaver => "Battery Saver",
            Self::DataSense => "Data Sense",
        }
    }
    /// Empty means the user's accent colour; branded hubs retain their original colour.
    pub fn color(self) -> &'static str {
        match self {
            Self::Office => "#D24726",
            Self::OneNote => "#80397B",
            Self::Games => "#108A15",
            Self::Music => "#F15A24",
            Self::Video => "#D80073",
            Self::Maps => "#00A5BC",
            Self::Weather => "#1686BC",
            _ => "",
        }
    }
    pub fn icon(self) -> &'static str {
        match self {
            Self::Phone => "phone",
            Self::Messaging => "message",
            Self::People => "people",
            Self::Outlook => "mail",
            Self::Calendar => "calendar",
            Self::Photos => "photos",
            Self::Camera => "camera",
            Self::InternetExplorer => "browser",
            Self::Music => "music",
            Self::Video => "video",
            Self::Games => "games",
            Self::Store => "store",
            Self::Settings => "settings",
            Self::Alarms => "alarm",
            Self::Calculator => "calculator",
            Self::OneNote => "note",
            Self::Maps => "map",
            Self::Cortana => "cortana",
            Self::Weather => "weather",
            Self::Files => "folder",
            Self::Office => "office",
            Self::BatterySaver => "battery",
            Self::DataSense => "data",
        }
    }
    pub fn initial(self) -> char {
        self.label()
            .chars()
            .next()
            .unwrap_or('#')
            .to_ascii_lowercase()
    }
    pub fn search(query: &str) -> Vec<Self> {
        let query = query.trim().to_lowercase();
        Self::ALL
            .into_iter()
            .filter(|app| app.label().to_lowercase().contains(&query))
            .collect()
    }
}
