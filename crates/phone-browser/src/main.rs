#![cfg_attr(target_os = "emscripten", no_main)]
mod app;
mod exports;
mod host;
mod inspection;
use ferroui_browser::{BrowserAccessibilityMode, BrowserAppBuilder, BrowserPlatformOptions};
use ferroui_controls::AppBuilder;
use ferroui_fonts_inter::AppBuilderExtension;
use wasm_bindgen::prelude::*;

#[wasm_bindgen(js_name = runMain)]
pub fn run_main() {
    AppBuilder::configure::<app::App>()
        .with_inter_font()
        .start_browser_app("out", Some(BrowserPlatformOptions {
            accessibility: BrowserAccessibilityMode::Enabled,
            ..Default::default()
        }));
}
#[cfg(not(target_os = "emscripten"))]
fn main() { println!("Build Windows Phone with scripts/build-browser.sh"); }
