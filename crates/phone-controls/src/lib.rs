//! Reusable Windows Phone 8.1 controls over FerroUI's WinUI control library.
//!
//! The control templates retain native focus, keyboard, gesture and automation
//! behavior. The shell and individual apps share the same Metro design tokens.
mod application_bar;
pub mod barcode;
mod button_styles;
mod buttons;
mod checked_state;
mod choices;
mod grouped_list;
mod icons;
mod inputs;
mod motion;
mod paths;
mod pickers;
mod pivot_host;
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
pub use grouped_list::{attach_grouped_list, GroupedListState, ListGroup};
pub use icons::{icon, Icon};
pub use motion::{live_tile_enter, motion_enabled, page_enter, reveal, set_motion_enabled, settle};
pub use pickers::{
    attach_picker_focus, close_phone_pickers, dismiss_phone_picker, has_open_phone_picker,
};
pub use pivot_host::{pivot_pages, PivotPage};
pub use theme::{install_theme, set_palette};
pub use tilt::attach_press_tilt;
pub use tokens::*;
pub use typography::*;

ferroui_markup_xaml::include_compiled_xaml!();
