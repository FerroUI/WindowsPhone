//! Transient OS data, deliberately excluded from PhoneData and its backups.
use crate::PhoneViewModel;
use phone_host::{platform::*, HostRequest};
use std::collections::BTreeMap;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct DeviceStore {
    pub location: Option<(f64, f64)>,
    pub location_revision: u64,
    pub snapshot: DeviceSnapshot,
    pub contacts: Vec<DeviceContact>,
    pub call_history: Vec<DeviceCallRecord>,
    pub missed_calls: Option<MissedCallActivity>,
    pub calls: Vec<ActiveCall>,
    pub threads: Vec<DeviceMessageThread>,
    pub message_thread: Option<String>,
    pub messages: Vec<DeviceMessage>,
    pub notifications: Vec<DeviceNotification>,
    pub shortcut_application: Option<String>,
    pub shortcuts: Vec<DeviceShortcut>,
    pub shortcut_revision: u64,
    pub send_results: BTreeMap<String, (String, MessageDelivery)>,
    pub compose_draft: Option<NativeMessageDraft>,
    pub draft_attachments: Vec<DeviceAttachment>,
    pub draft_attachment_client: Option<String>,
    pub draft_attachment_cancelled: bool,
    pub draft_attachment_revision: u64,
    pub attachment_message: Option<String>,
    pub message_attachments: Vec<DeviceAttachment>,
    pub download_revision: u64,
    pub download_results: BTreeMap<String, (u64, String)>,
}
impl PhoneViewModel {
    pub fn set_device_location(&self, latitude: f64, longitude: f64) -> Result<(), String> {
        if !latitude.is_finite()
            || !longitude.is_finite()
            || !(-90.0..=90.0).contains(&latitude)
            || !(-180.0..=180.0).contains(&longitude)
        {
            return Err("Invalid device location.".into());
        }
        {
            let mut state = self.device_state.borrow_mut();
            let revision = state
                .location_revision
                .checked_add(1)
                .ok_or("Location observation identities exhausted.")?;
            state.location = Some((latitude, longitude));
            state.location_revision = revision;
        }
        self.notify();
        Ok(())
    }
    pub fn device_state(&self) -> DeviceStore {
        self.device_state.borrow().clone()
    }
    pub fn device_revision(&self) -> u64 {
        self.device_revision.get()
    }
    pub fn clear_device_call_activity(&self) {
        let changed = self.device_state.borrow_mut().missed_calls.take().is_some();
        if changed {
            self.notify();
        }
    }
    pub fn request_device(&self, request: DeviceRequest) {
        if let Err(error) = request.validate() {
            self.set_status(error);
            return;
        }
        self.request(HostRequest::Platform {
            request: PlatformRequest::Device { request },
        });
    }
    pub fn refresh_device(&self) {
        if self.platform_state().supports_device_snapshot() {
            self.request_device(DeviceRequest::Snapshot);
        }
    }
    pub fn set_device_response(&self, response: DeviceResponse) -> Result<(), String> {
        response.validate()?;
        let mut state = self.device_state.borrow().clone();
        response.admit(&state.snapshot)?;
        let epoch = self.document_epoch();
        let mut status = None;
        match response {
            DeviceResponse::Snapshot(snapshot) => {
                if matches!(
                    snapshot.permission(DevicePermission::Location),
                    PermissionState::Unknown
                        | PermissionState::Denied
                        | PermissionState::Restricted
                ) {
                    state.location = None;
                }
                // Limited access can replace the allowed subset while reads
                // remain usable. Publish permission and retirement of the old
                // rows together, before any subscriber can observe the change.
                if snapshot.permission(DevicePermission::Contacts)
                    != state.snapshot.permission(DevicePermission::Contacts)
                    || snapshot.supports(DeviceOperation::ReadContacts)
                        != state.snapshot.supports(DeviceOperation::ReadContacts)
                    || !snapshot.supports(DeviceOperation::ReadContacts)
                    || !matches!(
                        snapshot.permission(DevicePermission::Contacts),
                        PermissionState::Granted | PermissionState::Limited
                    )
                {
                    state.contacts.clear();
                }
                if !snapshot.supports(DeviceOperation::CallHistory) {
                    state.call_history.clear();
                }
                if !snapshot.supports(DeviceOperation::ReadMissedCalls) {
                    state.missed_calls = None;
                } else if !snapshot.supports(DeviceOperation::AcknowledgeMissedCalls) {
                    if let Some(value) = &mut state.missed_calls {
                        value.acknowledgement = None;
                    }
                }
                if !snapshot.supports(DeviceOperation::ActiveCalls) {
                    state.calls.clear();
                }
                if !snapshot.supports(DeviceOperation::ReadMessages) {
                    state.threads.clear();
                    state.message_thread = None;
                    state.messages.clear();
                    state.attachment_message = None;
                    state.message_attachments.clear();
                    state.download_results.clear();
                }
                if !snapshot.supports(DeviceOperation::ReadNotifications) {
                    state.notifications.clear();
                }
                if !snapshot.supports(DeviceOperation::AppShortcuts) {
                    state.shortcuts.clear();
                    state.shortcut_application = None;
                }
                if !snapshot.supports(DeviceOperation::MessageAttachments) {
                    state.attachment_message = None;
                    state.message_attachments.clear();
                }
                state.snapshot = snapshot;
            }
            DeviceResponse::Contacts(mut rows) => {
                rows.sort_by_cached_key(|v| (v.name.to_lowercase(), v.id.clone()));
                state.contacts = rows;
            }
            DeviceResponse::CallHistory(mut rows) => {
                rows.sort_by_key(|v| std::cmp::Reverse(v.timestamp));
                state.call_history = rows;
            }
            DeviceResponse::MissedCalls(value) => {
                if state
                    .missed_calls
                    .as_ref()
                    .is_some_and(|old| old.revision >= value.revision)
                {
                    return Ok(());
                }
                state.missed_calls = Some(value);
            }
            DeviceResponse::Calls(rows) => {
                state.calls = rows;
            }
            DeviceResponse::MessageThreads(mut rows) => {
                rows.sort_by_key(|v| std::cmp::Reverse(v.timestamp));
                state.threads = rows;
            }
            DeviceResponse::Messages {
                thread_id,
                mut messages,
            } => {
                messages.sort_by_key(|v| (v.timestamp, v.id.clone()));
                state.message_thread = Some(thread_id);
                state.messages = messages;
            }
            DeviceResponse::Notifications(mut rows) => {
                rows.sort_by_key(|v| std::cmp::Reverse(v.timestamp));
                state.notifications = rows;
            }
            DeviceResponse::Shortcuts {
                application,
                shortcuts,
            } => {
                state.shortcut_revision = state
                    .shortcut_revision
                    .checked_add(1)
                    .ok_or("Shortcut observation revision exhausted.")?;
                state.shortcut_application = Some(application);
                state.shortcuts = shortcuts;
            }
            DeviceResponse::MessageSent {
                client_id,
                id,
                status,
            } => {
                if state.send_results.len() >= 64 && !state.send_results.contains_key(&client_id) {
                    state.send_results.pop_first();
                }
                state.send_results.insert(client_id, (id, status));
            }
            DeviceResponse::SendFailed { client_id, message } => {
                if state.send_results.len() >= 64 {
                    state.send_results.pop_first();
                }
                state
                    .send_results
                    .insert(client_id, (String::new(), MessageDelivery::Failed));
                status = Some(message);
            }
            DeviceResponse::ComposeDraft(draft) => {
                state.compose_draft = Some(draft);
            }
            DeviceResponse::DraftAttachments {
                client_id,
                attachments,
                error,
            } => {
                state.draft_attachment_revision = state
                    .draft_attachment_revision
                    .checked_add(1)
                    .ok_or("Attachment observation revision exhausted.")?;
                state.draft_attachment_cancelled = attachments.is_none();
                state.draft_attachment_client = Some(client_id);
                state.draft_attachments = attachments.unwrap_or_default();
                status = error;
            }
            DeviceResponse::MessageAttachments {
                message_id,
                attachments,
            } => {
                state.attachment_message = Some(message_id);
                state.message_attachments = attachments;
            }
            DeviceResponse::MessageDownloaded {
                message_id,
                thread_id,
            } => {
                state.download_revision = state
                    .download_revision
                    .checked_add(1)
                    .ok_or("Message download revision exhausted.")?;
                if state.download_results.len() >= 64
                    && !state.download_results.contains_key(&message_id)
                {
                    let oldest = state
                        .download_results
                        .iter()
                        .min_by_key(|(_, (revision, _))| *revision)
                        .map(|(id, _)| id.clone());
                    if let Some(oldest) = oldest {
                        state.download_results.remove(&oldest);
                    }
                }
                state
                    .download_results
                    .insert(message_id, (state.download_revision, thread_id));
            }
            DeviceResponse::MessageComposed { outcome } => {
                self.set_status(match outcome {
                    CompositionOutcome::Sent => "Message accepted by the system composer. Delivery is managed by the carrier.",
                    CompositionOutcome::Cancelled => "Message composition cancelled. Your draft is available.",
                    CompositionOutcome::Failed => "The system composer could not send the message.",
                }.into());
                return Ok(());
            }
            DeviceResponse::Accepted { message } => {
                if !message.is_empty() {
                    self.set_status(message);
                }
                return Ok(());
            }
        }
        let revision = self
            .device_revision
            .get()
            .checked_add(1)
            .ok_or("Device observation revision exhausted.")?;
        let changed = *self.device_state.borrow() != state;
        *self.device_state.borrow_mut() = state;
        self.device_revision.set(revision);
        // A revision is observable even for an unchanged query result, which
        // allows bounded refresh/navigation flows without comparing row bytes.
        if changed || revision != 0 {
            self.notify();
        }
        if self.document_epoch() == epoch {
            if let Some(message) = status {
                self.set_status(message);
            }
        }
        Ok(())
    }
    /// A hardware tile uses current observed state. No optimistic settings write.
    pub fn activate_device_control(&self, control: DeviceControl) {
        let state = self
            .device_state
            .borrow()
            .snapshot
            .control(control)
            .cloned();
        let request = match state {
            Some(ControlState {
                access: ControlAccess::Writable,
                value: ControlValue::Toggle(value),
                ..
            }) => DeviceRequest::SetControl {
                control,
                value: ControlValue::Toggle(!value),
            },
            Some(ControlState {
                access: ControlAccess::Writable,
                value: ControlValue::Choice(value),
                ..
            }) if control == DeviceControl::RingerMode => DeviceRequest::SetControl {
                control,
                value: ControlValue::Choice(
                    if value == "normal" {
                        "vibrate"
                    } else {
                        "normal"
                    }
                    .into(),
                ),
            },
            Some(ControlState {
                access: ControlAccess::Unavailable | ControlAccess::ReadOnly,
                ..
            }) => {
                self.set_status("This device control is unavailable to the current host.".into());
                return;
            }
            _ => DeviceRequest::OpenControl { control },
        };
        self.request_device(request);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ferroui_reactive::ObservableExt;
    use ferroui_reactiveui::IReactiveNotifyPropertyChanged;
    use std::{cell::Cell, rc::Rc};
    #[test]
    fn pending_location_authorization_retires_the_last_observed_position_atomically() {
        let vm = PhoneViewModel::new(phone_domain::PhoneData::demo(), Rc::new(|_| {}));
        vm.set_device_location(50.0, 18.0).unwrap();
        assert!(vm.device_state().location.is_some());
        let weak = Rc::downgrade(&vm);
        let subscription = vm.changed().subscribe_next(move |_| {
            let state = weak.upgrade().unwrap().device_state();
            assert_eq!(
                state.snapshot.permission(DevicePermission::Location),
                PermissionState::Unknown
            );
            assert!(state.location.is_none());
        });
        vm.set_device_response(DeviceResponse::Snapshot(DeviceSnapshot {
            permissions: vec![DevicePermissionState {
                permission: DevicePermission::Location,
                status: PermissionState::Unknown,
            }],
            ..Default::default()
        }))
        .unwrap();
        subscription.dispose();
        assert!(vm.device_state().location.is_none());
    }

    #[test]
    fn changed_contact_subsets_are_retired_in_the_same_snapshot_publication() {
        let vm = PhoneViewModel::new(phone_domain::PhoneData::demo(), Rc::new(|_| {}));
        let snapshot = |status| DeviceSnapshot {
            permissions: vec![DevicePermissionState {
                permission: DevicePermission::Contacts,
                status,
            }],
            operations: vec![DeviceOperation::ReadContacts],
            ..Default::default()
        };
        vm.set_device_response(DeviceResponse::Snapshot(snapshot(PermissionState::Granted)))
            .unwrap();
        vm.set_device_response(DeviceResponse::Contacts(vec![DeviceContact {
            id: "contact:private".into(),
            name: "Former allowed subset".into(),
            ..Default::default()
        }]))
        .unwrap();
        let publications = Rc::new(Cell::new(0));
        let count = publications.clone();
        let weak = Rc::downgrade(&vm);
        let subscription = vm.changed().subscribe_next(move |_| {
            let state = weak.upgrade().unwrap().device_state();
            assert_eq!(
                state.snapshot.permission(DevicePermission::Contacts),
                PermissionState::Limited
            );
            assert!(
                state.contacts.is_empty(),
                "the former full-access rows escaped in a notification"
            );
            count.set(count.get() + 1);
        });
        vm.set_device_response(DeviceResponse::Snapshot(snapshot(PermissionState::Limited)))
            .unwrap();
        subscription.dispose();
        assert_eq!(publications.get(), 1);
    }
    #[test]
    fn observed_personal_data_is_transient_atomic_and_cleared_on_revocation_or_restore() {
        let saves = Rc::new(Cell::new(0));
        let record = saves.clone();
        let vm = PhoneViewModel::new(
            phone_domain::PhoneData::demo(),
            Rc::new(move |_| record.set(record.get() + 1)),
        );
        let original = vm.data().to_json().unwrap();
        let permitted = DeviceSnapshot {
            permissions: vec![DevicePermissionState {
                permission: DevicePermission::Contacts,
                status: PermissionState::Granted,
            }],
            operations: vec![DeviceOperation::ReadContacts],
            ..Default::default()
        };
        vm.set_device_response(DeviceResponse::Snapshot(permitted.clone()))
            .unwrap();
        let contact = DeviceContact {
            id: "device:7".into(),
            name: "Native person".into(),
            phones: vec!["123".into()],
            ..Default::default()
        };
        vm.set_device_response(DeviceResponse::Contacts(vec![contact.clone()]))
            .unwrap();
        let revision = vm.device_revision();
        assert!(vm
            .set_device_response(DeviceResponse::Contacts(vec![contact.clone(), contact]))
            .is_err());
        assert_eq!(vm.device_revision(), revision);
        assert_eq!(vm.data().to_json().unwrap(), original);
        assert_eq!(saves.get(), 0);
        vm.set_device_response(DeviceResponse::Snapshot(DeviceSnapshot::default()))
            .unwrap();
        assert!(vm.device_state().contacts.is_empty());
        assert!(vm
            .set_device_response(DeviceResponse::Contacts(vec![DeviceContact {
                id: "late".into(),
                ..Default::default()
            }]))
            .is_err());
        vm.set_device_response(DeviceResponse::Snapshot(permitted))
            .unwrap();
        vm.set_device_response(DeviceResponse::Contacts(vec![DeviceContact {
            id: "8".into(),
            ..Default::default()
        }]))
        .unwrap();
        vm.replace_data_after_save(phone_domain::PhoneData::demo(), |_| Ok(()))
            .unwrap();
        assert!(vm.device_state().contacts.is_empty());
    }
}
