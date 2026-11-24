//! Reusable Windows Phone 8.1 controls over FerroUI's WinUI control library.
//!
//! The control templates retain native focus, keyboard, gesture and automation
//! behavior. The shell and individual apps share the same Metro design tokens.
pub mod barcode;
mod application_bar;
mod buttons;
mod checked_state;
mod choices;
mod icons;
mod inputs;
mod motion;
mod paths;
mod pivot_host;
mod pickers;
mod sliders;
mod switches;
mod templates;
mod theme;
mod tilt;
mod tokens;
mod typography;

pub use application_bar::*;
pub use buttons::*;
pub use checked_state::{checked_state, CheckedState};
pub use choices::{check, radio};
pub use icons::{icon, Icon};
pub use pivot_host::{pivot_pages, PivotPage};
pub use motion::{live_tile_enter, motion_enabled, page_enter, reveal, set_motion_enabled, settle};
pub use pickers::attach_picker_focus;
pub use theme::{install_theme, set_palette};
pub use tilt::attach_press_tilt;
pub use tokens::*;
pub use typography::*;

ferroui_markup_xaml::include_compiled_xaml!();
