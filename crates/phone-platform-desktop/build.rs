fn main() {
    println!("cargo:rerun-if-changed=native/applications.m");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("macos") {
        cc::Build::new()
            .file("native/applications.m")
            .flag("-fobjc-arc")
            .flag("-fblocks")
            .compile("phone_desktop_applications");
        println!("cargo:rustc-link-lib=framework=Foundation");
        println!("cargo:rustc-link-lib=framework=AppKit");
    }
}
