use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum HostRequest {
    Platform {
        request: crate::platform::PlatformRequest,
    },
    HttpGet {
        request_id: u32,
        url: String,
        max_bytes: u32,
    },
    MapTile {
        request_id: u32,
        zoom: u8,
        x: u32,
        y: u32,
    },
    ChooseFile {
        request_id: u32,
        extensions: Vec<String>,
        max_bytes: u32,
    },
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
    CheckForUpdates,
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
