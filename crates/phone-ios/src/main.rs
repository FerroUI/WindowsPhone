#[cfg(target_os = "ios")]
fn main() {
    // Keep the earliest opt-in checkpoint independent of UIKit/Foundation.
    if std::env::var("PHONE_IOS_SMOKE").as_deref() == Ok("1") {
        eprintln!("[ios-smoke-entry] pid={} process.main", std::process::id());
    }
    ferroui_ios::run_application_with(std::rc::Rc::new(phone_ios::AppDelegate::default()));
}

#[cfg(not(target_os = "ios"))]
fn main() {
    eprintln!("phone-ios requires an iOS device or simulator. Use scripts/build-ios.sh on macOS.");
    std::process::exit(2);
}
