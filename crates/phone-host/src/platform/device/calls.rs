use serde::{Deserialize, Serialize};

pub const MAX_MISSED_CALL_ACKNOWLEDGEMENT: u16 = 500;

/// An opaque, short-lived, one-use receipt for a bounded set of observed calls.
/// A provider must match each captured row's immutable identity on write. Calls
/// arriving after the read are never part of this acknowledgement.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MissedCallAcknowledgement {
    pub token: String,
    pub count: u16,
}

/// Total unacknowledged missed calls, independent of the displayed history page.
/// Revision is allocated before the provider query and increases for the entire
/// host lifetime, including across permission changes and observer restarts.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MissedCallActivity {
    pub revision: u64,
    pub count: u32,
    pub acknowledgement: Option<MissedCallAcknowledgement>,
}
impl MissedCallActivity {
    pub(super) fn valid(&self) -> bool {
        self.revision > 0
            && self.acknowledgement.as_ref().is_none_or(|receipt| {
                (1..=MAX_MISSED_CALL_ACKNOWLEDGEMENT).contains(&receipt.count)
                    && u32::from(receipt.count) <= self.count
                    && receipt.token.len() <= 128
                    && super::identity(&receipt.token)
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::{
        DeviceOperation as O, DeviceRequest as Q, DeviceResponse as A, DeviceSnapshot,
    };

    #[test]
    fn receipts_are_bounded_and_acknowledgement_is_never_a_read() {
        let mut value = MissedCallActivity {
            revision: 1,
            count: 900,
            acknowledgement: Some(MissedCallAcknowledgement {
                token: "receipt-1".into(),
                count: 500,
            }),
        };
        assert!(A::MissedCalls(value.clone()).validate().is_ok());
        value.acknowledgement.as_mut().unwrap().count = 501;
        assert!(A::MissedCalls(value.clone()).validate().is_err());
        value.acknowledgement.as_mut().unwrap().count = 2;
        value.count = 1;
        assert!(A::MissedCalls(value.clone()).validate().is_err());
        value.acknowledgement = None;
        value.revision = 0;
        assert!(A::MissedCalls(value).validate().is_err());
        let request = Q::AcknowledgeMissedCalls {
            token: "receipt-1".into(),
        };
        assert_eq!(request.observation_channel(), None);
        assert_eq!(Q::MissedCalls.observation_channel(), Some(11));
    }

    #[test]
    fn receipts_require_live_read_and_acknowledgement_capabilities() {
        let value = A::MissedCalls(MissedCallActivity {
            revision: 1,
            count: 1,
            acknowledgement: Some(MissedCallAcknowledgement {
                token: "receipt-1".into(),
                count: 1,
            }),
        });
        let mut snapshot = DeviceSnapshot::default();
        assert!(value.admit(&snapshot).is_err());
        snapshot.operations.push(O::ReadMissedCalls);
        assert!(value.admit(&snapshot).is_err());
        snapshot.operations.push(O::AcknowledgeMissedCalls);
        assert!(value.admit(&snapshot).is_ok());
        assert!(Q::MissedCalls.accepts_response(&value));
        assert!(Q::AcknowledgeMissedCalls {
            token: "receipt-1".into()
        }
        .accepts_response(&value));
        assert!(!Q::CallHistory { limit: 200 }.accepts_response(&value));
        let empty = A::MissedCalls(MissedCallActivity {
            revision: 2,
            count: 0,
            acknowledgement: None,
        });
        assert!(empty.admit(&DeviceSnapshot::default()).is_ok());
    }
}
