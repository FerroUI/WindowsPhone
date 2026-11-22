use serde::{Deserialize, Serialize};

/// The twenty accent colours offered by Windows Phone 8.1.
pub const ACCENTS: [(&str, &str); 20] = [
    ("lime", "#A4C400"),
    ("green", "#60A917"),
    ("emerald", "#008A00"),
    ("teal", "#00ABA9"),
    ("cyan", "#1BA1E2"),
    ("cobalt", "#0050EF"),
    ("indigo", "#6A00FF"),
    ("violet", "#AA00FF"),
    ("pink", "#F472D0"),
    ("magenta", "#D80073"),
    ("crimson", "#A20025"),
    ("red", "#E51400"),
    ("orange", "#FA6800"),
    ("amber", "#F0A30A"),
    ("yellow", "#E3C800"),
    ("brown", "#825A2C"),
    ("olive", "#6D8764"),
    ("steel", "#647687"),
    ("mauve", "#76608A"),
    ("taupe", "#87794E"),
];

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub accent: String,
    pub dark: bool,
    pub more_tiles: bool,
    pub wallpaper: String,
    pub wi_fi: bool,
    pub bluetooth: bool,
    pub airplane: bool,
    pub rotation_lock: bool,
    pub location: bool,
    pub battery_saver: bool,
    pub time_24h: bool,
    pub volume: u8,
    pub brightness: f64,
    pub phone_name: String,
    pub lock_timeout: u32,
    pub notifications: bool,
    pub sounds: bool,
    pub vibration: bool,
    pub text_scale: f64,
}
impl Default for Settings {
    fn default() -> Self {
        Self {
            accent: "#1BA1E2".into(),
            dark: true,
            more_tiles: false,
            wallpaper: String::new(),
            wi_fi: true,
            bluetooth: false,
            airplane: false,
            rotation_lock: true,
            location: false,
            battery_saver: false,
            time_24h: true,
            volume: 16,
            brightness: 0.8,
            phone_name: "Lumia".into(),
            lock_timeout: 60,
            notifications: true,
            sounds: true,
            vibration: true,
            text_scale: 1.0,
        }
    }
}
impl Settings {
    pub fn columns(&self) -> usize {
        if self.more_tiles {
            6
        } else {
            4
        }
    }
    pub fn foreground(&self) -> &'static str {
        if self.dark {
            "#FFFFFF"
        } else {
            "#000000"
        }
    }
    pub fn background(&self) -> &'static str {
        if self.dark {
            "#000000"
        } else {
            "#FFFFFF"
        }
    }
    pub fn secondary(&self) -> &'static str {
        if self.dark {
            "#A6A6A6"
        } else {
            "#666666"
        }
    }
    pub fn accent_name(&self) -> &str {
        ACCENTS
            .iter()
            .find(|(_, color)| color.eq_ignore_ascii_case(&self.accent))
            .map(|(name, _)| *name)
            .unwrap_or("custom")
    }
}
