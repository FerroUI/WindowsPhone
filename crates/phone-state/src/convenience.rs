use crate::{HostRequest, PhoneViewModel};
use phone_domain::*;

impl PhoneViewModel {
    pub fn pin(&self, app: AppId) {
        self.dispatch(Action::Pin(app));
    }
    pub fn unpin(&self, id: Id) {
        self.dispatch(Action::Unpin(id));
    }
    pub fn resize_tile(&self, id: Id) {
        self.dispatch(Action::ResizeTile(id));
    }
    pub fn set_tile_size(&self, id: Id, size: TileSize) {
        self.dispatch(Action::SetTileSize(id, size));
    }
    pub fn move_tile(&self, id: Id, before: Option<Id>) {
        self.dispatch(Action::MoveTile { id, before });
    }
    pub fn create_folder(&self, source: Id, target: Id, name: String) {
        self.dispatch(Action::CreateFolder {
            source,
            target,
            name,
        });
    }
    pub fn rename_folder(&self, id: Id, name: String) {
        self.dispatch(Action::RenameFolder { id, name });
    }
    pub fn remove_from_folder(&self, id: Id, app: AppId) {
        self.dispatch(Action::RemoveFromFolder { id, app });
    }
    pub fn set_settings(&self, settings: Settings) {
        self.dispatch(Action::SetSettings(settings));
    }
    pub fn toggle_setting(&self, name: &str) {
        self.dispatch(Action::ToggleSetting(name.into()));
    }
    pub fn save_contact(&self, value: Contact) {
        self.dispatch(Action::SaveContact(value));
    }
    pub fn delete_contact(&self, id: Id) {
        self.dispatch(Action::DeleteContact(id));
    }
    pub fn toggle_favorite_contact(&self, id: Id) {
        self.dispatch(Action::ToggleFavoriteContact(id));
    }
    pub fn save_note(&self, value: Note) {
        self.dispatch(Action::SaveNote(value));
    }
    pub fn delete_note(&self, id: Id) {
        self.dispatch(Action::DeleteNote(id));
    }
    pub fn save_event(&self, value: CalendarEvent) {
        self.dispatch(Action::SaveEvent(value));
    }
    pub fn delete_event(&self, id: Id) {
        self.dispatch(Action::DeleteEvent(id));
    }
    pub fn save_alarm(&self, value: Alarm) {
        self.dispatch(Action::SaveAlarm(value));
    }
    pub fn delete_alarm(&self, id: Id) {
        self.dispatch(Action::DeleteAlarm(id));
    }
    pub fn toggle_alarm(&self, id: Id) {
        self.dispatch(Action::ToggleAlarm(id));
    }
    pub fn save_photo(&self, value: Photo) {
        self.dispatch(Action::SavePhoto(value));
    }
    pub fn delete_photo(&self, id: Id) {
        self.dispatch(Action::DeletePhoto(id));
    }
    pub fn toggle_favorite_photo(&self, id: Id) {
        self.dispatch(Action::ToggleFavoritePhoto(id));
    }
    pub fn add_photo(&self, name: String, source: String) {
        self.save_photo(Photo {
            id: 0,
            name,
            source,
            favorite: false,
        });
    }
    pub fn mark_mail(&self, id: Id, read: bool) {
        self.dispatch(Action::MarkMail { id, read });
    }
    pub fn toggle_flag_mail(&self, id: Id) {
        self.dispatch(Action::ToggleFlagMail(id));
    }
    pub fn delete_mail(&self, id: Id) {
        self.dispatch(Action::DeleteMail(id));
    }
    pub fn mark_conversation_read(&self, id: Id) {
        self.dispatch(Action::MarkConversationRead(id));
    }
    pub fn delete_conversation(&self, id: Id) {
        self.dispatch(Action::DeleteConversation(id));
    }
    pub fn save_conversation(&self, value: Conversation) {
        self.dispatch(Action::SaveConversation(value));
    }
    pub fn clear_notifications(&self) {
        self.dispatch(Action::ClearNotifications);
    }
    pub fn dismiss_notification(&self, id: Id) {
        self.dispatch(Action::DismissNotification(id));
    }
    pub fn dismiss_notification_group(&self, id: Id) {
        let app = self
            .data()
            .notifications
            .iter()
            .find(|item| item.id == id)
            .map(|item| item.app);
        if let Some(app) = app {
            self.dispatch(Action::DismissAppNotifications(app));
        }
    }
    pub fn add_notification(&self, value: Notification) {
        self.dispatch(Action::AddNotification(value));
    }
    pub fn add_bookmark(&self, title: String, url: String) {
        self.dispatch(Action::SaveBookmark(Bookmark { id: 0, title, url }));
    }
    pub fn delete_bookmark(&self, id: Id) {
        self.dispatch(Action::DeleteBookmark(id));
    }
    pub fn add_track(&self, title: String, source: String) {
        self.add_media(title, source, "audio".into());
    }
    pub fn add_media(&self, title: String, source: String, media_kind: String) {
        self.dispatch(Action::SaveTrack(MediaTrack {
            id: 0,
            title,
            source,
            artist: String::new(),
            duration_seconds: 0.0,
            media_kind,
        }));
    }
    pub fn delete_track(&self, id: Id) {
        self.dispatch(Action::DeleteTrack(id));
    }
    pub fn install(&self, app: AppId) {
        self.dispatch(Action::Install(app));
    }
    pub fn uninstall(&self, app: AppId) {
        self.dispatch(Action::Uninstall(app));
    }

    pub fn create_conversation(&self, recipient: String) -> Id {
        let recipient = recipient.trim().to_string();
        if recipient.is_empty() {
            self.set_status("Enter a recipient.".into());
            return 0;
        }
        if let Some(id) = self
            .data()
            .conversations
            .iter()
            .find(|c| c.recipient == recipient)
            .map(|c| c.id)
        {
            return id;
        }
        let contact_id = self
            .data()
            .contacts
            .iter()
            .find(|c| c.phone == recipient || c.name.eq_ignore_ascii_case(&recipient))
            .map(|c| c.id);
        let number = contact_id
            .and_then(|id| self.data().contact(id).map(|c| c.phone.clone()))
            .filter(|phone| !phone.is_empty())
            .unwrap_or(recipient);
        let canonical = phone_key(&number);
        if let Some(id) = self
            .data()
            .conversations
            .iter()
            .find(|conversation| phone_key(&conversation.recipient) == canonical)
            .map(|conversation| conversation.id)
        {
            return id;
        }
        let id = self.next_id();
        self.save_conversation(Conversation {
            id,
            contact_id,
            recipient: number,
            messages: Vec::new(),
            unread: false,
        });
        id
    }
    /// Stores an outgoing draft and opens the device's SMS composer. No delivery is claimed.
    pub fn send_message(&self, id: Id, text: String) {
        if text.trim().is_empty() {
            self.set_status("Type a message first.".into());
            return;
        }
        let number = self.data().conversation(id).map(|c| c.recipient.clone());
        if let Some(number) = number {
            self.dispatch(Action::DraftMessage {
                id,
                text: text.clone(),
            });
            if self
                .platform_state()
                .supports(phone_host::platform::HostFeature::Messaging)
            {
                self.request_device(phone_host::platform::DeviceRequest::ComposeMessage {
                    recipients: vec![number],
                    body: text,
                });
            } else {
                self.request(HostRequest::Sms { number, body: text });
            }
        }
    }
    /// Records the dial request; the device owns the actual cellular call.
    pub fn record_call(&self, name: String, number: String) {
        if !phone_host::platform::phone_number(&number) {
            self.set_status("Enter a valid phone number.".into());
            return;
        }
        if self
            .platform_state()
            .supports(phone_host::platform::HostFeature::Telephony)
        {
            self.request_device(phone_host::platform::DeviceRequest::Dial {
                number,
                account: None,
            });
            return;
        }
        self.dispatch(Action::AddCall(CallRecord {
            id: 0,
            name: if name.trim().is_empty() {
                number.clone()
            } else {
                name
            },
            number: number.clone(),
            timestamp: self.now(),
            incoming: false,
            missed: false,
            acknowledged: false,
        }));
        self.request(HostRequest::Dial { number });
    }
}

fn phone_key(number: &str) -> String {
    let key = number
        .chars()
        .filter(|character| character.is_ascii_digit() || *character == '+')
        .collect::<String>();
    if key.is_empty() {
        number.trim().to_lowercase()
    } else {
        key
    }
}
