//! Reusable Windows Phone 8.1 controls over FerroUI's WinUI control library.
//!
//! The control templates retain native focus, keyboard, gesture and automation
//! behavior. The shell and individual apps share the same Metro design tokens.
mod buttons;
mod icons;
mod inputs;
mod paths;
mod templates;
mod theme;
mod tokens;
mod typography;

pub use buttons::*;
pub use icons::{icon, Icon};
pub use theme::{install_theme, set_palette};
pub use tokens::*;
pub use typography::*;

ferroui_markup_xaml::include_compiled_xaml!();
