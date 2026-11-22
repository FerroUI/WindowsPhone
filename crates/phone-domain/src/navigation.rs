use crate::{AppId, Id};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Route {
    #[default]
    Start,
    Apps,
    App(AppId),
    Conversation(Id),
    Contact(Id),
    Mail(Id),
    Note(Id),
    Document(Id),
    FileFolder(Option<Id>),
    Setting(String),
    Folder(Id),
    Search,
    Lock,
    Switcher,
    Notifications,
}

impl Route {
    pub fn app(&self) -> Option<AppId> {
        match self {
            Self::App(app) => Some(*app),
            Self::Conversation(_) => Some(AppId::Messaging),
            Self::Contact(_) => Some(AppId::People),
            Self::Mail(_) => Some(AppId::Outlook),
            Self::Note(_) => Some(AppId::OneNote),
            Self::Document(_) => Some(AppId::Office),
            Self::FileFolder(_) => Some(AppId::Files),
            Self::Setting(_) => Some(AppId::Settings),
            Self::Search => Some(AppId::Cortana),
            _ => None,
        }
    }
    pub fn is_overlay(&self) -> bool {
        matches!(
            self,
            Self::Lock | Self::Switcher | Self::Notifications | Self::Folder(_)
        )
    }
}
