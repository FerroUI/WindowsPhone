use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum HostRequest {
    OpenUrl {
        url: String,
    },
    Browse {
        url: String,
    },
    ClearBrowser,
    Dial {
        number: String,
    },
    Sms {
        number: String,
        body: String,
    },
    Email {
        to: String,
        subject: String,
        body: String,
    },
    RequestNotificationPermission,
    RequestPersistentStorage,
    RefreshEnvironment,
    ImportContacts,
    ExportContacts {
        ids: Vec<u64>,
    },
    ShareContacts {
        ids: Vec<u64>,
    },
    ImportCalendar,
    ExportCalendar {
        ids: Vec<u64>,
    },
    ShareCalendar {
        ids: Vec<u64>,
    },
    ImportPhotos,
    CapturePhoto,
    StartCamera,
    StopCamera,
    ImportMusic,
    ImportVideo,
    PlayMedia {
        source: String,
    },
    MediaControl {
        action: String,
    },
    DeleteMedia {
        source: String,
    },
    ExportBackup,
    ImportBackup,
    ImportDocuments {
        folder: Option<u64>,
    },
    ExportDocument {
        id: u64,
    },
    ShareDocuments {
        ids: Vec<u64>,
    },
    AlarmAlert {
        active: bool,
        volume: u8,
        vibration: bool,
    },
    CopyText {
        text: String,
    },
    Locate,
    FetchWeather,
    Share {
        title: String,
        text: String,
        url: String,
    },
    Notify {
        title: String,
        body: String,
    },
}
