fn main() {
    for file in [
        "bridge.h",
        "bridge.m",
        "share.m",
        "notifications.m",
        "network.m",
        "notification_policy.m",
        "location.m",
        "device.h",
        "device.m",
        "contacts.m",
        "hardware.m",
        "communication.m",
        "attachments.h",
        "attachments.m",
    ] {
        println!("cargo:rerun-if-changed=native/{file}");
    }
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("ios") {
        let entitled_provider = std::env::var_os("CARGO_FEATURE_IOS26_TELEPHONY").is_some();
        cc::Build::new()
            .files([
                "native/bridge.m",
                "native/share.m",
                "native/notifications.m",
                "native/network.m",
                "native/notification_policy.m",
                "native/location.m",
                "native/device.m",
                "native/contacts.m",
                "native/hardware.m",
                "native/communication.m",
                "native/attachments.m",
            ])
            .define(
                "WP_IOS_TELEPHONY",
                if entitled_provider { "1" } else { "0" },
            )
            .flag("-fobjc-arc")
            .flag("-fblocks")
            .compile("phone_ios_integrations");
        for framework in [
            "Foundation",
            "UIKit",
            "UserNotifications",
            "Network",
            "CoreLocation",
            "Contacts",
            "AVFoundation",
            "AVKit",
            "MediaPlayer",
            "MessageUI",
            "MapKit",
            "UniformTypeIdentifiers",
        ] {
            println!("cargo:rustc-link-lib=framework={framework}");
        }
        if entitled_provider {
            build_telephony();
        }
    }
}

fn xcrun(arguments: &[&str]) -> String {
    let output = std::process::Command::new("xcrun")
        .args(arguments)
        .output()
        .expect("The entitled iOS provider requires Xcode command line tools");
    assert!(
        output.status.success(),
        "xcrun failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout)
        .expect("Xcode path is UTF-8")
        .trim()
        .into()
}

fn build_telephony() {
    use std::{path::PathBuf, process::Command};
    let sources = [
        "Bridge.swift",
        "TelephonyProvider.swift",
        "CarrierStore.swift",
        "CarrierMessages.swift",
        "CarrierAttachments.swift",
    ];
    for source in sources {
        println!("cargo:rerun-if-changed=native/telephony/{source}");
    }
    println!("cargo:rerun-if-env-changed=IPHONEOS_DEPLOYMENT_TARGET");
    let target = std::env::var("TARGET").expect("Cargo target");
    let (sdk, architecture, simulator) = match target.as_str() {
        "aarch64-apple-ios" => ("iphoneos", "arm64", false),
        "aarch64-apple-ios-sim" => ("iphonesimulator", "arm64", true),
        "x86_64-apple-ios" => ("iphonesimulator", "x86_64", true),
        _ => panic!("Unsupported iOS telephony target: {target}"),
    };
    let sdk_path = xcrun(&["--sdk", sdk, "--show-sdk-path"]);
    let compiler = PathBuf::from(xcrun(&["--sdk", sdk, "--find", "swiftc"]));
    let toolchain_usr = compiler
        .parent()
        .and_then(|path| path.parent())
        .expect("Xcode Swift toolchain");
    let minimum = std::env::var("IPHONEOS_DEPLOYMENT_TARGET").unwrap_or_else(|_| "15.0".into());
    let swift_target = format!(
        "{architecture}-apple-ios{minimum}{}",
        if simulator { "-simulator" } else { "" }
    );
    let output = PathBuf::from(std::env::var_os("OUT_DIR").expect("Cargo output directory"));
    let mut command = Command::new(&compiler);
    command
        .args([
            "-sdk",
            &sdk_path,
            "-target",
            &swift_target,
            "-swift-version",
            "5",
            "-O",
            "-parse-as-library",
            "-emit-library",
            "-static",
            "-module-name",
            "PhoneTelephony",
            "-o",
        ])
        .arg(output.join("libPhoneTelephony.a"));
    for source in sources {
        command.arg(format!("native/telephony/{source}"));
    }
    assert!(
        command
            .status()
            .expect("Run the iOS Swift compiler")
            .success(),
        "The iOS 26 provider failed to compile"
    );
    println!("cargo:rustc-link-search=native={}", output.display());
    println!(
        "cargo:rustc-link-search=native={}",
        toolchain_usr.join("lib/swift").join(sdk).display()
    );
    println!("cargo:rustc-link-search=native={sdk_path}/usr/lib/swift");
    println!("cargo:rustc-link-lib=static=PhoneTelephony");
    println!("cargo:rustc-link-lib=sqlite3");
    println!("cargo:rustc-link-arg=-Wl,-rpath,/usr/lib/swift");
    println!("cargo:rustc-link-arg=-Wl,-weak_framework,LiveCommunicationKit");
    println!("cargo:rustc-link-arg=-Wl,-weak_framework,TelephonyMessagingKit");
}
