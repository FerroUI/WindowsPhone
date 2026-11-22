//! Atomic mail/thread selections: one projection update and one persistence write.
use phone_domain::*;
use std::collections::HashSet;

pub(crate) fn mutate(data: &mut PhoneData, action: Action) -> Result<bool, String> {
    match action {
        Action::MailBatch { ids, operation } => mail(data, ids, operation),
        Action::ConversationBatch { ids, delete } => conversations(data, ids, delete),
        Action::DeleteMessage {
            conversation,
            message,
        } => {
            let thread = data
                .conversations
                .iter_mut()
                .find(|item| item.id == conversation)
                .ok_or("This conversation no longer exists.")?;
            let before = thread.messages.len();
            thread.messages.retain(|item| item.id != message);
            Ok(before != thread.messages.len())
        }
        _ => unreachable!("communication actions are selected by the dispatcher"),
    }
}

fn mail(data: &mut PhoneData, ids: Vec<Id>, operation: MailOperation) -> Result<bool, String> {
    let ids = ids.into_iter().collect::<HashSet<_>>();
    if ids.is_empty() {
        return Ok(false);
    }
    let existing = data.mail.iter().map(|item| item.id).collect::<HashSet<_>>();
    if !ids.is_subset(&existing) {
        return Err("A selected message no longer exists.".into());
    }
    let mut changed = false;
    data.mail.retain_mut(|item| {
        if !ids.contains(&item.id) {
            return true;
        }
        match operation {
            MailOperation::Read | MailOperation::Unread => {
                let read = operation == MailOperation::Read;
                changed |= item.read != read;
                item.read = read;
            }
            MailOperation::Flag | MailOperation::Unflag => {
                let flagged = operation == MailOperation::Flag;
                changed |= item.flagged != flagged;
                item.flagged = flagged;
            }
            MailOperation::Delete => {
                changed = true;
                if item.folder == "deleted" {
                    return false;
                }
                item.folder = "deleted".into();
            }
            MailOperation::Restore => {
                if item.folder == "deleted" {
                    item.folder = "inbox".into();
                    changed = true;
                }
            }
        }
        true
    });
    if matches!(operation, MailOperation::Read | MailOperation::Delete) {
        let before = data.notifications.len();
        data.notifications
            .retain(|item| !matches!(item.route, Route::Mail(id) if ids.contains(&id)));
        changed |= before != data.notifications.len();
    }
    Ok(changed)
}

fn conversations(data: &mut PhoneData, ids: Vec<Id>, delete: bool) -> Result<bool, String> {
    let ids = ids.into_iter().collect::<HashSet<_>>();
    if ids.is_empty() {
        return Ok(false);
    }
    let existing = data
        .conversations
        .iter()
        .map(|item| item.id)
        .collect::<HashSet<_>>();
    if !ids.is_subset(&existing) {
        return Err("A selected conversation no longer exists.".into());
    }
    let mut changed = false;
    data.conversations.retain_mut(|item| {
        if !ids.contains(&item.id) {
            return true;
        }
        if delete {
            changed = true;
            return false;
        }
        changed |= item.unread;
        item.unread = false;
        true
    });
    let before = data.notifications.len();
    data.notifications
        .retain(|item| !matches!(item.route, Route::Conversation(id) if ids.contains(&id)));
    Ok(changed || before != data.notifications.len())
}
