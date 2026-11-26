//! Typed native boundary, shared by immediate completions and observation events.
use phone_host::platform::{PlatformResponse, device::DeviceResponse};

pub(crate) const MAX_REPLY_BYTES: usize = 8 * 1024 * 1024;

pub(crate) fn decode(text: &str) -> Result<PlatformResponse, String> {
    if text.len() > MAX_REPLY_BYTES {
        return Err("The iOS device provider returned too much data.".into());
    }
    let response: DeviceResponse = serde_json::from_str(text)
        .map_err(|_| "The iOS device provider returned an invalid response.".to_string())?;
    response.validate()?;
    Ok(PlatformResponse::Device(response))
}

#[cfg(target_os = "ios")]
pub(crate) fn start(
    session: &std::rc::Rc<crate::pending::Session>,
    request: phone_host::platform::device::DeviceRequest,
) -> Result<phone_host::platform::PlatformTask, String> {
    request.validate()?;
    if matches!(
        request,
        phone_host::platform::device::DeviceRequest::RequestPermission {
            permission: phone_host::platform::device::DevicePermission::Notifications
        }
    ) {
        let permission = crate::native::permission(session, true)?;
        let session = session.clone();
        return Ok(Box::pin(async move {
            permission.await?;
            let snapshot = crate::native::device(&session, r#"{"kind":"snapshot"}"#, true)?.await?;
            decode(&snapshot.message)
        }));
    }
    let json = serde_json::to_string(&request).map_err(|error| error.to_string())?;
    let task = crate::native::device(session, &json, request.observation_channel().is_some())?;
    Ok(Box::pin(async move {
        let response = task.await?;
        if response.code != 4 {
            return Err("The iOS device provider returned an unexpected completion.".into());
        }
        decode(&response.message)
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use phone_host::platform::device::{CompositionOutcome, DevicePermission, PermissionState};

    #[test]
    fn pending_authorization_survives_the_native_boundary_without_granting_private_data() {
        let PlatformResponse::Device(DeviceResponse::Snapshot(snapshot)) = decode(
            r#"{"kind":"snapshot","value":{"permissions":[{"permission":"location","status":"unknown"},{"permission":"contacts","status":"unknown"}],"operations":["readContacts"]}}"#,
        ).unwrap() else { panic!("expected snapshot") };
        assert_eq!(
            snapshot.permission(DevicePermission::Location),
            PermissionState::Unknown
        );
        assert_ne!(
            snapshot.permission(DevicePermission::Location),
            PermissionState::Prompt
        );
        let PlatformResponse::Device(contacts) = decode(
            r#"{"kind":"contacts","value":[{"id":"native:1","name":"Not yet authorized"}]}"#,
        )
        .unwrap() else {
            panic!("expected contacts")
        };
        assert!(contacts.admit(&snapshot).is_err());
        assert!(DeviceResponse::Contacts(vec![]).admit(&snapshot).is_ok());
    }

    #[test]
    fn limited_contacts_and_actual_composer_outcomes_survive_the_native_boundary() {
        let PlatformResponse::Device(DeviceResponse::Snapshot(snapshot)) = decode(
            r#"{"kind":"snapshot","value":{"permissions":[{"permission":"contacts","status":"limited"}]}}"#,
        ).unwrap() else { panic!("expected snapshot") };
        assert_eq!(
            snapshot.permission(DevicePermission::Contacts),
            PermissionState::Limited
        );
        for (wire, expected) in [
            ("sent", CompositionOutcome::Sent),
            ("cancelled", CompositionOutcome::Cancelled),
            ("failed", CompositionOutcome::Failed),
        ] {
            let json = format!(r#"{{"kind":"messageComposed","value":{{"outcome":"{wire}"}}}}"#);
            let PlatformResponse::Device(DeviceResponse::MessageComposed { outcome }) =
                decode(&json).unwrap()
            else {
                panic!("expected composer result")
            };
            assert_eq!(outcome, expected);
        }
    }

    #[test]
    fn native_catalogs_cannot_smuggle_duplicate_identities_or_invalid_control_values() {
        assert!(decode(r#"{"kind":"contacts","value":[{"id":"one"},{"id":"one"}]}"#).is_err());
        assert!(decode(r#"{"kind":"snapshot","value":{"controls":[{"control":"brightness","access":"writable","value":{"kind":"level","value":2.0}}]}}"#).is_err());
        assert!(decode(r#"{"kind":"messageComposed","value":{"outcome":"delivered"}}"#).is_err());
    }

    #[test]
    fn picker_outcomes_retain_draft_identity_and_require_opaque_attachment_leases() {
        let PlatformResponse::Device(DeviceResponse::DraftAttachments { client_id, attachments, error }) = decode(
            r#"{"kind":"draftAttachments","value":{"client_id":"draft-1","attachments":null,"error":null}}"#,
        ).unwrap() else { panic!("expected a correlated picker cancellation") };
        assert_eq!(client_id, "draft-1");
        assert!(attachments.is_none() && error.is_none());
        assert!(decode(r#"{"kind":"draftAttachments","value":{"client_id":"draft-1","attachments":[{"id":"draft-attachment:one","name":"photo.png","mime":"image/png","bytes":42}],"error":null}}"#).is_ok());
        assert!(decode(r#"{"kind":"draftAttachments","value":{"client_id":"draft-1","attachments":[{"id":"/private/photo.png","name":"photo.png","mime":"image/png"}],"error":null}}"#).is_err());
        assert!(decode(r#"{"kind":"draftAttachments","value":[]}"#).is_err());
    }
}
