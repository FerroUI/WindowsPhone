//! Reusable host contracts and platform primitives, independent of the UI and
//! application view models. Host observations never become persisted settings.
mod environment;
mod request;
pub mod urls;

#[cfg(not(any(target_arch = "wasm32", target_os = "emscripten")))]
pub mod native;

pub use environment::*;
pub use request::*;
