//! Application SDK for the retained FerroUI Windows Phone environment.
//!
//! Applications register metadata, render declarative views and own their typed
//! session state. The SDK has no dependency on the shell or any application.
pub mod common;
mod context;
pub mod images;
mod manifest;
mod registration;
mod registry;
pub mod session;
pub mod tasks;
pub use tasks::{complete_file, is_file_task_pending, FilePickerOptions, FileTask, PickedFile};
mod storage;
mod surfaces;
mod tile;

pub use context::*;
pub use manifest::*;
pub use phone_controls as controls;
pub use phone_domain as domain;
pub use phone_host as host;
pub use phone_state as state;
pub use registration::*;
pub use registry::*;
pub use storage::*;
pub use surfaces::*;
pub use tile::*;

pub mod prelude {
    pub use crate::{common::*, session::Session};
    pub use crate::{
        AppCategory, AppContext, AppManifest, AppRegistration, AppStorage, Capability,
    };
    pub use ferroui_view::prelude::*;
    pub use phone_controls::{action, app_bar, flat, input, pivot, Icon};
    pub use phone_domain::{AppId, Route};
    pub use phone_state::PhoneViewModel;
    pub use std::rc::Rc;
}
