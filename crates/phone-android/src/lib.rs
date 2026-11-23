//! Native Android application and optional user-selected HOME launcher.
//! Rendering, input, activity lifetime and native media stay in FerroUI.
#[cfg(all(
    any(target_os = "android", test),
    not(any(target_arch = "wasm32", target_os = "emscripten"))
))]
mod media_wire;
#[cfg(any(target_os = "android", test))]
mod protocol;

#[cfg(all(target_os = "android", feature = "standalone"))]
mod application;
#[cfg(target_os = "android")]
mod bridge;
#[cfg(target_os = "android")]
mod media;
#[cfg(target_os = "android")]
mod plugin;
#[cfg(target_os = "android")]
pub use plugin::AndroidPlatformPlugin;

#[cfg(all(target_os = "android", feature = "standalone"))]
ferroui_android::android_application!(application::build);
