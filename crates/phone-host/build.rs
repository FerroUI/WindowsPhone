fn main() {
    println!("cargo:rerun-if-changed=native/notifications.m");
    println!("cargo:rerun-if-changed=native/location.m");
    println!("cargo:rerun-if-changed=native/media_session.m");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("macos") {
        cc::Build::new()
            .file("native/notifications.m")
            .file("native/location.m")
            .file("native/media_session.m")
            .flag("-fobjc-arc")
            .flag("-fblocks")
            .compile("phone_notifications");
        println!("cargo:rustc-link-lib=framework=Foundation");
        println!("cargo:rustc-link-lib=framework=AppKit");
        println!("cargo:rustc-link-lib=framework=UserNotifications");
        println!("cargo:rustc-link-lib=framework=CoreLocation");
        println!("cargo:rustc-link-lib=framework=MediaPlayer");
    }
}
