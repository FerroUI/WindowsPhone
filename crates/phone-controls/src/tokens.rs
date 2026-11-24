//! Logical pixels. A four-cell Start grid is two medium Windows Phone tiles.
use std::cell::{Cell, RefCell};

pub const DEFAULT_ACCENT: &str = "#00ABA9";
pub const PHONE_WIDTH: f64 = 400.0;
pub const PAGE_MARGIN: f64 = 20.0;
pub const TILE_GAP: f64 = 8.0;
pub const STATUS_HEIGHT: f64 = 26.0;
pub const NAVIGATION_HEIGHT: f64 = 56.0;
pub const APP_BAR_HEIGHT: f64 = 60.0;
pub const MIN_TOUCH_TARGET: f64 = 44.0;
/// The retained phone surface is designed at 400 logical pixels. Original
/// Silverlight/Toolkit values use a 480-pixel reference; angles and timings do
/// not scale. Keep reference geometry in one place instead of mixing units.
pub const REFERENCE_SCALE: f64 = PHONE_WIDTH / 480.0;
pub const fn reference_pixels(value: f64) -> f64 {
    value * REFERENCE_SCALE
}
pub const CONTROL_BORDER: f64 = reference_pixels(3.0);
pub const FONT_SMALL: f64 = reference_pixels(18.667);
pub const FONT_NORMAL: f64 = reference_pixels(20.0);
pub const FONT_MEDIUM: f64 = reference_pixels(22.667);
pub const FONT_MEDIUM_LARGE: f64 = reference_pixels(25.333);
pub const FONT_LARGE: f64 = reference_pixels(32.0);
pub const FONT_EXTRA_LARGE: f64 = reference_pixels(42.667);
pub const FONT_PAGE_TITLE: f64 = reference_pixels(72.0);
pub const CHOICE_INDICATOR_SIZE: f64 = reference_pixels(32.0);
pub const SWITCH_TRACK_WIDTH: f64 = reference_pixels(89.0);
pub const SWITCH_TRACK_HEIGHT: f64 = reference_pixels(34.0);
pub const SWITCH_THUMB_WIDTH: f64 = reference_pixels(28.0);
pub const SWITCH_THUMB_HEIGHT: f64 = reference_pixels(38.0);
pub const SWITCH_TRAVEL: f64 = reference_pixels(69.0);
/// Segoe is used when installed by the host. Inter is bundled by FerroUI.
pub const FONT_FAMILY: &str = "Segoe WP, Segoe UI, fonts:Phone#Selawik, fonts:Inter#Inter";

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
    pub motion_enabled: bool,
    pub high_contrast: bool,
}

impl Palette {
    pub fn new(accent: &str, light: bool) -> Self {
        let high_contrast = HIGH_CONTRAST.with(Cell::get);
        // High contrast is one complete dark palette. Mixing a white canvas
        // with the former white secondary ink made settings labels disappear.
        let light = light && !high_contrast;
        let accent = if ferroui_base::media::Color::parse(accent).is_ok() {
            accent.to_owned()
        } else {
            DEFAULT_ACCENT.to_owned()
        };
        Self {
            accent,
            background: if light { "#FFFFFF" } else { "#000000" },
            foreground: if light { "#000000" } else { "#FFFFFF" },
            secondary: if high_contrast {
                "#FFFFFF"
            } else if light {
                "#666666"
            } else {
                "#A6A6A6"
            },
            surface: if high_contrast {
                "#000000"
            } else if light {
                "#E6E6E6"
            } else {
                "#1F1F1F"
            },
            subtle: if light { "#EEEEEE" } else { "#333333" },
            light,
            text_scale: text_scale(),
            motion_enabled: crate::motion_enabled(),
            high_contrast,
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
    static HIGH_CONTRAST: Cell<bool> = const {Cell::new(false)};
    static VIEWPORT_HEIGHT: Cell<f64> = const {Cell::new(720.0)};
}

/// The shell supplies its phone-surface height; standalone controls retain a
/// conservative default. Long command menus preserve native keyboard/touch scrolling.
pub fn set_phone_viewport_height(height: f64) {
    if height.is_finite() {
        VIEWPORT_HEIGHT.with(|value| value.set(height.max(280.0)));
    }
}
pub(crate) fn app_menu_height() -> f64 {
    VIEWPORT_HEIGHT.with(|height| {
        (height.get() - STATUS_HEIGHT - NAVIGATION_HEIGHT - APP_BAR_HEIGHT - 100.0)
            .clamp(80.0, 480.0)
    })
}

pub fn set_high_contrast(enabled: bool) {
    HIGH_CONTRAST.with(|value| value.set(enabled));
}

pub fn secondary_color() -> &'static str {
    PALETTE.with(|palette| palette.borrow().secondary)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn high_contrast_overrides_light_background_as_a_complete_palette() {
        let previous = HIGH_CONTRAST.with(|value| value.replace(true));
        let p = Palette::new(DEFAULT_ACCENT, true);
        HIGH_CONTRAST.with(|value| value.set(previous));
        assert!(!p.light);
        assert_eq!(p.background, "#000000");
        assert_eq!(p.foreground, "#FFFFFF");
        assert_eq!(p.secondary, "#FFFFFF");
        assert_eq!(p.surface, "#000000");
    }
}
