//! Application SDK for the retained FerroUI Windows Phone environment.
//!
//! Applications register metadata, render declarative views and own their typed
//! session state. The SDK has no dependency on the shell or any application.
pub mod common;
mod context;
pub mod device;
pub mod images;
mod manifest;
pub mod media;
mod registration;
mod registry;
pub mod session;
pub mod tasks;
pub use tasks::{FilePickerOptions, FileTask, PickedFile, complete_file, is_file_task_pending};
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
    pub use crate::{
        AppCategory, AppContext, AppManifest, AppRegistration, AppStorage, Capability,
    };
    pub use crate::{common::*, session::Session};
    pub use ferroui_view::prelude::*;
    pub use phone_controls::{
        Icon, PanoramaPage, PivotPage, action, activity_progress, app_bar, flat, input,
        minimized_app_bar, panorama_pages, pivot, pivot_pages, progress,
    };
    pub use phone_domain::{AppId, Route};
    pub use phone_state::PhoneViewModel;
    pub use std::rc::Rc;
}
