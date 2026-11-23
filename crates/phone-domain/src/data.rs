use crate::*;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

pub const DATA_VERSION: u32 = 1;
pub const MAX_BACKUP_BYTES: usize = 16 * 1024 * 1024;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct PhoneData {
    pub version: u32,
    pub next_id: Id,
    pub settings: Settings,
    pub tiles: Vec<Tile>,
    pub contacts: Vec<Contact>,
    pub conversations: Vec<Conversation>,
    pub mail: Vec<Mail>,
    pub events: Vec<CalendarEvent>,
    pub notes: Vec<Note>,
    pub documents: Vec<OfficeDocument>,
    pub document_folders: Vec<DocumentFolder>,
    pub browser: BrowserData,
    pub alarms: Vec<Alarm>,
    pub photos: Vec<Photo>,
    pub notifications: Vec<Notification>,
    pub call_history: Vec<CallRecord>,
    pub bookmarks: Vec<Bookmark>,
    pub tracks: Vec<MediaTrack>,
    pub installed: Vec<AppId>,
}
impl Default for PhoneData {
    fn default() -> Self {
        Self {
            version: DATA_VERSION,
            next_id: 1_000,
            settings: Settings::default(),
            tiles: Vec::new(),
            contacts: Vec::new(),
            conversations: Vec::new(),
            mail: Vec::new(),
            events: Vec::new(),
            notes: Vec::new(),
            documents: Vec::new(),
            document_folders: Vec::new(),
            browser: BrowserData::default(),
            alarms: Vec::new(),
            photos: Vec::new(),
            notifications: Vec::new(),
            call_history: Vec::new(),
            bookmarks: Vec::new(),
            tracks: Vec::new(),
            installed: AppId::ALL.to_vec(),
        }
    }
}
impl PhoneData {
    pub fn demo() -> Self {
        crate::seed::demo()
    }
    pub fn new() -> Self {
        Self::demo()
    }
    pub fn allocate_id(&mut self) -> Id {
        let id = self.next_id;
        self.next_id = self.next_id.saturating_add(1);
        id
    }
    pub fn tile(&self, id: Id) -> Option<&Tile> {
        self.tiles
            .iter()
            .flat_map(|tile| std::iter::once(tile).chain(tile.folder_tiles.iter()))
            .find(|tile| tile.id == id)
    }
    pub fn contact(&self, id: Id) -> Option<&Contact> {
        self.contacts.iter().find(|t| t.id == id)
    }
    pub fn conversation(&self, id: Id) -> Option<&Conversation> {
        self.conversations.iter().find(|t| t.id == id)
    }
    pub fn mail(&self, id: Id) -> Option<&Mail> {
        self.mail.iter().find(|t| t.id == id)
    }
    pub fn note(&self, id: Id) -> Option<&Note> {
        self.notes.iter().find(|t| t.id == id)
    }
    pub fn document(&self, id: Id) -> Option<&OfficeDocument> {
        self.documents.iter().find(|document| document.id == id)
    }
    pub fn contact_name(&self, conversation: &Conversation) -> String {
        conversation
            .contact_id
            .and_then(|id| self.contact(id))
            .map(|c| c.name.clone())
            .unwrap_or_else(|| conversation.recipient.clone())
    }
    pub fn unread_messages(&self) -> usize {
        self.conversations.iter().filter(|c| c.unread).count()
    }
    pub fn unread_mail(&self) -> usize {
        self.mail
            .iter()
            .filter(|m| !m.read && m.folder == "inbox")
            .count()
    }
    pub fn badge(&self, app: AppId) -> usize {
        match app {
            AppId::Messaging => self.unread_messages(),
            AppId::Outlook => self.unread_mail(),
            AppId::Phone => self.call_history.iter().filter(|c| c.missed).count(),
            _ => 0,
        }
    }
    pub fn is_pinned(&self, app: AppId) -> bool {
        self.all_tiles()
            .any(|tile| !tile.is_folder() && tile.target.is_none() && tile.app == app)
    }
    pub fn to_json(&self) -> Result<String, String> {
        serde_json::to_string(self).map_err(|e| e.to_string())
    }
    pub fn from_json(json: &str) -> Result<Self, String> {
        if json.len() > MAX_BACKUP_BYTES {
            return Err("This backup exceeds the 16 MB limit.".into());
        }
        let mut data: Self = serde_json::from_str(json)
            .map_err(|e| format!("Could not read this phone backup: {e}"))?;
        data.normalize()?;
        Ok(data)
    }
    /// Validate before replacing an existing document. Unknown future versions are never rewritten.
    pub fn normalize(&mut self) -> Result<(), String> {
        if self.version != DATA_VERSION {
            return Err(format!(
                "Unsupported phone backup version {}.",
                self.version
            ));
        }
        if self.tiles.len() > 256 || self.contacts.len() > 20_000 || self.notes.len() > 10_000 {
            return Err("This backup contains too many items.".into());
        }
        let mut max_id = 0;
        macro_rules! validate_ids {
            ($values:expr, $name:literal) => {{
                let mut seen = HashSet::new();
                for item in &$values {
                    if item.id == 0 || !seen.insert(item.id) {
                        return Err(concat!("Duplicate or zero identifier in ", $name, ".").into());
                    }
                    max_id = max_id.max(item.id);
                }
            }};
        }
        validate_ids!(self.tiles, "tiles");
        validate_ids!(self.contacts, "contacts");
        validate_ids!(self.conversations, "conversations");
        validate_ids!(self.mail, "mail");
        validate_ids!(self.events, "events");
        validate_ids!(self.notes, "notes");
        validate_ids!(self.documents, "documents");
        validate_ids!(self.document_folders, "document folders");
        validate_ids!(self.alarms, "alarms");
        validate_ids!(self.photos, "photos");
        validate_ids!(self.notifications, "notifications");
        validate_ids!(self.call_history, "call history");
        validate_ids!(self.bookmarks, "bookmarks");
        validate_ids!(self.tracks, "tracks");
        for conversation in &self.conversations {
            validate_ids!(conversation.messages, "messages");
        }
        if max_id == Id::MAX || self.next_id == Id::MAX {
            return Err("The phone backup has exhausted its identifier range.".into());
        }
        self.next_id = self.next_id.max(max_id + 1).max(1_000);
        crate::documents::validate_library(&self.documents, &self.document_folders)?;
        crate::validate_calendar_events(&self.events)?;
        if self.alarms.len() > crate::MAX_ALARMS {
            return Err("The phone can hold up to 256 alarms.".into());
        }
        self.settings.normalize();
        self.browser.normalize()?;
        self.settings.volume = self.settings.volume.min(30);
        if !self.settings.brightness.is_finite() {
            self.settings.brightness = 0.8;
        }
        self.settings.brightness = self.settings.brightness.clamp(0.1, 1.0);
        if !self.settings.text_scale.is_finite() {
            self.settings.text_scale = 1.0;
        }
        self.settings.text_scale = self.settings.text_scale.clamp(0.8, 1.6);
        if self.settings.accent.len() != 7
            || !self.settings.accent.starts_with('#')
            || !self.settings.accent[1..]
                .bytes()
                .all(|v| v.is_ascii_hexdigit())
        {
            self.settings.accent = Settings::default().accent;
        }
        let mut installed = HashSet::new();
        self.installed.retain(|app| installed.insert(*app));
        for alarm in &mut self.alarms {
            alarm.hour = alarm.hour.min(23);
            alarm.minute = alarm.minute.min(59);
            alarm.weekdays.retain(|day| *day < 7);
            alarm.weekdays.sort_unstable();
            alarm.weekdays.dedup();
            if !crate::ALARM_SNOOZE_MINUTES.contains(&alarm.snooze_minutes) {
                alarm.snooze_minutes = crate::default_alarm_snooze();
            }
        }
        self.normalize_tiles()?;
        let fallback = Settings::default().quick_actions;
        let mut quick = HashSet::new();
        for (index, key) in self.settings.quick_actions.iter_mut().enumerate() {
            if ![
                "wi_fi",
                "bluetooth",
                "airplane",
                "rotation_lock",
                "location",
                "brightness",
                "camera",
            ]
            .contains(&key.as_str())
                || !quick.insert(key.clone())
            {
                *key = fallback
                    .iter()
                    .find(|item| !quick.contains(*item))
                    .cloned()
                    .unwrap_or_else(|| fallback[index].clone());
                quick.insert(key.clone());
            }
        }
        Ok(())
    }
    pub fn search(&self, query: &str) -> Vec<SearchResult> {
        let query = query.trim().to_lowercase();
        if query.is_empty() {
            return Vec::new();
        }
        let mut result = AppId::search(&query)
            .into_iter()
            .filter(|app| self.installed.contains(app))
            .map(|app| SearchResult {
                title: app.label().into(),
                subtitle: "app".into(),
                route: Route::App(app),
                app,
            })
            .collect::<Vec<_>>();
        result.extend(
            self.contacts
                .iter()
                .filter(|c| {
                    format!("{} {} {}", c.name, c.phone, c.email)
                        .to_lowercase()
                        .contains(&query)
                })
                .map(|c| SearchResult {
                    title: c.name.clone(),
                    subtitle: "people".into(),
                    route: Route::Contact(c.id),
                    app: AppId::People,
                }),
        );
        result.extend(
            self.notes
                .iter()
                .filter(|n| {
                    format!("{} {}", n.title, n.body)
                        .to_lowercase()
                        .contains(&query)
                })
                .map(|n| SearchResult {
                    title: n.title.clone(),
                    subtitle: "OneNote".into(),
                    route: Route::Note(n.id),
                    app: AppId::OneNote,
                }),
        );
        result.extend(
            self.documents
                .iter()
                .filter(|document| document.name.to_lowercase().contains(&query))
                .map(|document| SearchResult {
                    title: document.name.clone(),
                    subtitle: document.kind().label().into(),
                    route: Route::Document(document.id),
                    app: AppId::Office,
                }),
        );
        result.truncate(100);
        result
    }
}
