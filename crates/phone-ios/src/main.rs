#[cfg(target_os = "ios")]
fn main() {
    ferroui_ios::run_application_with(std::rc::Rc::new(phone_ios::AppDelegate::default()));
}

#[cfg(not(target_os = "ios"))]
fn main() {
    eprintln!("phone-ios requires an iOS device or simulator. Use scripts/build-ios.sh on macOS.");
    std::process::exit(2);
}
