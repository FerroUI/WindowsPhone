use phone_domain::*;

pub(crate) fn mutate(data: &mut PhoneData, action: Action, now: i64) -> Result<bool, String> {
    match action {
        Action::Document(action) => crate::documents::mutate(data, action, now),
        Action::Browser(action) => data.browser.apply(action, now),
        action @ (Action::MailBatch { .. }
        | Action::ConversationBatch { .. }
        | Action::DeleteMessage { .. }) => crate::communication::mutate(data, action),
        action @ (Action::Pin(_)
        | Action::PinTarget(_)
        | Action::PinApp { .. }
        | Action::Unpin(_)
        | Action::ResizeTile(_)
        | Action::SetTileSize(..)
        | Action::SetTileAppearance { .. }
        | Action::PublishTileNotification { .. }
        | Action::ClearTileNotifications { .. }
        | Action::DissolveFolder(_)
        | Action::MoveIntoFolder { .. }
        | Action::MoveTile { .. }
        | Action::MoveTileRelative { .. }
        | Action::CreateFolder { .. }
        | Action::RenameFolder { .. }
        | Action::RemoveFromFolder { .. }
        | Action::ExtractFolderTile { .. }
        | Action::MoveFolderTile { .. }) => crate::tiles::mutate_tiles(data, action, now),
        Action::SetSettings(settings) => {
            let mut validation = PhoneData {
                settings,
                ..Default::default()
            };
            validation.normalize()?;
            let changed = data.settings != validation.settings;
            data.settings = validation.settings;
            Ok(changed)
        }
        Action::ToggleSetting(name) => {
            let value = match name.as_str() {
                "wi_fi" | "wifi" | "wi-fi" => &mut data.settings.wi_fi,
                "bluetooth" => &mut data.settings.bluetooth,
                "airplane" | "flight mode" => &mut data.settings.airplane,
                "rotation_lock" | "rotation lock" => &mut data.settings.rotation_lock,
                "location" => &mut data.settings.location,
                "battery_saver" | "battery saver" => &mut data.settings.battery_saver,
                "dark" => &mut data.settings.dark,
                "more_tiles" => &mut data.settings.more_tiles,
                "time_24h" => &mut data.settings.time_24h,
                "notifications" => &mut data.settings.notifications,
                "sounds" => &mut data.settings.sounds,
                "vibration" => &mut data.settings.vibration,
                _ => return Err("This setting is not available.".into()),
            };
            *value = !*value;
            Ok(true)
        }
        Action::SaveContact(mut contact) => {
            contact.name = contact.name.trim().to_string();
            if contact.name.is_empty() {
                return Err("Enter a contact name.".into());
            }
            if contact.name.chars().count() > 256 {
                return Err("This contact name is too long.".into());
            }
            if contact.id == 0 {
                contact.id = data.allocate_id();
            }
            if contact.color.is_empty() {
                contact.color = data.settings.accent.clone();
            }
            Ok(upsert(&mut data.contacts, contact, |v| v.id))
        }
        Action::DeleteContact(id) => {
            let changed = remove(&mut data.contacts, id, |v| v.id);
            if changed {
                for conversation in &mut data.conversations {
                    if conversation.contact_id == Some(id) {
                        conversation.contact_id = None;
                    }
                }
            }
            Ok(changed)
        }
        Action::ToggleFavoriteContact(id) => Ok(data
            .contacts
            .iter_mut()
            .find(|c| c.id == id)
            .is_some_and(|c| {
                c.favorite = !c.favorite;
                true
            })),
        Action::SaveNote(mut note) => {
            note.title = note.title.trim().to_owned();
            if note.title.is_empty() {
                note.title = "Untitled note".into();
            }
            if note.body.len() > 1_048_576 {
                return Err("This note exceeds the 1 MB limit.".into());
            }
            if data
                .note(note.id)
                .is_some_and(|old| old.title == note.title && old.body == note.body)
            {
                return Ok(false);
            }
            if note.id == 0 {
                note.id = data.allocate_id();
            }
            note.modified = now;
            Ok(upsert(&mut data.notes, note, |v| v.id))
        }
        Action::DeleteNote(id) => Ok(remove(&mut data.notes, id, |v| v.id)),
        Action::Calendar(action) => crate::calendar::mutate(data, action),
        Action::SaveEvent(mut event) => {
            crate::validation::event(&event)?;
            if event.id != 0 && !data.events.iter().any(|item| item.id == event.id) {
                return Err("This appointment no longer exists.".into());
            }
            if event.id == 0 && data.events.len() >= MAX_CALENDAR_EVENTS {
                return Err(
                    "The phone calendar can hold up to 4,096 appointments and series.".into(),
                );
            }
            if event.id == 0 {
                event.id = data.allocate_id();
            }
            Ok(upsert(&mut data.events, event, |v| v.id))
        }
        Action::DeleteEvent(id) => Ok(remove(&mut data.events, id, |v| v.id)),
        Action::SaveAlarm(mut alarm) => {
            if alarm.id != 0 && !data.alarms.iter().any(|item| item.id == alarm.id) {
                return Err("This alarm no longer exists.".into());
            }
            if alarm.label.chars().count() > 256 {
                return Err("Give your alarm a name of up to 256 characters.".into());
            }
            if !ALARM_SNOOZE_MINUTES.contains(&alarm.snooze_minutes) {
                return Err("Choose a snooze time of 5, 10, 20, 30 or 60 minutes.".into());
            }
            if alarm.id == 0 && data.alarms.len() >= MAX_ALARMS {
                return Err("The phone can hold up to 256 alarms.".into());
            }
            if alarm.hour > 23 || alarm.minute > 59 {
                return Err("Choose a valid alarm time.".into());
            }
            if alarm.id == 0 {
                alarm.id = data.allocate_id();
            }
            alarm.weekdays.retain(|day| *day < 7);
            alarm.weekdays.sort_unstable();
            alarm.weekdays.dedup();
            if alarm.label.trim().is_empty() {
                alarm.label = "Alarm".into();
            }
            Ok(upsert(&mut data.alarms, alarm, |v| v.id))
        }
        Action::DeleteAlarm(id) => Ok(remove(&mut data.alarms, id, |v| v.id)),
        Action::ToggleAlarm(id) => {
            Ok(data
                .alarms
                .iter_mut()
                .find(|c| c.id == id)
                .is_some_and(|c| {
                    c.enabled = !c.enabled;
                    true
                }))
        }
        Action::SavePhoto(mut photo) => {
            if photo.source.is_empty() {
                return Err("This photo is empty.".into());
            }
            if photo.id == 0 {
                photo.id = data.allocate_id();
            }
            Ok(upsert(&mut data.photos, photo, |v| v.id))
        }
        Action::DeletePhoto(id) => {
            let source = data
                .photos
                .iter()
                .find(|photo| photo.id == id)
                .map(|photo| photo.source.clone());
            let changed = remove(&mut data.photos, id, |photo| photo.id);
            if let Some(source) = source {
                if !data.photos.iter().any(|photo| photo.source == source) {
                    if data.settings.wallpaper == source {
                        data.settings.wallpaper.clear();
                    }
                    if data.settings.lock_wallpaper == source {
                        data.settings.lock_wallpaper.clear();
                    }
                }
            }
            Ok(changed)
        }
        Action::ToggleFavoritePhoto(id) => Ok(data
            .photos
            .iter_mut()
            .find(|c| c.id == id)
            .is_some_and(|c| {
                c.favorite = !c.favorite;
                true
            })),
        Action::MarkMail { id, read } => {
            let changed = data
                .mail
                .iter_mut()
                .find(|m| m.id == id && m.read != read)
                .is_some_and(|m| {
                    m.read = read;
                    true
                });
            let notifications = data.notifications.len();
            if read {
                data.notifications.retain(|n| n.route != Route::Mail(id));
            }
            Ok(changed || notifications != data.notifications.len())
        }
        Action::ToggleFlagMail(id) => {
            Ok(data.mail.iter_mut().find(|c| c.id == id).is_some_and(|c| {
                c.flagged = !c.flagged;
                true
            }))
        }
        Action::DeleteMail(id) => crate::communication::mutate(
            data,
            Action::MailBatch {
                ids: vec![id],
                operation: MailOperation::Delete,
            },
        ),
        Action::MarkConversationRead(id) => {
            let changed = data
                .conversations
                .iter_mut()
                .find(|c| c.id == id && c.unread)
                .is_some_and(|c| {
                    c.unread = false;
                    true
                });
            let notifications = data.notifications.len();
            data.notifications
                .retain(|n| n.route != Route::Conversation(id));
            Ok(changed || notifications != data.notifications.len())
        }
        Action::DeleteConversation(id) => crate::communication::mutate(
            data,
            Action::ConversationBatch {
                ids: vec![id],
                delete: true,
            },
        ),
        Action::SaveConversation(mut value) => {
            if value.recipient.trim().is_empty() {
                return Err("Enter a recipient.".into());
            }
            if value.id == 0 {
                value.id = data.allocate_id();
            }
            Ok(upsert(&mut data.conversations, value, |v| v.id))
        }
        Action::DraftMessage { id, text } => {
            let text = text.trim();
            if text.is_empty() {
                return Err("Type a message first.".into());
            }
            if !data.conversations.iter().any(|c| c.id == id) {
                return Err("This conversation no longer exists.".into());
            }
            let message_id = data.allocate_id();
            let conversation = data
                .conversations
                .iter_mut()
                .find(|c| c.id == id)
                .expect("validated conversation");
            conversation.messages.push(Message {
                id: message_id,
                text: text.into(),
                incoming: false,
                timestamp: now,
                draft: true,
            });
            conversation.unread = false;
            Ok(true)
        }
        Action::AddCall(mut value) => {
            if value.id == 0 {
                value.id = data.allocate_id();
            }
            data.call_history.insert(0, value);
            data.call_history.truncate(500);
            Ok(true)
        }
        Action::SaveBookmark(mut value) => {
            if !crate::validation::web_url(&value.url) {
                return Err("Enter an http:// or https:// web address.".into());
            }
            if value.id == 0 {
                value.id = data.allocate_id();
            }
            if value.title.trim().is_empty() {
                value.title = value.url.clone();
            }
            Ok(upsert(&mut data.bookmarks, value, |v| v.id))
        }
        Action::DeleteBookmark(id) => Ok(remove(&mut data.bookmarks, id, |v| v.id)),
        Action::SaveTrack(mut value) => {
            if value.source.is_empty() {
                return Err("Choose an audio file.".into());
            }
            if value.id == 0 {
                value.id = data.allocate_id();
            }
            if !value.duration_seconds.is_finite() || value.duration_seconds < 0.0 {
                value.duration_seconds = 0.0;
            }
            Ok(upsert(&mut data.tracks, value, |v| v.id))
        }
        Action::DeleteTrack(id) => Ok(remove(&mut data.tracks, id, |v| v.id)),
        Action::ClearNotifications => {
            let changed = !data.notifications.is_empty();
            data.notifications.clear();
            Ok(changed)
        }
        Action::DismissNotification(id) => Ok(remove(&mut data.notifications, id, |v| v.id)),
        Action::DismissAppNotifications(app) => {
            let before = data.notifications.len();
            data.notifications.retain(|item| item.app != app);
            Ok(data.notifications.len() != before)
        }
        Action::AddNotification(mut value) => {
            if !data.settings.notifications_allowed(value.app) {
                return Ok(false);
            }
            if value.id == 0 {
                value.id = data.allocate_id();
            }
            let changed = upsert(&mut data.notifications, value, |v| v.id);
            if data.notifications.len() > 100 {
                data.notifications.drain(..data.notifications.len() - 100);
            }
            Ok(changed)
        }
        Action::SetExtensionEnabled { app, enabled } => {
            validate_app_key(&app)?;
            if enabled {
                return Ok(data.disabled_extensions.remove(&app));
            }
            if data.disabled_extensions.contains(&app) {
                return Ok(false);
            }
            if data.disabled_extensions.len() >= MAX_APP_NAMESPACES {
                return Err("Too many disabled apps.".into());
            }
            data.disabled_extensions.insert(app.clone());
            data.app_links.retain(|link| link.app != app);
            // The caller's normal cleanup removes orphaned top-level and folder tiles.
            Ok(true)
        }
        Action::Install(app) => {
            if data.installed.contains(&app) {
                Ok(false)
            } else {
                data.installed.push(app);
                Ok(true)
            }
        }
        Action::Uninstall(app) => {
            if matches!(
                app,
                AppId::Phone | AppId::Messaging | AppId::People | AppId::Settings | AppId::Store
            ) {
                return Err("This system app cannot be uninstalled.".into());
            }
            let count = data.installed.len();
            data.installed.retain(|id| *id != app);
            Ok(count != data.installed.len())
        }
        Action::Navigate(_) | Action::Back | Action::Home => Ok(false),
    }
}
fn upsert<T: PartialEq>(items: &mut Vec<T>, value: T, id: impl Fn(&T) -> Id) -> bool {
    if let Some(old) = items.iter_mut().find(|old| id(old) == id(&value)) {
        if *old == value {
            return false;
        }
        *old = value;
    } else {
        items.push(value);
    }
    true
}
fn remove<T>(items: &mut Vec<T>, id: Id, key: impl Fn(&T) -> Id) -> bool {
    let count = items.len();
    items.retain(|value| key(value) != id);
    count != items.len()
}
