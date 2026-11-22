use crate::{AppId, Id, Route};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Contact {
    pub id: Id,
    pub name: String,
    pub phone: String,
    pub email: String,
    pub company: String,
    pub favorite: bool,
    pub color: String,
}
impl Contact {
    pub fn initials(&self) -> String {
        self.name
            .split_whitespace()
            .filter_map(|word| word.chars().next())
            .take(2)
            .flat_map(char::to_uppercase)
            .collect()
    }
}
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Message {
    pub id: Id,
    pub text: String,
    pub incoming: bool,
    pub timestamp: i64,
    /// Outgoing messages remain drafts until a real messaging service confirms delivery.
    pub draft: bool,
}
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Conversation {
    pub id: Id,
    pub contact_id: Option<Id>,
    pub recipient: String,
    pub messages: Vec<Message>,
    pub unread: bool,
}
impl Conversation {
    pub fn preview(&self) -> &str {
        self.messages.last().map(|m| m.text.as_str()).unwrap_or("")
    }
    pub fn timestamp(&self) -> i64 {
        self.messages.last().map(|m| m.timestamp).unwrap_or(0)
    }
}
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Mail {
    pub id: Id,
    pub from: String,
    pub address: String,
    pub subject: String,
    pub body: String,
    pub timestamp: i64,
    pub read: bool,
    pub flagged: bool,
    pub folder: String,
}
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CalendarEvent {
    pub id: Id,
    pub title: String,
    pub date: String,
    pub time: String,
    pub duration_minutes: u32,
    pub location: String,
    pub details: String,
}
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Note {
    pub id: Id,
    pub title: String,
    pub body: String,
    pub modified: i64,
}
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Alarm {
    pub id: Id,
    pub label: String,
    pub hour: u8,
    pub minute: u8,
    pub enabled: bool,
    pub weekdays: Vec<u8>,
}
impl Alarm {
    pub fn time_text(&self) -> String {
        format!("{:02}:{:02}", self.hour, self.minute)
    }
    pub fn repeat_text(&self) -> String {
        const DAYS: [&str; 7] = ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"];
        if self.weekdays.is_empty() {
            return "Only once".into();
        }
        if self.weekdays.len() == 7 {
            return "Every day".into();
        }
        self.weekdays
            .iter()
            .filter_map(|day| DAYS.get(*day as usize))
            .copied()
            .collect::<Vec<_>>()
            .join(", ")
    }
}
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Photo {
    pub id: Id,
    pub name: String,
    pub source: String,
    pub favorite: bool,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Notification {
    pub id: Id,
    pub app: AppId,
    pub title: String,
    pub body: String,
    pub timestamp: i64,
    pub route: Route,
}
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CallRecord {
    pub id: Id,
    pub name: String,
    pub number: String,
    pub timestamp: i64,
    pub incoming: bool,
    pub missed: bool,
}
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Bookmark {
    pub id: Id,
    pub title: String,
    pub url: String,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct MediaTrack {
    pub id: Id,
    pub title: String,
    pub artist: String,
    pub source: String,
    pub duration_seconds: f64,
    #[serde(default = "audio_kind")]
    pub media_kind: String,
}
fn audio_kind() -> String {
    "audio".into()
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SearchResult {
    pub title: String,
    pub subtitle: String,
    pub route: Route,
    pub app: AppId,
}
