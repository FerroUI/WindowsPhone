use phone_domain::PhoneData;
use serde::{Deserialize, Serialize};
use std::rc::Rc;

pub type SaveCallback = Rc<dyn Fn(&PhoneData)>;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum HostRequest {
    OpenUrl {
        url: String,
    },
    Browse {
        url: String,
    },
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
