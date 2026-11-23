//! UIKit hosting and replaceable iOS integration for the portable phone runtime.
#[cfg(target_os = "ios")]
mod application;
#[cfg(any(target_os = "ios", test))]
mod device;
mod native;
#[cfg_attr(not(target_os = "ios"), allow(dead_code))]
mod pending;
mod platform;
#[cfg(any(all(target_os = "ios", feature = "ios26-telephony"), test))]
mod signature;
#[cfg(target_os = "ios")]
mod smoke;
#[cfg(target_os = "ios")]
mod smoke_devices;

#[cfg(target_os = "ios")]
pub use application::AppDelegate;
pub use native::data_path;
pub use platform::IosPlatform;
