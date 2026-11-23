fn main() {
    for file in [
        "bridge.h",
        "bridge.m",
        "share.m",
        "notifications.m",
        "location.m",
    ] {
        println!("cargo:rerun-if-changed=native/{file}");
    }
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("ios") {
        cc::Build::new()
            .files([
                "native/bridge.m",
                "native/share.m",
                "native/notifications.m",
                "native/location.m",
            ])
            .flag("-fobjc-arc")
            .flag("-fblocks")
            .compile("phone_ios_integrations");
        for framework in ["Foundation", "UIKit", "UserNotifications", "CoreLocation"] {
            println!("cargo:rustc-link-lib=framework={framework}");
        }
    }
}
