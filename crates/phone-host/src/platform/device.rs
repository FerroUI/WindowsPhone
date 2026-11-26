//! Shared, bounded OS contracts. Device observations stay outside phone backups.
//! A capability means a provider implements the protocol; each operation still
//! checks current role, permission, hardware and foreground eligibility.
//! Native timestamps are Unix milliseconds; call durations are whole seconds.
use super::{HostFeature, HostRole, SystemSettingsPage};
mod admission;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

pub const MAX_DEVICE_CONTACTS: usize = 1000;
pub const MAX_DEVICE_MESSAGES: usize = 500;
pub const MAX_DEVICE_NOTIFICATIONS: usize = 256;

/// Preferences for this application's own background message announcements.
/// This grants no OS permission and never modifies another app's channels or
/// required foreground-call presentation. Providers may persist the policy.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeviceNotificationPolicy {
    pub enabled: bool,
    pub message_center: bool,
    pub message_banners: bool,
    pub message_sound: bool,
    pub message_vibration: bool,
    pub lock_screen: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum DeviceControl {
    Wifi,
    Bluetooth,
    Cellular,
    Airplane,
    Location,
    Rotation,
    Brightness,
    AutoBrightness,
    Flashlight,
    RingerVolume,
    MediaVolume,
    AlarmVolume,
    RingerMode,
    DoNotDisturb,
    BatterySaver,
    Hotspot,
    Nfc,
    ScreenTimeout,
    NightLight,
    Cast,
    NearbyShare,
    Vpn,
    AudioOutput,
    Sync,
    DataSaver,
}
impl DeviceControl {
    pub const ALL: [Self; 25] = [
        Self::Wifi,
        Self::Bluetooth,
        Self::Cellular,
        Self::Airplane,
        Self::Location,
        Self::Rotation,
        Self::Brightness,
        Self::AutoBrightness,
        Self::Flashlight,
        Self::RingerVolume,
        Self::MediaVolume,
        Self::AlarmVolume,
        Self::RingerMode,
        Self::DoNotDisturb,
        Self::BatterySaver,
        Self::Hotspot,
        Self::Nfc,
        Self::ScreenTimeout,
        Self::NightLight,
        Self::Cast,
        Self::NearbyShare,
        Self::Vpn,
        Self::AudioOutput,
        Self::Sync,
        Self::DataSaver,
    ];
    pub fn label(self) -> &'static str {
        match self {
            Self::Wifi => "Wi-Fi",
            Self::Bluetooth => "Bluetooth",
            Self::Cellular => "cellular+SIM",
            Self::Airplane => "flight mode",
            Self::Location => "location",
            Self::Rotation => "rotation lock",
            Self::Brightness => "brightness",
            Self::AutoBrightness => "automatic brightness",
            Self::Flashlight => "flashlight",
            Self::RingerVolume => "ringer volume",
            Self::MediaVolume => "media volume",
            Self::AlarmVolume => "alarm volume",
            Self::RingerMode => "ringer",
            Self::DoNotDisturb => "quiet hours",
            Self::BatterySaver => "battery saver",
            Self::Hotspot => "internet sharing",
            Self::Nfc => "tap+send",
            Self::ScreenTimeout => "screen timeout",
            Self::NightLight => "night light",
            Self::Cast => "project my screen",
            Self::NearbyShare => "nearby sharing",
            Self::Vpn => "VPN",
            Self::AudioOutput => "audio output",
            Self::Sync => "account sync",
            Self::DataSaver => "data saver",
        }
    }
    pub fn key(self) -> &'static str {
        match self {
            Self::Wifi => "wi_fi",
            Self::Bluetooth => "bluetooth",
            Self::Cellular => "cellular",
            Self::Airplane => "airplane",
            Self::Location => "location",
            Self::Rotation => "rotation_lock",
            Self::Brightness => "brightness",
            Self::AutoBrightness => "auto_brightness",
            Self::Flashlight => "flashlight",
            Self::RingerVolume => "ringer_volume",
            Self::MediaVolume => "media_volume",
            Self::AlarmVolume => "alarm_volume",
            Self::RingerMode => "ringer_mode",
            Self::DoNotDisturb => "do_not_disturb",
            Self::BatterySaver => "battery_saver",
            Self::Hotspot => "hotspot",
            Self::Nfc => "nfc",
            Self::ScreenTimeout => "screen_timeout",
            Self::NightLight => "night_light",
            Self::Cast => "cast",
            Self::NearbyShare => "nearby_share",
            Self::Vpn => "vpn",
            Self::AudioOutput => "audio_output",
            Self::Sync => "sync",
            Self::DataSaver => "data_saver",
        }
    }
    pub fn from_key(key: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|v| v.key() == key)
    }
    pub fn settings(self) -> SystemSettingsPage {
        use SystemSettingsPage as P;
        match self {
            Self::Wifi => P::Wifi,
            Self::Bluetooth => P::Bluetooth,
            Self::Cellular => P::Cellular,
            Self::Airplane => P::Airplane,
            Self::Location => P::Location,
            Self::Rotation => P::Rotation,
            Self::Brightness | Self::AutoBrightness => P::Brightness,
            Self::Flashlight => P::Permissions,
            Self::RingerVolume | Self::MediaVolume | Self::AlarmVolume | Self::RingerMode => {
                P::Sound
            }
            Self::DoNotDisturb => P::DoNotDisturb,
            Self::BatterySaver => P::BatterySaver,
            Self::Hotspot => P::Hotspot,
            Self::Nfc => P::Nfc,
            Self::ScreenTimeout => P::ScreenTimeout,
            Self::NightLight => P::NightLight,
            Self::Cast => P::Cast,
            Self::NearbyShare => P::NearbyShare,
            Self::Vpn => P::Vpn,
            Self::AudioOutput => P::AudioOutput,
            Self::Sync => P::Sync,
            Self::DataSaver => P::DataSaver,
        }
    }
    pub fn accepts(self, value: &ControlValue) -> bool {
        match (self, value) {
            (
                Self::Brightness | Self::RingerVolume | Self::MediaVolume | Self::AlarmVolume,
                ControlValue::Level(v),
            ) => v.is_finite() && (0.0..=1.0).contains(v),
            (Self::ScreenTimeout, ControlValue::Choice(v)) => {
                ["15", "30", "60", "120", "300", "600", "1800"].contains(&v.as_str())
            }
            (Self::RingerMode, ControlValue::Choice(v)) => {
                ["normal", "vibrate", "silent"].contains(&v.as_str())
            }
            (
                Self::Wifi
                | Self::Bluetooth
                | Self::Cellular
                | Self::Airplane
                | Self::Location
                | Self::Rotation
                | Self::AutoBrightness
                | Self::Flashlight
                | Self::DoNotDisturb
                | Self::BatterySaver
                | Self::Hotspot
                | Self::Nfc
                | Self::NightLight
                | Self::Sync
                | Self::DataSaver,
                ControlValue::Toggle(_),
            ) => true,
            _ => false,
        }
    }
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ControlAccess {
    #[default]
    Unavailable,
    ReadOnly,
    Panel,
    Permission,
    Writable,
}
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "value", rename_all = "camelCase")]
pub enum ControlValue {
    #[default]
    Unknown,
    Toggle(bool),
    Level(f64),
    Choice(String),
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ControlState {
    pub control: DeviceControl,
    pub access: ControlAccess,
    pub value: ControlValue,
    #[serde(default)]
    pub detail: String,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum DeviceRole {
    Home,
    Dialer,
    Sms,
    NotificationListener,
    WriteSettings,
    DoNotDisturb,
}
impl DeviceRole {
    pub fn label(self) -> &'static str {
        match self {
            Self::Home => "default launcher",
            Self::Dialer => "default phone app",
            Self::Sms => "default messaging app",
            Self::NotificationListener => "device notifications",
            Self::WriteSettings => "change system settings",
            Self::DoNotDisturb => "quiet hours access",
        }
    }
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeviceRoleState {
    pub role: DeviceRole,
    pub status: HostRole,
    #[serde(default)]
    pub detail: String,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum DevicePermission {
    Contacts,
    CallLog,
    ReadSms,
    SendSms,
    Phone,
    Camera,
    Bluetooth,
    Location,
    Notifications,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum PermissionState {
    #[default]
    Unsupported,
    /// Supported permission whose native authorization has not been observed yet.
    Unknown,
    Prompt,
    Granted,
    Limited,
    Denied,
    Restricted,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DevicePermissionState {
    pub permission: DevicePermission,
    pub status: PermissionState,
}
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct PhoneAccount {
    pub id: String,
    pub label: String,
    pub is_default: bool,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum DeviceOperation {
    ReadContacts,
    WriteContacts,
    CallHistory,
    ActiveCalls,
    CallControl,
    Dial,
    ReadMessages,
    SendMessages,
    ComposeMessage,
    SendMultimediaMessages,
    PickMessageAttachments,
    DownloadMessages,
    DeleteMessages,
    MessageAttachments,
    ReadNotifications,
    NotificationActions,
    AppShortcuts,
    ApplicationDetails,
    UninstallApplications,
    Maps,
}
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct DeviceSnapshot {
    pub controls: Vec<ControlState>,
    pub roles: Vec<DeviceRoleState>,
    pub permissions: Vec<DevicePermissionState>,
    pub phone_accounts: Vec<PhoneAccount>,
    pub sms_subscriptions: Vec<PhoneAccount>,
    pub settings: Vec<SystemSettingsPage>,
    pub operations: Vec<DeviceOperation>,
    pub detail: String,
}
impl DeviceSnapshot {
    pub fn supports(&self, operation: DeviceOperation) -> bool {
        self.operations.contains(&operation)
    }
    pub fn control(&self, control: DeviceControl) -> Option<&ControlState> {
        self.controls.iter().find(|v| v.control == control)
    }
    pub fn role(&self, role: DeviceRole) -> HostRole {
        self.roles
            .iter()
            .find(|v| v.role == role)
            .map_or(HostRole::Unsupported, |v| v.status)
    }
    pub fn permission(&self, permission: DevicePermission) -> PermissionState {
        self.permissions
            .iter()
            .find(|v| v.permission == permission)
            .map_or(PermissionState::Unsupported, |v| v.status)
    }
}
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct DeviceContact {
    pub id: String,
    pub name: String,
    pub phones: Vec<String>,
    pub emails: Vec<String>,
    pub favorite: bool,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum CallDirection {
    #[default]
    Unknown,
    Incoming,
    Outgoing,
    Missed,
    Rejected,
    Blocked,
    Voicemail,
}
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct DeviceCallRecord {
    pub id: String,
    pub number: String,
    pub name: String,
    pub timestamp: i64,
    pub duration: u32,
    pub direction: CallDirection,
    pub account: Option<String>,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum CallState {
    #[default]
    Connecting,
    Dialing,
    Ringing,
    Active,
    Holding,
    Disconnected,
}
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct ActiveCall {
    pub id: String,
    pub number: String,
    pub name: String,
    pub state: CallState,
    pub incoming: bool,
    pub muted: bool,
    pub speaker: bool,
    pub can_hold: bool,
    pub connected_at: Option<i64>,
    pub account: Option<String>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum CallCommand {
    Answer,
    Reject,
    End,
    Hold,
    Resume,
    SetMuted { muted: bool },
    SetSpeaker { enabled: bool },
    Dtmf { tone: String },
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum MessageDelivery {
    #[default]
    Unknown,
    Draft,
    Queued,
    Sending,
    Sent,
    Delivered,
    Failed,
    Received,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum CompositionOutcome {
    Sent,
    Cancelled,
    Failed,
}
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct DeviceMessageThread {
    pub id: String,
    pub recipients: Vec<String>,
    pub title: String,
    pub snippet: String,
    pub timestamp: i64,
    pub unread: u32,
    pub has_attachments: bool,
}
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct DeviceMessage {
    pub id: String,
    pub thread_id: String,
    pub address: String,
    pub body: String,
    pub subject: String,
    pub can_download: bool,
    pub timestamp: i64,
    pub outgoing: bool,
    pub read: bool,
    pub status: MessageDelivery,
    pub subscription: Option<String>,
    pub attachment_count: u16,
}
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct NativeMessageDraft {
    pub id: String,
    pub recipients: Vec<String>,
    pub body: String,
}
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct DeviceAttachment {
    pub id: String,
    pub name: String,
    pub mime: String,
    pub bytes: Option<u64>,
}
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct DeviceNotificationAction {
    pub id: String,
    pub title: String,
    pub allows_reply: bool,
}
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct DeviceNotification {
    pub key: String,
    pub package_id: String,
    pub application: String,
    pub title: String,
    pub body: String,
    pub timestamp: i64,
    pub clearable: bool,
    pub ongoing: bool,
    pub actions: Vec<DeviceNotificationAction>,
}
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct DeviceShortcut {
    pub id: String,
    pub title: String,
    pub enabled: bool,
    pub pinned: bool,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum MapTravelMode {
    #[default]
    Driving,
    Walking,
    Cycling,
    Transit,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum DeviceRequest {
    Snapshot,
    SetControl {
        control: DeviceControl,
        value: ControlValue,
    },
    OpenControl {
        control: DeviceControl,
    },
    RequestRole {
        role: DeviceRole,
    },
    RequestPermission {
        permission: DevicePermission,
    },
    Contacts {
        query: String,
        limit: u16,
    },
    SaveContact {
        contact: DeviceContact,
    },
    DeleteContact {
        id: String,
    },
    CallHistory {
        limit: u16,
    },
    Calls,
    Dial {
        number: String,
        account: Option<String>,
    },
    CallAction {
        id: String,
        action: CallCommand,
    },
    MessageThreads {
        limit: u16,
    },
    Messages {
        thread_id: String,
        limit: u16,
    },
    ComposeMessage {
        recipients: Vec<String>,
        body: String,
    },
    ComposeMultimediaMessage {
        recipients: Vec<String>,
        subject: String,
        body: String,
        attachments: Vec<String>,
    },
    PickMessageAttachments {
        client_id: String,
    },
    DiscardMessageAttachments {
        ids: Vec<String>,
    },
    MessageAttachments {
        message_id: String,
    },
    OpenMessageAttachment {
        message_id: String,
        id: String,
    },
    DownloadMessage {
        message_id: String,
    },
    SendMultimediaMessage {
        recipients: Vec<String>,
        subject: String,
        body: String,
        attachments: Vec<String>,
        subscription: Option<String>,
        client_id: String,
    },
    SendMessage {
        recipients: Vec<String>,
        body: String,
        subscription: Option<String>,
        client_id: String,
    },
    MarkMessagesRead {
        thread_id: String,
    },
    DeleteMessages {
        thread_id: String,
        ids: Vec<String>,
    },
    Notifications,
    NotificationAction {
        key: String,
        action: Option<String>,
        reply: Option<String>,
    },
    DismissNotification {
        key: String,
    },
    DismissNotifications {
        keys: Vec<String>,
    },
    Shortcuts {
        application: String,
    },
    LaunchShortcut {
        application: String,
        id: String,
    },
    OpenApplicationSettings {
        application: String,
    },
    RequestUninstallApplication {
        application: String,
    },
    OpenMap {
        query: String,
        latitude: Option<f64>,
        longitude: Option<f64>,
        destination: bool,
        mode: MapTravelMode,
    },
}
impl DeviceRequest {
    pub fn feature(&self) -> Option<HostFeature> {
        use HostFeature as F;
        Some(match self {
            Self::Snapshot => return None,
            Self::SetControl { .. } | Self::OpenControl { .. } => F::HardwareControls,
            Self::RequestRole { role } => match role {
                DeviceRole::Home => F::LauncherRole,
                DeviceRole::Dialer => F::Telephony,
                DeviceRole::Sms => F::Messaging,
                DeviceRole::NotificationListener => F::NotificationCenter,
                DeviceRole::WriteSettings | DeviceRole::DoNotDisturb => F::HardwareControls,
            },
            Self::RequestPermission { permission } => match permission {
                DevicePermission::Contacts => F::Contacts,
                DevicePermission::CallLog | DevicePermission::Phone => F::Telephony,
                DevicePermission::ReadSms | DevicePermission::SendSms => F::Messaging,
                DevicePermission::Camera | DevicePermission::Bluetooth => F::HardwareControls,
                DevicePermission::Location => F::Location,
                DevicePermission::Notifications => F::Notifications,
            },
            Self::Contacts { .. } | Self::SaveContact { .. } | Self::DeleteContact { .. } => {
                F::Contacts
            }
            Self::CallHistory { .. }
            | Self::Calls
            | Self::Dial { .. }
            | Self::CallAction { .. } => F::Telephony,
            Self::MessageThreads { .. }
            | Self::Messages { .. }
            | Self::ComposeMessage { .. }
            | Self::ComposeMultimediaMessage { .. }
            | Self::PickMessageAttachments { .. }
            | Self::DiscardMessageAttachments { .. }
            | Self::MessageAttachments { .. }
            | Self::OpenMessageAttachment { .. }
            | Self::DownloadMessage { .. }
            | Self::SendMultimediaMessage { .. }
            | Self::SendMessage { .. }
            | Self::MarkMessagesRead { .. }
            | Self::DeleteMessages { .. } => F::Messaging,
            Self::Notifications
            | Self::NotificationAction { .. }
            | Self::DismissNotification { .. }
            | Self::DismissNotifications { .. } => F::NotificationCenter,
            Self::Shortcuts { .. } | Self::LaunchShortcut { .. } => F::AppShortcuts,
            Self::OpenApplicationSettings { .. } | Self::RequestUninstallApplication { .. } => {
                F::InstalledApps
            }
            Self::OpenMap { .. } => F::Maps,
        })
    }
    /// Coalesce only read observations. Side effects never supersede each other.
    pub fn observation_channel(&self) -> Option<u8> {
        Some(match self {
            Self::Snapshot => 2,
            Self::Contacts { .. } => 3,
            Self::CallHistory { .. } => 4,
            Self::Calls => 5,
            Self::MessageThreads { .. } => 6,
            Self::Messages { .. } => 7,
            Self::Notifications => 8,
            Self::Shortcuts { .. } => 9,
            Self::MessageAttachments { .. } => 10,
            _ => return None,
        })
    }
    pub fn validate(&self) -> Result<(), String> {
        let valid = match self {
            Self::SetControl { control, value } => control.accepts(value),
            Self::Contacts { query, limit } => line(query, 512, true) && (1..=1000).contains(limit),
            Self::SaveContact { contact } => {
                (contact.id.is_empty() || identity(&contact.id))
                    && line(&contact.name, 512, false)
                    && contact.phones.len() <= 32
                    && contact.emails.len() <= 32
                    && contact.phones.iter().all(|v| phone_number(v))
                    && contact
                        .emails
                        .iter()
                        .all(|v| line(v, 512, false) && v.contains('@'))
            }
            Self::DeleteContact { id } => identity(id),
            Self::CallHistory { limit } | Self::MessageThreads { limit } => {
                (1..=500).contains(limit)
            }
            Self::Dial { number, account } => phone_number(number) && optional_id(account),
            Self::CallAction { id, action } => {
                identity(id)
                    && match action {
                        CallCommand::Dtmf { tone } => {
                            tone.len() == 1
                                && tone
                                    .bytes()
                                    .all(|c| c.is_ascii_digit() || b"*#".contains(&c))
                        }
                        _ => true,
                    }
            }
            Self::Messages { thread_id, limit } => identity(thread_id) && (1..=500).contains(limit),
            Self::ComposeMessage { recipients, body } => {
                recipients_valid(recipients) && body_text(body, 16000)
            }
            Self::ComposeMultimediaMessage {
                recipients,
                subject,
                body,
                attachments,
            } => multimedia_valid(recipients, subject, body, attachments),
            Self::SendMultimediaMessage {
                recipients,
                subject,
                body,
                attachments,
                subscription,
                client_id,
            } => {
                multimedia_valid(recipients, subject, body, attachments)
                    && optional_id(subscription)
                    && identity(client_id)
            }
            Self::PickMessageAttachments { client_id } => identity(client_id),
            Self::DiscardMessageAttachments { ids } => !ids.is_empty() && attachment_ids(ids),
            Self::MessageAttachments { message_id } => identity(message_id),
            Self::DownloadMessage { message_id } => identity(message_id),
            Self::OpenMessageAttachment { message_id, id } => identity(message_id) && identity(id),
            Self::SendMessage {
                recipients,
                body,
                subscription,
                client_id,
            } => {
                recipients_valid(recipients)
                    && !body.trim().is_empty()
                    && body_text(body, 16000)
                    && optional_id(subscription)
                    && identity(client_id)
            }
            Self::MarkMessagesRead { thread_id } => identity(thread_id),
            Self::DeleteMessages { thread_id, ids } => {
                identity(thread_id)
                    && !ids.is_empty()
                    && ids.len() <= 500
                    && unique(ids, |v| v.as_str())
                    && ids.iter().all(|v| identity(v))
            }
            Self::NotificationAction { key, action, reply } => {
                identity(key)
                    && optional_id(action)
                    && reply.as_ref().is_none_or(|v| {
                        action.is_some() && body_text(v, 4096) && !v.trim().is_empty()
                    })
            }
            Self::DismissNotification { key } => identity(key),
            Self::DismissNotifications { keys } => {
                !keys.is_empty()
                    && keys.len() <= MAX_DEVICE_NOTIFICATIONS
                    && unique(keys, |v| v.as_str())
                    && keys.iter().all(|v| identity(v))
            }
            Self::Shortcuts { application } => identity(application),
            Self::LaunchShortcut { application, id } => identity(application) && identity(id),
            Self::OpenApplicationSettings { application }
            | Self::RequestUninstallApplication { application } => identity(application),
            Self::OpenMap {
                query,
                latitude,
                longitude,
                ..
            } => {
                line(query, 2048, true)
                    && (!query.trim().is_empty() || latitude.is_some())
                    && coordinates(*latitude, *longitude)
            }
            _ => true,
        };
        valid
            .then_some(())
            .ok_or_else(|| "Invalid or oversized device operation.".into())
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "kind", content = "value", rename_all = "camelCase")]
pub enum DeviceResponse {
    Snapshot(DeviceSnapshot),
    Contacts(Vec<DeviceContact>),
    CallHistory(Vec<DeviceCallRecord>),
    Calls(Vec<ActiveCall>),
    MessageThreads(Vec<DeviceMessageThread>),
    Messages {
        thread_id: String,
        messages: Vec<DeviceMessage>,
    },
    Notifications(Vec<DeviceNotification>),
    Shortcuts {
        application: String,
        shortcuts: Vec<DeviceShortcut>,
    },
    MessageSent {
        client_id: String,
        id: String,
        status: MessageDelivery,
    },
    SendFailed {
        client_id: String,
        message: String,
    },
    MessageComposed {
        outcome: CompositionOutcome,
    },
    Accepted {
        message: String,
    },
    ComposeDraft(NativeMessageDraft),
    DraftAttachments {
        client_id: String,
        attachments: Option<Vec<DeviceAttachment>>,
        error: Option<String>,
    },
    MessageAttachments {
        message_id: String,
        attachments: Vec<DeviceAttachment>,
    },
    /// A downloaded MMS can reveal different participants and move threads.
    /// Only the initiating conversation may elect to follow this result.
    MessageDownloaded {
        message_id: String,
        thread_id: String,
    },
}
impl DeviceResponse {
    pub fn validate(&self) -> Result<(), String> {
        let valid = match self {
            Self::Snapshot(v) => {
                v.operations.len() <= 20
                    && unique(&v.operations, |v| *v)
                    && v.settings.len() <= 48
                    && unique(&v.settings, |v| *v)
                    && v.controls.len() <= 25
                    && unique(&v.controls, |v| v.control)
                    && v.controls.iter().all(|v| {
                        body_text(&v.detail, 2048)
                            && match &v.value {
                                ControlValue::Unknown => true,
                                ControlValue::Choice(s) if v.access != ControlAccess::Writable => {
                                    line(s, 512, true)
                                }
                                value => v.control.accepts(value),
                            }
                    })
                    && v.roles.len() <= 6
                    && unique(&v.roles, |v| v.role)
                    && v.roles.iter().all(|v| body_text(&v.detail, 2048))
                    && v.permissions.len() <= 9
                    && unique(&v.permissions, |v| v.permission)
                    && v.phone_accounts.len() <= 16
                    && unique(&v.phone_accounts, |v| v.id.as_str())
                    && v.phone_accounts
                        .iter()
                        .all(|v| identity(&v.id) && line(&v.label, 512, false))
                    && v.sms_subscriptions.len() <= 16
                    && unique(&v.sms_subscriptions, |v| v.id.as_str())
                    && v.sms_subscriptions
                        .iter()
                        .all(|v| identity(&v.id) && line(&v.label, 512, false))
                    && body_text(&v.detail, 4096)
            }
            Self::Contacts(v) => {
                v.len() <= MAX_DEVICE_CONTACTS
                    && unique(v, |v| v.id.as_str())
                    && v.iter().all(|v| {
                        identity(&v.id)
                            && line(&v.name, 512, true)
                            && v.phones.len() <= 32
                            && v.emails.len() <= 32
                            && v.phones.iter().all(|v| line(v, 256, false))
                            && v.emails.iter().all(|v| line(v, 512, false))
                    })
            }
            Self::CallHistory(v) => {
                v.len() <= 500
                    && unique(v, |v| v.id.as_str())
                    && v.iter().all(|v| {
                        identity(&v.id)
                            && line(&v.number, 256, true)
                            && line(&v.name, 512, true)
                            && optional_id(&v.account)
                    })
            }
            Self::Calls(v) => {
                v.len() <= 32
                    && unique(v, |v| v.id.as_str())
                    && v.iter().all(|v| {
                        identity(&v.id)
                            && line(&v.number, 256, true)
                            && line(&v.name, 512, true)
                            && optional_id(&v.account)
                    })
            }
            Self::MessageThreads(v) => {
                v.len() <= 500
                    && unique(v, |v| v.id.as_str())
                    && v.iter().all(|v| {
                        identity(&v.id)
                            && line(&v.title, 512, true)
                            && body_text(&v.snippet, 4096)
                            && v.recipients.len() <= 32
                            && v.recipients.iter().all(|v| line(v, 256, false))
                    })
            }
            Self::Messages {
                thread_id,
                messages,
            } => {
                identity(thread_id)
                    && messages.len() <= MAX_DEVICE_MESSAGES
                    && unique(messages, |v| v.id.as_str())
                    && messages.iter().all(|v| {
                        identity(&v.id)
                            && v.thread_id == *thread_id
                            && line(&v.address, 256, true)
                            && body_text(&v.body, 65536)
                            && line(&v.subject, 512, true)
                            && optional_id(&v.subscription)
                            && v.attachment_count <= 100
                    })
            }
            Self::Notifications(v) => {
                v.len() <= MAX_DEVICE_NOTIFICATIONS
                    && unique(v, |v| v.key.as_str())
                    && v.iter().all(|v| {
                        identity(&v.key)
                            && identity(&v.package_id)
                            && line(&v.application, 512, false)
                            && body_text(&v.title, 2048)
                            && body_text(&v.body, 8192)
                            && v.actions.len() <= 8
                            && unique(&v.actions, |v| v.id.as_str())
                            && v.actions
                                .iter()
                                .all(|v| identity(&v.id) && line(&v.title, 256, false))
                    })
            }
            Self::Shortcuts {
                application,
                shortcuts,
            } => {
                identity(application)
                    && shortcuts.len() <= 64
                    && unique(shortcuts, |v| v.id.as_str())
                    && shortcuts
                        .iter()
                        .all(|v| identity(&v.id) && line(&v.title, 512, false))
            }
            Self::MessageSent { client_id, id, .. } => identity(client_id) && identity(id),
            Self::SendFailed { client_id, message } => {
                identity(client_id) && body_text(message, 4096)
            }
            Self::MessageComposed { .. } => true,
            Self::ComposeDraft(v) => {
                identity(&v.id)
                    && (v.recipients.is_empty() || recipients_valid(&v.recipients))
                    && body_text(&v.body, 16000)
            }
            Self::DraftAttachments {
                client_id,
                attachments,
                error,
            } => {
                identity(client_id)
                    && error.as_ref().is_none_or(|v| body_text(v, 4096))
                    && !(attachments.is_some() && error.is_some())
                    && attachments.as_ref().is_none_or(|v| {
                        v.len() <= 8
                            && attachments_valid(v)
                            && v.iter().all(|v| v.id.starts_with("draft-attachment:"))
                    })
            }
            Self::MessageAttachments {
                message_id,
                attachments,
            } => identity(message_id) && attachments.len() <= 100 && attachments_valid(attachments),
            Self::MessageDownloaded {
                message_id,
                thread_id,
            } => identity(message_id) && identity(thread_id),
            Self::Accepted { message } => body_text(message, 4096),
        };
        valid
            .then_some(())
            .ok_or_else(|| "The device provider returned invalid or oversized data.".into())
    }
}
fn line(v: &str, max: usize, empty: bool) -> bool {
    v.len() <= max && (empty || !v.trim().is_empty()) && !v.chars().any(char::is_control)
}
fn body_text(v: &str, max: usize) -> bool {
    v.len() <= max && !v.contains('\0')
}
fn identity(v: &str) -> bool {
    line(v, 1024, false)
}
fn optional_id(v: &Option<String>) -> bool {
    v.as_ref().is_none_or(|v| identity(v))
}
fn unique<'a, T: 'a, K: Ord>(items: &'a [T], key: impl Fn(&'a T) -> K) -> bool {
    let mut seen = BTreeSet::new();
    items.iter().all(|v| seen.insert(key(v)))
}
fn recipients_valid(v: &[String]) -> bool {
    !v.is_empty() && v.len() <= 20 && unique(v, |v| v.as_str()) && v.iter().all(|v| phone_number(v))
}
fn attachment_ids(ids: &[String]) -> bool {
    ids.len() <= 8
        && unique(ids, |v| v.as_str())
        && ids
            .iter()
            .all(|v| identity(v) && v.starts_with("draft-attachment:"))
}
fn multimedia_valid(
    recipients: &[String],
    subject: &str,
    body: &str,
    attachments: &[String],
) -> bool {
    recipients_valid(recipients)
        && line(subject, 512, true)
        && body_text(body, 16000)
        && attachment_ids(attachments)
        && (!body.trim().is_empty() || !subject.trim().is_empty() || !attachments.is_empty())
}
fn attachments_valid(values: &[DeviceAttachment]) -> bool {
    unique(values, |v| v.id.as_str())
        && values.iter().all(|v| {
            identity(&v.id)
                && line(&v.name, 512, false)
                && v.mime.len() <= 255
                && v.mime
                    .split_once('/')
                    .is_some_and(|(a, b)| !a.is_empty() && !b.is_empty())
                && v.mime
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b"/.-+_".contains(&b))
                && v.bytes.is_none_or(|b| b <= 32 * 1024 * 1024)
        })
}
pub fn phone_number(v: &str) -> bool {
    !v.trim().is_empty()
        && v.len() <= 128
        && v.bytes()
            .all(|c| c.is_ascii_digit() || b"+*# -().,;".contains(&c))
        && v.bytes().any(|c| c.is_ascii_digit() || b"*#".contains(&c))
}
fn coordinates(latitude: Option<f64>, longitude: Option<f64>) -> bool {
    match (latitude, longitude) {
        (None, None) => true,
        (Some(a), Some(b)) => {
            a.is_finite()
                && b.is_finite()
                && (-90.0..=90.0).contains(&a)
                && (-180.0..=180.0).contains(&b)
        }
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn typed_wire_format_roundtrips_without_command_strings() {
        let request = DeviceRequest::SetControl {
            control: DeviceControl::Brightness,
            value: ControlValue::Level(0.4),
        };
        let json = serde_json::to_value(&request).unwrap();
        assert_eq!(
            json,
            serde_json::json!({"kind":"setControl","control":"brightness","value":{"kind":"level","value":0.4}})
        );
        serde_json::from_value::<DeviceRequest>(json)
            .unwrap()
            .validate()
            .unwrap();
        assert!(
            serde_json::from_str::<DeviceRequest>(r#"{"kind":"exec","command":"anything"}"#)
                .is_err()
        );
    }
    #[test]
    fn controls_reject_wrong_units_and_unknown_write_values() {
        for value in [
            ControlValue::Level(f64::NAN),
            ControlValue::Level(2.0),
            ControlValue::Toggle(true),
            ControlValue::Unknown,
        ] {
            assert!(DeviceRequest::SetControl {
                control: DeviceControl::Brightness,
                value
            }
            .validate()
            .is_err());
        }
        assert!(DeviceRequest::SetControl {
            control: DeviceControl::RingerMode,
            value: ControlValue::Choice("arbitrary".into())
        }
        .validate()
        .is_err());
    }
    #[test]
    fn messages_and_calls_are_bounded_at_the_shared_boundary() {
        assert!(DeviceRequest::Dial {
            number: "tel:123?evil=true".into(),
            account: None
        }
        .validate()
        .is_err());
        assert!(DeviceRequest::CallAction {
            id: "call:1".into(),
            action: CallCommand::Dtmf { tone: "12".into() }
        }
        .validate()
        .is_err());
        let request = DeviceRequest::SendMessage {
            recipients: vec!["+48123456".into()],
            body: "hello".into(),
            subscription: None,
            client_id: "draft:1".into(),
        };
        request.validate().unwrap();
        let mut response = DeviceResponse::Messages {
            thread_id: "7".into(),
            messages: vec![DeviceMessage {
                id: "sms:1".into(),
                thread_id: "8".into(),
                ..Default::default()
            }],
        };
        assert!(response.validate().is_err());
        if let DeviceResponse::Messages { messages, .. } = &mut response {
            messages[0].thread_id = "7".into();
        }
        response.validate().unwrap();
    }
    #[test]
    fn native_catalogs_cannot_replace_each_other_with_ambiguous_ids() {
        let item = DeviceContact {
            id: "1".into(),
            name: "Person".into(),
            ..Default::default()
        };
        assert!(DeviceResponse::Contacts(vec![item.clone(), item])
            .validate()
            .is_err());
        assert!(DeviceRequest::NotificationAction {
            key: "k".into(),
            action: None,
            reply: Some("hello".into())
        }
        .validate()
        .is_err());
        assert!(DeviceRequest::OpenMap {
            query: "".into(),
            latitude: Some(91.0),
            longitude: Some(0.0),
            destination: true,
            mode: MapTravelMode::Walking
        }
        .validate()
        .is_err());
    }
}
