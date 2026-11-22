//! Logical pixels. A four-cell Start grid is two medium Windows Phone tiles.
use std::cell::{Cell, RefCell};

pub const DEFAULT_ACCENT: &str = "#00ABA9";
pub const PHONE_WIDTH: f64 = 400.0;
pub const PAGE_MARGIN: f64 = 20.0;
pub const TILE_GAP: f64 = 8.0;
pub const STATUS_HEIGHT: f64 = 26.0;
pub const NAVIGATION_HEIGHT: f64 = 56.0;
pub const APP_BAR_HEIGHT: f64 = 62.0;
pub const MIN_TOUCH_TARGET: f64 = 44.0;
/// Segoe is used when installed by the host. Inter is bundled by FerroUI.
pub const FONT_FAMILY: &str = "Segoe UI, fonts:Inter#Inter";

#[derive(Clone, Debug, PartialEq)]
pub struct Palette {
    pub accent: String,
    pub background: &'static str,
    pub foreground: &'static str,
    pub secondary: &'static str,
    pub surface: &'static str,
    pub subtle: &'static str,
    pub light: bool,
    pub text_scale: f64,
}

impl Palette {
    pub fn new(accent: &str, light: bool) -> Self {
        let accent = if ferroui_base::media::Color::parse(accent).is_ok() {
            accent.to_owned()
        } else {
            DEFAULT_ACCENT.to_owned()
        };
        Self {
            accent,
            background: if light { "#FFFFFF" } else { "#000000" },
            foreground: if light { "#000000" } else { "#FFFFFF" },
            secondary: if light { "#666666" } else { "#A6A6A6" },
            surface: if light { "#E6E6E6" } else { "#1F1F1F" },
            subtle: if light { "#EEEEEE" } else { "#333333" },
            light,
            text_scale: text_scale(),
        }
    }
}

impl Default for Palette {
    fn default() -> Self {
        Self::new(DEFAULT_ACCENT, false)
    }
}

ferroui_base::ferro_thread_local! {
    pub(crate) static PALETTE: RefCell<Palette> = RefCell::new(Palette::default());
    static TEXT_SCALE: Cell<f64> = const {Cell::new(1.0)};
}

pub fn text_scale() -> f64 {
    TEXT_SCALE.with(Cell::get)
}
pub fn set_text_scale(value: f64) {
    TEXT_SCALE.with(|scale| {
        scale.set(if value.is_finite() {
            value.clamp(0.8, 1.6)
        } else {
            1.0
        })
    });
}

pub fn palette() -> Palette {
    PALETTE.with(|p| p.borrow().clone())
}
