//! Native primitives. Platform UI hosts own prompting and dispatcher scheduling.
pub mod background;
mod files;
pub mod http;
mod launch;
pub mod location;
pub mod media_session;
pub mod media_backend;
pub mod notifications;
mod paths;
mod power;
pub mod weather;

pub use files::atomic_write;
pub use launch::open_external;
pub use paths::data_directory;
pub use power::{PowerMonitor, PowerSnapshot};
