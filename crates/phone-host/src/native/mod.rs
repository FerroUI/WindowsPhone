//! Native primitives. Platform UI hosts own prompting and dispatcher scheduling.
pub mod background;
mod files;
mod launch;
mod paths;
mod power;

pub use files::atomic_write;
pub use launch::open_external;
pub use paths::data_directory;
pub use power::{PowerMonitor, PowerSnapshot};
