//! Foreground bootstrap of already-authorized device observations.
//!
//! These are reads, never consent requests. Message counts intentionally cover
//! the bounded recent conversation page supplied by the existing device API.
use phone_host::platform::{
    DeviceOperation, DevicePermission, DeviceRequest, DeviceResponse, DeviceSnapshot, HostFeature,
    PermissionState, PlatformState,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Seed {
    Snapshot,
    Messages,
    Notifications,
}
impl Seed {
    pub fn request(self) -> DeviceRequest {
        match self {
            Self::Snapshot => DeviceRequest::Snapshot,
            Self::Messages => DeviceRequest::MessageThreads { limit: 200 },
            Self::Notifications => DeviceRequest::Notifications,
        }
    }
    pub fn channel(self) -> u8 {
        match self {
            Self::Snapshot => 2,
            Self::Messages => 6,
            Self::Notifications => 8,
        }
    }
    pub fn for_event(response: &DeviceResponse) -> Option<Self> {
        match response {
            DeviceResponse::MessageThreads(_) | DeviceResponse::Messages { .. } => {
                Some(Self::Messages)
            }
            DeviceResponse::Notifications(_) => Some(Self::Notifications),
            _ => None,
        }
    }
}

/// Only equivalent observation reads may share an in-flight request. A larger
/// requested conversation page must never inherit a smaller page's completion.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ReadKey {
    Snapshot,
    Messages(u16),
    Notifications,
}
impl ReadKey {
    pub fn of(request: &DeviceRequest) -> Option<Self> {
        match request {
            DeviceRequest::Snapshot => Some(Self::Snapshot),
            DeviceRequest::MessageThreads { limit } => Some(Self::Messages(*limit)),
            DeviceRequest::Notifications => Some(Self::Notifications),
            _ => None,
        }
    }
    pub fn seed(self) -> Option<Seed> {
        match self {
            Self::Snapshot => Some(Seed::Snapshot),
            Self::Messages(limit) => (limit >= 200).then_some(Seed::Messages),
            Self::Notifications => Some(Seed::Notifications),
        }
    }
}

#[derive(Default)]
struct Bootstrap {
    complete: bool,
    dirty: bool,
    failures: u8,
    retry_at: Option<i64>,
}
impl Bootstrap {
    fn due(&self, now: i64) -> bool {
        !self.complete && self.retry_at.is_none_or(|time| now >= time)
    }
    fn settled(&mut self, success: bool, now: i64) {
        if success {
            *self = Self {
                // The provider may have published a newer observation while
                // this read was pending. Initial unscoped rows stay gated, and
                // a fresh read must supersede this potentially older page.
                complete: !self.dirty,
                ..Self::default()
            };
        } else {
            self.complete = false;
            self.dirty = false;
            let delay = (4_i64 << self.failures.min(3)).min(30);
            self.failures = self.failures.saturating_add(1);
            self.retry_at = Some(now.saturating_add(delay));
        }
    }
    fn clock_changed(&mut self, old: i64, now: i64) {
        if now < old {
            self.retry_at = self
                .retry_at
                .map(|time| now.saturating_add(time.saturating_sub(old).clamp(0, 30)));
        }
    }
    fn admits_rows(&mut self, usable: bool, empty: bool, refreshing: bool) -> bool {
        if usable && (!self.complete || refreshing) {
            // Empty events can be newer deletions too. Admit their cleanup,
            // while preventing an older pending seed from becoming final.
            self.dirty = true;
        }
        empty || (usable && self.complete)
    }
}

pub(crate) struct Observations {
    epoch: u64,
    pub foreground: bool,
    messages_usable: bool,
    notifications_usable: bool,
    authorities: [PermissionState; 8],
    clock: Option<i64>,
    snapshot: Bootstrap,
    messages: Bootstrap,
    notifications: Bootstrap,
}
impl Observations {
    pub fn new(epoch: u64) -> Self {
        Self {
            epoch,
            foreground: true,
            messages_usable: false,
            notifications_usable: false,
            authorities: [PermissionState::Unsupported; 8],
            clock: None,
            snapshot: Bootstrap::default(),
            messages: Bootstrap::default(),
            notifications: Bootstrap::default(),
        }
    }
    pub fn synchronize_epoch(&mut self, epoch: u64) -> bool {
        if self.epoch == epoch {
            return false;
        }
        let foreground = self.foreground;
        *self = Self::new(epoch);
        self.foreground = foreground;
        true
    }
    pub fn visibility(&mut self, foreground: bool) -> bool {
        if self.foreground == foreground {
            return false;
        }
        let epoch = self.epoch;
        *self = Self::new(epoch);
        self.foreground = foreground;
        true
    }
    pub fn invalidate(&mut self) {
        let foreground = self.foreground;
        *self = Self::new(self.epoch);
        self.foreground = foreground;
    }
    /// Return read channels whose old grant must no longer authorize delivery.
    /// Both loss and regrant invalidate pending reads before private VM mutation.
    pub fn snapshot(&mut self, value: &DeviceSnapshot, platform: &PlatformState) -> Vec<u8> {
        self.snapshot.settled(true, 0);
        let usable = |feature, operation| {
            if platform.supports(feature) && value.supports(operation) {
                PermissionState::Granted
            } else {
                PermissionState::Unsupported
            }
        };
        let contacts = value.permission(DevicePermission::Contacts);
        let mut next = [
            usable(HostFeature::Contacts, DeviceOperation::ReadContacts),
            usable(HostFeature::Telephony, DeviceOperation::CallHistory),
            usable(HostFeature::Telephony, DeviceOperation::ActiveCalls),
            usable(HostFeature::Messaging, DeviceOperation::ReadMessages),
            usable(HostFeature::Messaging, DeviceOperation::ReadMessages),
            usable(
                HostFeature::NotificationCenter,
                DeviceOperation::ReadNotifications,
            ),
            usable(HostFeature::AppShortcuts, DeviceOperation::AppShortcuts),
            usable(HostFeature::Messaging, DeviceOperation::MessageAttachments),
        ];
        next[0] = if next[0] == PermissionState::Granted
            && matches!(
                contacts,
                PermissionState::Granted | PermissionState::Limited
            ) {
            contacts
        } else {
            PermissionState::Unsupported
        };
        let messages = next[3] == PermissionState::Granted;
        let notifications = next[5] == PermissionState::Granted;
        let mut invalidated = Vec::new();
        for (index, (old, new)) in self.authorities.iter().zip(&next).enumerate() {
            if old != new {
                invalidated.push(index as u8 + 3);
            }
        }
        self.authorities = next;
        if self.messages_usable != messages {
            self.messages_usable = messages;
            self.messages = Bootstrap::default();
        }
        if self.notifications_usable != notifications {
            self.notifications_usable = notifications;
            self.notifications = Bootstrap::default();
        }
        invalidated
    }
    pub fn due(&mut self, platform: &PlatformState, now: i64) -> Vec<Seed> {
        if let Some(old) = self.clock.replace(now) {
            self.snapshot.clock_changed(old, now);
            self.messages.clock_changed(old, now);
            self.notifications.clock_changed(old, now);
        }
        if !self.foreground || !platform.supports_device_snapshot() {
            return Vec::new();
        }
        if !self.snapshot.complete {
            return self
                .snapshot
                .due(now)
                .then_some(Seed::Snapshot)
                .into_iter()
                .collect();
        }
        let mut due = Vec::new();
        if self.messages_usable && self.messages.due(now) {
            due.push(Seed::Messages);
        }
        if self.notifications_usable && self.notifications.due(now) {
            due.push(Seed::Notifications);
        }
        due
    }
    pub fn settled(&mut self, seed: Seed, success: bool, now: i64) {
        match seed {
            Seed::Snapshot => &mut self.snapshot,
            Seed::Messages => &mut self.messages,
            Seed::Notifications => &mut self.notifications,
        }
        .settled(success, now);
    }
    pub fn started(&mut self, seed: Seed) {
        // A new read covers observations preceding its dispatch. Joining an
        // existing read must not clear changes that happened during that read.
        match seed {
            Seed::Snapshot => &mut self.snapshot,
            Seed::Messages => &mut self.messages,
            Seed::Notifications => &mut self.notifications,
        }
        .dirty = false;
    }
    /// Unsolicited rows cannot refill a new grant/document before its own initial
    /// read completes. Providers additionally invalidate their queued old work.
    pub fn admits_event(&mut self, response: &DeviceResponse, refreshing: bool) -> bool {
        match response {
            DeviceResponse::MessageThreads(rows) => self.messages.admits_rows(
                self.foreground && self.messages_usable,
                rows.is_empty(),
                refreshing,
            ),
            DeviceResponse::Messages { messages, .. } => self.messages.admits_rows(
                self.foreground && self.messages_usable,
                messages.is_empty(),
                refreshing,
            ),
            DeviceResponse::MessageAttachments { attachments, .. } => {
                attachments.is_empty()
                    || (self.foreground && self.authorities[7] == PermissionState::Granted)
            }
            DeviceResponse::Notifications(rows) => self.notifications.admits_rows(
                self.foreground && self.notifications_usable,
                rows.is_empty(),
                refreshing,
            ),
            _ => true,
        }
    }
}

pub(crate) fn restrict_snapshot(value: &mut DeviceSnapshot, platform: &PlatformState) {
    value.operations.retain(|operation| {
        let feature = match operation {
            DeviceOperation::ReadContacts => HostFeature::Contacts,
            DeviceOperation::CallHistory | DeviceOperation::ActiveCalls => HostFeature::Telephony,
            DeviceOperation::ReadMessages | DeviceOperation::MessageAttachments => {
                HostFeature::Messaging
            }
            DeviceOperation::ReadNotifications => HostFeature::NotificationCenter,
            DeviceOperation::AppShortcuts => HostFeature::AppShortcuts,
            _ => return true,
        };
        platform.supports(feature)
    });
}
