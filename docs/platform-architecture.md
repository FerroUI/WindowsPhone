# Portable host and shell architecture

The phone is a FerroUI control tree with shared Rust applications, state and SDK. Its operating-system boundary is composed explicitly. Android, iOS, browser, Windows, macOS and Linux use the same application registrations and navigation model; each host provides its actual capabilities and owns its platform lifetime.

## Package boundaries

| Package | Responsibility |
| --- | --- |
| `phone-host` | Portable requests, environment and platform contracts; explicit service registry; URI/catalog validation; native I/O/network primitives and OS media-session contracts. No UI or app dependencies. |
| `phone-runtime` | UI-thread request broker, capability checks, bounded pending tasks, cancellation, environment merging, lifecycle and validated app activation. Depends on state and the system, not application implementations. |
| `phone-native` | Reusable native phone composition, sandbox persistence and recovery, FerroUI file/clipboard/URI adapters, shared media/camera and import/export workers. |
| `phone-platform-desktop` | Windows/macOS/Linux installed-app catalogs, native activation, system settings and observed session role. |
| `phone-desktop` | Desktop lifetime, window, renderer selection and command-line composition. |
| `phone-android` | FerroUI Android activity host, JNI bridge, sandbox path, Android role/services/media adapter and APK packaging. |
| `phone-ios` | FerroUI UIKit host, application delegate, ARC services bridge, Apple media-session integration and bundle packaging. |
| `phone-browser` | FerroUI WebAssembly lifetime, browser platform bridge and existing web services/PWA implementation. |
| `phone-sdk`, `phone-system`, `phone-ui`, app crates | Trusted app registration/services, shell, default composition and individual applications. |

```mermaid
flowchart TD
    Apps["SDK applications and system shell"] --> Runtime["PhoneRuntime: requests and lifecycle"]
    Runtime --> Services["Explicit service providers"]
    Runtime --> Plugin["PlatformPlugin: capabilities and OS operations"]
    Services --> Native["phone-native or browser services"]
    Native --> Ferro["FerroUI storage, media, input and rendering"]
    Plugin --> OS["Supported operating-system APIs"]
```

FerroUI remains responsible for controls, layout, styling, input, dispatch, platform windows/views, graphics, texture interop, native pickers, clipboard, URI activation and media/camera providers. The new code adds phone-specific composition and OS operations that are not exposed by the pinned framework. It does not duplicate a renderer or implement app screens in Java, Objective-C or HTML.

## Hosting modes and actual authority

`HostMode` expresses the requested presentation. `HostRole` records observed authority: `Unsupported`, `Available`, `Requested` or `Active`. These are separate values. Creating a fullscreen window does not grant a home or shell role.

| Platform | App / embedded surface | Launcher | Shell session |
| --- | --- | --- | --- |
| Android | Native FerroUI activity; application-owned files and normal OS permission flow | Separate HOME APK flavor; explicit `RoleManager.ROLE_HOME` request, then observed role ownership | No replacement of Android System UI, system services or kernel |
| iOS | Native UIKit/Metal application and reusable embedded composition | Not advertised; no installed-app catalog or Home Screen replacement in this adapter | Not advertised |
| Browser | FerroUI WebAssembly app/PWA; embedded mode is explicit | No arbitrary installed-app enumeration or OS HOME authority | Not advertised |
| Windows | Native desktop application or embedded control | Fullscreen native-app launcher using the AppsFolder catalog | Opt-in Shell Launcher configuration on supported editions; adapter verifies a configured shell-host process chain |
| Linux | Native X11 or Wayland application or embedded control | Fullscreen launcher using GIO's installed-app catalog | Opt-in labwc login session; labwc provides window management/compositing, phone provides home/launcher |
| macOS | Native desktop application or embedded control | Fullscreen application launcher using Launch Services / NSWorkspace | Replacement system shell is not advertised |

`PlatformState::validate` rejects impossible combinations, duplicate capabilities and unconfigured shell mode. Android/iOS/browser builds do not inherit desktop shell authority. Desktop HOME-role requests are not substituted for an Android role chooser.

The device catalog is transient and bounded to 4,096 unique entries. Public IDs are opaque identities from the observed OS catalog, never executable command lines. The desktop provider retains native targets privately and revalidates them when launching. Android considers enabled/exported launcher activities visible to the current profile, with manifest-scoped queries rather than broad package visibility.

**Device apps** (`org.ferroui.device`) is an independent SDK app. It presents a virtualized native-app list, search, open, refresh and hold-to-pin. Pins preserve a bounded encoded identity and refresh/revalidate the catalog before launching. The device page reports platform/mode/roles and exposes supported settings, HOME selection, haptics and notification-consent actions. Browser/iOS show the supported integrations without claiming a native app catalog.

## Replaceable services and application composition

`ServiceRegistry` gives one provider ownership of each request family. Duplicate registration fails; replacement is explicit. A denied operation is returned as an error instead of falling through to another provider. Families include networking, files, navigation, clipboard, media, camera, notifications, location, sharing, environment and updates. Platform requests are owned by the separate `PlatformPlugin` contract.

`NativeHost::new` uses the default apps/services. `NativeHost::with_services` customizes service families. `NativeHost::compose` also adds independently compiled SDK applications. A host that replaces the entire default app set can compose `phone_system::create_view` with its own `AppRegistry` and attach `PhoneRuntime` directly.

The following embedding function is complete at the composition boundary. Its owner configures the FerroUI application/fonts/theme first, supplies a real platform plugin and clipboard implementation, and retains the returned host for the lifetime of the mounted control:

```rust
use ferroui_base::Ref;
use ferroui_controls::{ContentControl, Control};
use phone_host::{platform::{HostService, PlatformPlugin}, HostRequest};
use phone_native::{AppRegistration, NativeHost};
use std::{path::PathBuf, rc::Rc};

pub fn mount_phone(
    parent: &Ref<ContentControl>,
    data_file: PathBuf,
    platform: Rc<dyn PlatformPlugin>,
    apps: Vec<AppRegistration>,
    copy_text: Rc<dyn Fn(String) -> Result<(), String>>,
) -> Result<Rc<NativeHost>, String> {
    let host = NativeHost::compose(data_file, platform, apps, move |services| {
        services.replace(
            "company.clipboard",
            HostService::Clipboard,
            Rc::new(move |request| match request {
                HostRequest::CopyText { text } => copy_text(text),
                _ => Err("Unsupported clipboard request.".into()),
            }),
        )
    })?;
    parent.set_content(Some(Control::boxed(&host.view())));
    Ok(host)
}
```

Forward `HostLifecycle::{Resumed,Suspended,Closing}` from the owner. Call `Closing` before removing the view and releasing the host. The pinned shell owns one mounted phone per UI thread; `NativeHost` enforces that constraint instead of allowing global session/media hooks to collide. Application extensions and provider implementations are trusted linked Rust code. This registry is not a process sandbox, a stable binary ABI, or a loader for XAP/APPX packages.

## Request and lifetime rules

Platform request dispatch begins synchronously inside the input handler so browser share/permission operations retain user activation. The broker validates the request and current platform state, admits at most eight concurrent operations and gives each a monotonically increasing identity. Refresh and catalog requests use last-response-wins channels. Completion is admitted only into the original live runtime and phone-document epoch.

Closing first flushes app state and cancels SDK tasks, then closes the platform gate and provider. Waiting futures wake and release their provider tasks. Native and JavaScript bridges use bounded request registries; callbacks do not retain Rust UI pointers. A late native response cannot complete a reused request identity. Suspension stops the camera and foreground clock/alarm processing. Foreground alarm suspension preserves the alert/snooze state, stops audio/tactile output and invalidates its presentation latch; a visible tick revalidates the alert before resuming it. Native haptic-capable hosts use bounded foreground alarm pulses that respect the phone vibration setting. Android's explicitly started media playback service can retain ongoing user-started audio; iOS pauses foreground playback because this bundle does not declare background audio. Neither platform resumes media merely because the application becomes visible again.

Native picker/import/export operations carry a runtime/document lease and a captured storage root. Worker jobs receive the correct `AssetStore` explicitly, rather than reading UI-thread-local storage configuration on another thread. Operations recheck scope after asynchronous work and before admitting files or replacing the document. Stale staged imports are cleaned up. A lease does not claim that an OS file picker has a cancellation API; abandoned results are discarded.

Media source generations, document epochs and finite bounded positions also protect OS media commands. The decoder remains FerroUI's platform provider. Windows SMTC, Linux MPRIS, macOS/iOS MediaPlayer and Android MediaSession publish metadata/position and enqueue transport commands. Android audio focus remains with the framework decoder. Background alarms, process resurrection and closed-app reminder scheduling are not implemented by the media service.

## Native mobile services

Android uses public intents and permission APIs for application/network/notification/default-app settings, text sharing, haptics, notification consent and foreground banners, one-shot foreground location, battery/network observations and user-selected HOME. Its ordinary App APK has no HOME category. The Launcher APK has the required HOME filter and a separate application identity. Both use the framework activity/rendering/input lifetime and singleTask activation handling.

Android external media handoff uses a private FileProvider restricted to the app's assets subtree. Canonical file identity, regular-file ownership, bounded content and temporary read-only URI leases are checked independently of the Rust asset validation. The service is private and declares only media playback foreground-service authority; no boot autoplay or background-start bypass is added.

iOS uses public UIKit settings URLs, UIActivityViewController, foreground CoreLocation, UserNotifications, haptic feedback and battery/power observations. The app supplies camera/location usage descriptions. File share controllers and their completion objects remain retained until dismissal and have an iPad presentation anchor. Sandbox-owned regular assets are validated again at the native boundary. AVAudioSession playback activation and interruption/headphone-disconnection handling accompany MediaPlayer transport controls.

Platform operations report observed availability, denial, cancellation or acceptance. Acceptance by an OS API does not prove recipient delivery, permission approval, target-app startup or successful physical playback.

## Startup and deployment

```sh
# Existing desktop session
cargo run --locked -p phone-desktop -- --mode app
cargo run --locked -p phone-desktop -- --mode launcher
# Linux native backend selection
cargo run --locked -p phone-desktop -- --mode launcher --backend wayland
cargo run --locked -p phone-desktop -- --mode app --backend x11
# Validated SDK activation (also supported by mobile platform entry points)
cargo run --locked -p phone-desktop -- --activate 'ferroui-phone://app/org.ferroui.device'
# Android application and HOME flavors
bash scripts/build-android.sh
bash scripts/build-android.sh --launcher --no-cargo
# macOS/Xcode: iOS Simulator bundle
bash scripts/build-ios.sh --target aarch64-apple-ios-sim
```

Activation accepts only `ferroui-phone://home` or `ferroui-phone://app/APP_ID?path=ENCODED_PATH`. Credentials, fragments, duplicate/unknown query parameters, control characters and oversized identities/paths are rejected. A startup link waits for the root SDK session to mount; disabled or unregistered targets are refused.

See [Android packaging](../crates/phone-android/README.md), [iOS packaging](../crates/phone-ios/README.md), [Linux session staging](../crates/phone-desktop/platform/linux/README.md), [Windows shell deployment](../crates/phone-desktop/platform/windows/README.md) and [desktop prerequisites](../crates/phone-desktop/platform/README.md). Session generators produce reviewable install/configuration artifacts. Creating them does not change the current machine's default launcher, login session, policies or shell. Mobile debug artifacts are development packages; device/distribution signing is separate.

The `[verify-mobile]` commit marker enables the Android APK and iOS Simulator gates; `[verify-native]` adds Windows/macOS/Linux build/link and protocol checks; `[verify-ui]` enables rendered browser interaction tests. Verification records distinguish source implementation, compile/package checks, simulator interaction and physical-device qualification. No build result by itself establishes permissions, hardware, compositor/device policy or complete Windows Phone fidelity.

## Primary platform references

- [Android roles and HOME consent](https://developer.android.com/reference/android/app/role/RoleManager)
- [Android foreground-service types](https://developer.android.com/develop/background-work/services/fgs/service-types)
- [Android FileProvider](https://developer.android.com/reference/androidx/core/content/FileProvider)
- [Apple default-app capabilities](https://developer.apple.com/documentation/updates/defaultapps)
- [Apple activity controllers](https://developer.apple.com/documentation/uikit/uiactivityviewcontroller)
- [Apple audio interruptions](https://developer.apple.com/documentation/avfaudio/avaudiosession/interruptionnotification)
- [Windows Shell Launcher requirements](https://learn.microsoft.com/windows/configuration/shell-launcher/)
- [GIO desktop applications](https://docs.gtk.org/gio/class.DesktopAppInfo.html)
- [Browser related-app visibility](https://developer.chrome.com/docs/capabilities/get-installed-related-apps)
