//! Reusable Windows Phone 8.1 controls over FerroUI's WinUI control library.
//!
//! The control templates retain native focus, keyboard, gesture and automation
//! behavior. The shell and individual apps share the same Metro design tokens.
mod buttons;
mod icons;
mod inputs;
mod motion;
mod paths;
mod pickers;
mod sliders;
mod templates;
mod theme;
mod tilt;
mod tokens;
mod typography;

pub use buttons::*;
pub use icons::{Icon, icon};
pub use motion::{live_tile_enter, motion_enabled, page_enter, reveal, set_motion_enabled, settle};
pub use theme::{install_theme, set_palette};
pub use tilt::attach_press_tilt;
pub use tokens::*;
pub use typography::*;

ferroui_markup_xaml::include_compiled_xaml!();
