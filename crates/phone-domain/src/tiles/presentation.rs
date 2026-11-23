use crate::Id;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

pub const MAX_TILE_NOTIFICATIONS: usize = 5;
pub const TILE_CARD_SECONDS: u64 = 8;
const MAX_TIME: i64 = 253_402_300_799;
const SCHEDULE_WINDOW: i64 = 30 * 86_400;

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum TileBackground {
    /// App branding, or the shared Start photograph for transparent tiles.
    #[default]
    App,
    Accent,
    Color(String),
    /// References an existing local library photo; never a path or URL.
    Photo(Id),
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct TileAppearance {
    pub label: Option<String>,
    pub background: TileBackground,
    pub show_label: bool,
    pub show_badge: bool,
    pub live: bool,
    pub queue: bool,
}
impl Default for TileAppearance {
    fn default() -> Self {
        Self {
            label: None,
            background: TileBackground::App,
            show_label: true,
            show_badge: true,
            live: true,
            queue: true,
        }
    }
}
impl TileAppearance {
    pub fn validate(&self) -> Result<(), String> {
        if let Some(label) = &self.label {
            text(label, 80, false)?;
            if label.chars().any(char::is_control) {
                return Err("A tile name must fit on one line.".into());
            }
        }
        match &self.background {
            TileBackground::Color(color)
                if color.len() != 7
                    || !color.starts_with('#')
                    || !color.as_bytes()[1..].iter().all(u8::is_ascii_hexdigit) =>
            {
                Err("Enter a tile color as #RRGGBB.".into())
            }
            TileBackground::Photo(0) => Err("Choose an existing photo.".into()),
            _ => Ok(()),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum TileBadgeGlyph {
    Attention,
    Activity,
    Alarm,
    Available,
    Away,
    Busy,
    NewMessage,
    Paused,
    Playing,
    Error,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum TileBadge {
    Count(u32),
    Glyph(TileBadgeGlyph),
}
impl TileBadge {
    pub fn accessible_text(self) -> String {
        match self {
            Self::Count(count) => format!("{count} new items"),
            Self::Glyph(glyph) => match glyph {
                TileBadgeGlyph::Attention => "attention",
                TileBadgeGlyph::Activity => "activity",
                TileBadgeGlyph::Alarm => "alarm",
                TileBadgeGlyph::Available => "available",
                TileBadgeGlyph::Away => "away",
                TileBadgeGlyph::Busy => "busy",
                TileBadgeGlyph::NewMessage => "new message",
                TileBadgeGlyph::Paused => "paused",
                TileBadgeGlyph::Playing => "playing",
                TileBadgeGlyph::Error => "error",
            }
            .into(),
        }
    }
    pub fn count_text(count: u32) -> String {
        if count > 99 {
            "99+".into()
        } else {
            count.to_string()
        }
    }
}

/// Reusable local tile templates. Image identities refer to the photo library,
/// so no tile can initiate network activity or open an arbitrary local file.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum TileTemplate {
    Text {
        title: String,
        body: String,
    },
    Flip {
        title: String,
        body: String,
        back_title: String,
        back_body: String,
    },
    Photo {
        photo: Id,
        title: String,
        body: String,
    },
    Cycle {
        photos: Vec<Id>,
        title: String,
    },
    List {
        title: String,
        lines: Vec<String>,
    },
    Progress {
        title: String,
        body: String,
        percent: u8,
    },
}
impl TileTemplate {
    pub fn validate(&self) -> Result<(), String> {
        match self {
            Self::Text { title, body } => {
                text(title, 80, true)?;
                text(body, 320, true)?;
            }
            Self::Flip {
                title,
                body,
                back_title,
                back_body,
            } => {
                text(title, 80, true)?;
                text(body, 320, true)?;
                text(back_title, 80, true)?;
                text(back_body, 320, true)?;
            }
            Self::Photo { photo, title, body } => {
                if *photo == 0 {
                    return Err("Choose an existing tile photo.".into());
                }
                text(title, 80, true)?;
                text(body, 320, true)?;
            }
            Self::Cycle { photos, title } => {
                if photos.is_empty()
                    || photos.len() > 9
                    || photos.contains(&0)
                    || photos.iter().collect::<HashSet<_>>().len() != photos.len()
                {
                    return Err("A cycle tile needs one to nine distinct photos.".into());
                }
                text(title, 80, true)?;
            }
            Self::List { title, lines } => {
                text(title, 80, true)?;
                if lines.is_empty() || lines.len() > 3 {
                    return Err("A tile list needs one to three lines.".into());
                }
                for line in lines {
                    text(line, 160, false)?;
                }
            }
            Self::Progress {
                title,
                body,
                percent,
            } => {
                text(title, 80, true)?;
                text(body, 320, true)?;
                if *percent > 100 {
                    return Err("Tile progress must be between 0 and 100.".into());
                }
            }
        }
        Ok(())
    }
    pub fn photo_ids(&self) -> Vec<Id> {
        match self {
            Self::Photo { photo, .. } => vec![*photo],
            Self::Cycle { photos, .. } => photos.clone(),
            _ => Vec::new(),
        }
    }
    pub fn accessible_text(&self) -> String {
        match self {
            Self::Text { title, body } | Self::Photo { title, body, .. } => {
                format!("{title}. {body}")
            }
            Self::Flip {
                title,
                body,
                back_title,
                back_body,
            } => format!("{title}. {body}. {back_title}. {back_body}"),
            Self::Cycle { photos, title } => format!("{title}. {} photos", photos.len()),
            Self::List { title, lines } => format!("{title}. {}", lines.join(". ")),
            Self::Progress {
                title,
                body,
                percent,
            } => format!("{title}. {body}. {percent} percent"),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TileNotification {
    /// Stable per-tile update key. Publishing the same tag replaces its card.
    pub tag: String,
    pub starts_at: i64,
    pub expires_at: Option<i64>,
    pub template: TileTemplate,
    #[serde(default)]
    pub badge: Option<TileBadge>,
}
impl TileNotification {
    pub fn new(tag: impl Into<String>, template: TileTemplate, now: i64) -> Self {
        Self {
            tag: tag.into(),
            starts_at: now,
            expires_at: None,
            template,
            badge: None,
        }
    }
    pub fn active_at(&self, now: i64) -> bool {
        self.starts_at <= now && self.expires_at.is_none_or(|expiry| now < expiry)
    }
    pub fn validate(&self) -> Result<(), String> {
        validate_tile_tag(&self.tag)?;
        if !(0..=MAX_TIME).contains(&self.starts_at)
            || self
                .expires_at
                .is_some_and(|expiry| expiry <= self.starts_at || expiry > MAX_TIME)
        {
            return Err("This tile notification has an invalid schedule or expiry.".into());
        }
        self.template.validate()
    }
    pub fn validate_publication(&self, now: i64) -> Result<(), String> {
        self.validate()?;
        if self.starts_at > now.saturating_add(SCHEDULE_WINDOW) {
            return Err("Schedule a tile notification no more than 30 days ahead.".into());
        }
        if self.expires_at.is_some_and(|expiry| expiry <= now) {
            return Err("This tile notification has already expired.".into());
        }
        Ok(())
    }
}
pub fn validate_tile_tag(tag: &str) -> Result<(), String> {
    if tag.is_empty()
        || tag.len() > 32
        || !tag
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"._-".contains(&b))
    {
        Err("Tile tags use 1–32 letters, digits, dots, underscores or hyphens.".into())
    } else {
        Ok(())
    }
}
fn text(value: &str, limit: usize, empty: bool) -> Result<(), String> {
    if (!empty && value.trim().is_empty())
        || value.chars().count() > limit
        || value.chars().any(|c| c.is_control() && c != '\n')
    {
        Err(format!(
            "Tile text must contain at most {limit} printable characters."
        ))
    } else {
        Ok(())
    }
}
