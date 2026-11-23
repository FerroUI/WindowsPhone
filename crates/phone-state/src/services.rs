use phone_domain::PhoneData;
use serde::{Deserialize, Serialize};
use std::rc::Rc;

pub type SaveCallback = Rc<dyn Fn(&PhoneData)>;

/// Observed host facts are ephemeral. An absent value is not fabricated as
/// connectivity or a full battery, and never overwrites the phone settings.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct HostEnvironment {
    pub battery_percent: Option<u8>,
    pub charging: Option<bool>,
    pub online: Option<bool>,
}

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
        folder: Option<phone_domain::Id>,
    },
    ExportDocument {
        id: phone_domain::Id,
    },
    ShareDocuments {
        ids: Vec<phone_domain::Id>,
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

#[derive(Clone)]
pub struct HostServices {
    pub request: Rc<dyn Fn(HostRequest)>,
}
impl Default for HostServices {
    fn default() -> Self {
        Self {
            request: Rc::new(|_| {}),
        }
    }
}
