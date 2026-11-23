#![cfg_attr(target_os = "emscripten", no_main)]
mod app;
mod documents;
mod exports;
mod host;
mod inspection;
mod pim;
mod platform;
mod sdk_tasks;
use ferroui_base::media::FontManagerOptions;
use ferroui_browser::{BrowserAccessibilityMode, BrowserAppBuilder, BrowserPlatformOptions};
use ferroui_controls::AppBuilder;
use ferroui_fonts_inter::AppBuilderExtension;
use phone_fonts::PhoneFontBuilderExt;
use std::rc::Rc;
use wasm_bindgen::prelude::*;

ferroui_base::ferro_thread_local! {
    static STARTED: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

#[wasm_bindgen(js_name = runMain)]
pub fn run_main() -> Result<(), JsValue> {
    if STARTED.with(|started| started.get()) {
        return Err(JsValue::from_str(
            "This browser module has already started. Reload the document to create another host.",
        ));
    }
    app::prepare_platform(Rc::new(
        platform::BrowserPlatform::new().map_err(|error| JsValue::from_str(&error))?,
    ));
    STARTED.with(|started| started.set(true));
    AppBuilder::configure::<app::App>()
        .with_inter_font()
        .with_phone_font()
        // Browser Skia has no system default family. Register the concrete
        // default before Application::initialize can construct FontManager;
        // with_inter_font installs the embedded collection after setup.
        .with(Rc::new(FontManagerOptions {
            default_family_name: Some(phone_fonts::FAMILY.into()),
            ..Default::default()
        }))
        .start_browser_app(
            "out",
            Some(BrowserPlatformOptions {
                accessibility: BrowserAccessibilityMode::Enabled,
                ..Default::default()
            }),
        );
    app::startup_result().map_err(|error| JsValue::from_str(&error))
}
#[cfg(not(target_os = "emscripten"))]
fn main() {
    println!("Build Windows Phone with scripts/build-browser.sh");
}
