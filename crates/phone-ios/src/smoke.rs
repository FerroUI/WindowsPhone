//! Explicit simulator-only observation hook; production launches create no timer.
use ferroui_base::threading::{DispatcherPriority, DispatcherTimer};
use ferroui_controls::TopLevel;
use phone_native::NativeHost;
use std::{rc::Rc, time::Duration};

pub fn install(host: &Rc<NativeHost>) {
    if std::env::var("PHONE_IOS_SMOKE").as_deref() != Ok("1") {
        return;
    }
    let weak = Rc::downgrade(host);
    DispatcherTimer::run(
        move || {
            let Some(host) = weak.upgrade() else {
                return false;
            };
            let view = host.view();
            let bounds = view.bounds();
            let attached = TopLevel::get_top_level(Some(&view)).is_some();
            let proof = serde_json::json!({
                "ready": attached && bounds.width > 0.0 && bounds.height > 0.0,
                "width": bounds.width, "height": bounds.height,
                "platform": "ios", "route": host.vm().route(),
            });
            // HOME is supplied by iOS for its own sandbox; it is never reassigned.
            if let Some(directory) = std::env::var_os("HOME").map(std::path::PathBuf::from) {
                let directory = directory.join("Documents");
                if std::fs::create_dir_all(&directory).is_ok() {
                    let temporary = directory.join("phone-ios-smoke.tmp");
                    if std::fs::write(&temporary, proof.to_string()).is_ok() {
                        let _ = std::fs::rename(temporary, directory.join("phone-ios-smoke.json"));
                    }
                }
            }
            true
        },
        Duration::from_millis(500),
        DispatcherPriority::BACKGROUND,
    )
    .detach();
}
