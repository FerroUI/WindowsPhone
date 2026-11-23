# iOS host

`phone-ios` mounts the shared `phone-native::NativeHost` inside FerroUI's UIKit
single-view lifetime. The phone UI, built-in apps, persistence, typed service
registry, activation parser, native media decoder, file import/export and
clipboard/URL services remain shared. This crate supplies the iOS entry point,
sandbox location and replaceable `IosPlatform` OS adapter.

## Implemented integrations

| Integration | Behavior |
| --- | --- |
| UIKit lifetime | Mounted FerroUI view, safe-area padding, phone/Inter fonts, foreground/background events and bounded incoming URI delivery. |
| Storage | Private `Application Support/FerroUIWindowsPhone/phone.json`; document pickers and scoped resource streams come from FerroUI. |
| Activation | `ferroui-phone://home` and `ferroui-phone://app/org.ferroui.wallet`; the shared activation parser validates app identity and route paths. |
| External actions | FerroUI's UIKit launcher handles web, telephone, email and SMS URL requests through OS completion callbacks. |
| Sharing | Native `UIActivityViewController` for text, URLs and validated sandbox media files, correct iPad popover anchor, actual completion/cancellation, no delivery claim for the selected recipient. |
| Settings | Public application settings, notification settings, and the public Default Apps page on supported OS/SDK versions. Direct network settings is explicitly unsupported. |
| Notifications | Actual authorization state, explicit permission request, silent local notification acceptance and foreground banner delegate. Existing embedding-app delegates are preserved. |
| Location | Explicit foreground When-in-Use authorization and one-shot CoreLocation request; invalid/stale locations fail; suspension cancels the manager. |
| Power and haptics | Observed battery/charging/Low Power Mode, unknown values preserved, and selection/light/medium/heavy UIKit feedback requests. |
| Now Playing | Shared Apple MediaPlayer metadata and bounded generation-stamped remote commands, foreground playback audio-session ownership, interruption/headphone-disconnect pause, and observer teardown. |
| Media/camera | Shared FerroUI Apple video provider and texture import; the bundle declares camera usage. |

The package is an ordinary UIKit application. `HostMode::App` and
`HostMode::Embedded` are supported; replacement Home Screen, installed-app
enumeration, bundle-ID application launching and a privileged system-shell role
are explicitly unavailable. iOS supports particular default-app categories, but
this package does not request their entitlements or claim a Home Screen role.

There is no `UIBackgroundModes` declaration. Media and camera follow foreground
host lifecycle policy; this package does not claim background playback,
background location, remote push, background scheduled alarms, or system-wide
lock-screen replacement. Notification requests are silent OS banners; the
shared phone foreground alarm service owns alarm sound.

## Ownership and cancellation

All platform dispatch starts on the UIKit thread. A process-wide bounded table
admits at most 16 operations. Native blocks hold operation IDs and native
objects; they never retain pointers into Rust UI state or temporary strings.
Rust copies completion text before returning to Objective-C. Ownership is
removed before waking a continuation, so late or duplicate callbacks cannot
complete a replacement request.

Native file handoff is a separate `PlatformPlugin::open_file` hook. Public
platform requests and URI activations cannot supply paths. The host resolves an
immutable asset first; the iOS bridge additionally requires a regular bounded
file in this host's `Application Support/FerroUIWindowsPhone/assets` directory,
rejects symlinks, non-owned files and names outside the media asset grammar,
and retains presentation ownership until completion or
cancellation. It uses the public UIKit activity controller, so attachment
handoff does not depend on an unretained preview-controller delegate.

Dropping a future cancels its native operation. Closing the host cancels every
operation; suspension cancels foreground location. Native timeouts bound
settings (15 seconds), sharing (120), notification authorization (60), banners
(15) and location (30). UIKit authorization dialogs remain controlled by iOS;
cancellation only discards ownership of their eventual result. Returning from
Settings refreshes actual notification authorization without prompting.

Native media callbacks enqueue commands for the shared UI-owned decoder. An
interruption or disconnected output clears pending commands and enqueues Pause
for the current source generation. Playback never resumes automatically when
an interruption ends. Audio activation failures propagate through the existing
media-session status contract.

## Build and run

Use macOS with Xcode and an installed iOS simulator runtime. Install the pinned
Rust toolchain from `FerroUI/scripts/browser/setup.sh`, then its iOS target:

```sh
rustup target add aarch64-apple-ios-sim
bash scripts/build-ios.sh --target aarch64-apple-ios-sim
bash scripts/smoke-ios.sh target/ios-bundles/phone-ios.app
```

The build script delegates compilation and base bundling to
`FerroUI/scripts/ios/bundle.sh`, adds the application URI scheme, privacy text,
existing phone icons, and signs the simulator bundle ad hoc. `--profile release`
and `--no-build` are supported. Intel simulators use `x86_64-apple-ios`.

For a device, use `--target aarch64-apple-ios`. The resulting bundle is unsigned;
installation requires the developer's own signing identity and provisioning
profile. The script does not change accounts, certificates or entitlements.

`.github/workflows/ios.yml` uses the pinned Rust/FerroUI revisions and runs on
manual dispatch or a main commit containing `[verify-mobile]` or `[verify-ios]`. It compiles the
actual simulator executable and Objective-C bridges, installs on a newly
created simulator, verifies that UIKit mounted a nonzero phone view, opens
Wallet through `simctl openurl`, returns Home, and uploads JSON/screenshot
evidence. The simulator is removed afterward. A production launch has no smoke
timer; the harness explicitly supplies `PHONE_IOS_SMOKE=1`.

Portable tests (`cargo test -p phone-ios --lib`) exercise completion ownership,
single delivery, stale callbacks, suspension and close cancellation. Linux
tests cannot validate Apple SDK compilation, real permissions, tactile output,
camera hardware or system media controls. The simulator compile and UI smoke
are separate gates; physical-device behavior still requires device testing.

## Platform references

- [Apple: Default apps updates](https://developer.apple.com/documentation/updates/defaultapps)
- [Apple: UIActivityViewController](https://developer.apple.com/documentation/uikit/uiactivityviewcontroller)
- [Apple: Requesting location authorization](https://developer.apple.com/documentation/corelocation/requesting-authorization-to-use-location-services)
- [Apple: UNUserNotificationCenter](https://developer.apple.com/documentation/usernotifications/unusernotificationcenter)
- [Apple: AVAudioSession interruptions](https://developer.apple.com/documentation/avfaudio/avaudiosession/interruptionnotification)
- [Apple: Audio route changes](https://developer.apple.com/documentation/avfaudio/responding-to-audio-route-changes)
