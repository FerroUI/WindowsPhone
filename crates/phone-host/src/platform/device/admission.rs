//! Request/result identity and live-capability admission. Validation of row
//! contents is separate so providers can validate before entering the UI thread.
use super::*;

impl DeviceRequest {
    pub fn accepts_response(&self, response: &DeviceResponse) -> bool {
        use DeviceRequest as Q;
        use DeviceResponse as A;
        match response {
            A::Snapshot(_) => matches!(self, Q::Snapshot) || self.observation_channel().is_none(),
            A::Contacts(_) => matches!(
                self,
                Q::Contacts { .. } | Q::SaveContact { .. } | Q::DeleteContact { .. }
            ),
            A::CallHistory(_) => matches!(self, Q::CallHistory { .. }),
            A::Calls(_) => matches!(self, Q::Calls | Q::CallAction { .. } | Q::Dial { .. }),
            A::MessageThreads(_) => matches!(
                self,
                Q::MessageThreads { .. } | Q::MarkMessagesRead { .. } | Q::DeleteMessages { .. }
            ),
            A::Messages { thread_id, .. } => match self {
                Q::Messages {
                    thread_id: expected,
                    ..
                }
                | Q::MarkMessagesRead {
                    thread_id: expected,
                }
                | Q::DeleteMessages {
                    thread_id: expected,
                    ..
                } => thread_id == expected,
                _ => false,
            },
            A::Notifications(_) => matches!(
                self,
                Q::Notifications
                    | Q::NotificationAction { .. }
                    | Q::DismissNotification { .. }
                    | Q::DismissNotifications { .. }
            ),
            A::Shortcuts { application, .. } => {
                matches!(self,Q::Shortcuts{application:expected} if application==expected)
            }
            A::MessageSent { client_id, .. } | A::SendFailed { client_id, .. } => {
                matches!(self,Q::SendMessage{client_id:expected,..}|Q::SendMultimediaMessage{client_id:expected,..} if client_id==expected)
            }
            A::MessageComposed { .. } => matches!(
                self,
                Q::ComposeMessage { .. } | Q::ComposeMultimediaMessage { .. }
            ),
            A::ComposeDraft(_) => false,
            A::DraftAttachments { client_id, .. } => {
                matches!(self,Q::PickMessageAttachments{client_id:expected} if client_id==expected)
            }
            A::MessageAttachments { message_id, .. } => {
                matches!(self,Q::MessageAttachments{message_id:expected} if message_id==expected)
            }
            A::MessageDownloaded { message_id, .. } => {
                matches!(self,Q::DownloadMessage{message_id:expected} if message_id==expected)
            }
            A::Accepted { .. } => self.observation_channel().is_none(),
        }
    }
}
impl DeviceResponse {
    /// Empty results may always remove formerly granted data. Nonempty private
    /// records require the latest usable operation, preventing a query started
    /// before role/permission revocation from repopulating the transient store.
    pub fn admit(&self, snapshot: &DeviceSnapshot) -> Result<(), String> {
        use DeviceOperation as O;
        let (operation, has_rows) = match self {
            Self::Contacts(rows) => {
                if !rows.is_empty()
                    && !matches!(
                        snapshot.permission(DevicePermission::Contacts),
                        PermissionState::Granted | PermissionState::Limited
                    )
                {
                    return Err("Device contacts access is no longer granted.".into());
                }
                (O::ReadContacts, !rows.is_empty())
            }
            Self::CallHistory(rows) => (O::CallHistory, !rows.is_empty()),
            Self::Calls(rows) => (O::ActiveCalls, !rows.is_empty()),
            Self::MessageThreads(rows) => (O::ReadMessages, !rows.is_empty()),
            Self::Messages { messages, .. } => (O::ReadMessages, !messages.is_empty()),
            Self::MessageDownloaded { .. } => (O::ReadMessages, true),
            Self::MessageAttachments { attachments, .. } => {
                (O::MessageAttachments, !attachments.is_empty())
            }
            Self::Notifications(rows) => (O::ReadNotifications, !rows.is_empty()),
            Self::Shortcuts { shortcuts, .. } => (O::AppShortcuts, !shortcuts.is_empty()),
            _ => return Ok(()),
        };
        if has_rows && !snapshot.supports(operation) {
            return Err(
                "Access to this device service has changed. Refresh its permissions and try again."
                    .into(),
            );
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn identities_are_correlated_before_delivery() {
        let request = DeviceRequest::Messages {
            thread_id: "thread-a".into(),
            limit: 20,
        };
        assert!(request.accepts_response(&DeviceResponse::Messages {
            thread_id: "thread-a".into(),
            messages: vec![]
        }));
        assert!(!request.accepts_response(&DeviceResponse::Messages {
            thread_id: "thread-b".into(),
            messages: vec![]
        }));
        assert!(!request.accepts_response(&DeviceResponse::Contacts(vec![])));
        let request = DeviceRequest::SendMessage {
            recipients: vec!["123".into()],
            body: "hello".into(),
            subscription: None,
            client_id: "send-a".into(),
        };
        assert!(request.accepts_response(&DeviceResponse::SendFailed {
            client_id: "send-a".into(),
            message: "radio unavailable".into()
        }));
        assert!(!request.accepts_response(&DeviceResponse::SendFailed {
            client_id: "send-b".into(),
            message: "radio unavailable".into()
        }));
        let request = DeviceRequest::DownloadMessage {
            message_id: "message-a".into(),
        };
        assert!(
            request.accepts_response(&DeviceResponse::MessageDownloaded {
                message_id: "message-a".into(),
                thread_id: "resolved".into()
            })
        );
        assert!(
            !request.accepts_response(&DeviceResponse::MessageDownloaded {
                message_id: "message-b".into(),
                thread_id: "resolved".into()
            })
        );
        assert!(!request.accepts_response(&DeviceResponse::Messages {
            thread_id: "resolved".into(),
            messages: vec![]
        }));
    }
    #[test]
    fn old_reads_cannot_restore_revoked_personal_data() {
        let contact = DeviceResponse::Contacts(vec![DeviceContact {
            id: "native-1".into(),
            ..Default::default()
        }]);
        let mut snapshot = DeviceSnapshot::default();
        assert!(contact.admit(&snapshot).is_err());
        snapshot.permissions.push(DevicePermissionState {
            permission: DevicePermission::Contacts,
            status: PermissionState::Limited,
        });
        assert!(contact.admit(&snapshot).is_err());
        snapshot.operations.push(DeviceOperation::ReadContacts);
        assert!(contact.admit(&snapshot).is_ok());
        snapshot.operations.clear();
        assert!(contact.admit(&snapshot).is_err());
        assert!(DeviceResponse::Contacts(vec![]).admit(&snapshot).is_ok());
    }
}
