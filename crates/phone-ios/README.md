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
| Device contacts | Explicit Contacts permission, full/limited selection observation, bounded native queries, create/update/delete through `CNContactStore`. Updates preserve unrelated fields and existing labels; an unchanged display name preserves structured name components. System Favorites are unavailable through this API. |
| Messages | The public MessageUI composer reports its actual Sent/Cancelled/Failed result. Sent means composer handoff, not carrier delivery. The native file picker returns expiring opaque attachment leases; multimedia composition copies the selected bytes into the system composer. |
| Maps | Cancellable `MKLocalSearch` and `MKMapItem` handoff to native Maps, with supported walking/driving/cycling/transit directions. This provider does not claim the default navigation role. |
| Hardware controls | UIKit brightness read/write, authorized rear-camera torch ownership, actual audio route and volume observation, native volume/output controls, and read-only Low Power Mode. Unsupported private Settings URLs and hidden volume-slider writes are not used. |
| Power and haptics | Observed battery/charging/Low Power Mode, unknown values preserved, and selection/light/medium/heavy UIKit feedback requests. |
| Now Playing | Shared Apple MediaPlayer metadata and bounded generation-stamped remote commands, foreground playback audio-session ownership, interruption/headphone-disconnect pause, and observer teardown. |
| Media/camera | Shared FerroUI Apple video provider and texture import; the bundle declares camera usage. |

The package is an ordinary UIKit application. `HostMode::App` and
`HostMode::Embedded` are supported; replacement Home Screen, installed-app
enumeration, bundle-ID application launching and a privileged system-shell role
are explicitly unavailable. iOS supports particular default-app categories, but
the default package does not request their managed entitlements or claim a Home
Screen role. The optional iOS 26 provider described below implements the
separate dialer/carrier categories without provisioning them automatically.

There is no `UIBackgroundModes` declaration. Media and camera follow foreground
host lifecycle policy; this package does not claim background playback,
background location, remote push, background scheduled alarms, or system-wide
lock-screen replacement. General host notification requests are silent OS
banners; the shared phone foreground alarm service owns alarm sound. Optional
carrier notifications follow the separate persisted message policy below.

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
operation; suspension cancels foreground location and device read operations,
stops hardware observation and releases an app-owned torch. Native timeouts bound
settings (15 seconds), sharing (120), notification authorization (60), banners
(15) and location (30). UIKit authorization dialogs remain controlled by iOS;
cancellation only discards ownership of their eventual result. Returning from
Settings refreshes actual notification authorization without prompting.

Device snapshots are coalesced and delivered before dependent data events.
Contacts queries recheck authorization and contact-store generation before
admitting their results. Native pickers carry the originating draft ID; a late
selection cannot attach itself to a different draft. Attachment import is limited
to eight regular files, 8 MiB each and 24 MiB per selection. At most sixteen leases
and 64 MiB are retained, each for fifteen minutes. Cancellation, expiry and host
closure remove the owned copies. Callers never receive or supply native paths.

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

## Optional iOS 26 dialer and carrier provider

```sh
bash scripts/build-ios.sh --target aarch64-apple-ios-sim --features ios26-telephony
```

This feature compiles Swift adapters for `LiveCommunicationKit` and
`TelephonyMessagingKit` using the installed Apple SDK. Use an SDK with the iOS
26.5 APIs; runtime availability guards retain the normal application fallback
on older supported iOS versions. The frameworks are weak-linked.

The feature flag grants no entitlement. The adapter reads only the running
executable's bounded Mach-O code-signature entitlement dictionary, rejects
missing/malformed/unsupported forms, and checks actual framework authorization
before use. It uses no private Security entitlement API. A simulator, iPad,
unentitled app, or app running on a Mac never advertises the carrier roles as
usable. Device signing and provisioning remain the developer's responsibility.

| Optional integration | Admission and behavior |
| --- | --- |
| Default Dialer | Requires an eligible iPhone, iOS 26 and signed `com.apple.developer.dialing-app`. Calls use `TelephonyConversationManager`, with actual cellular-service account IDs. Successful authorized conversation-history access establishes the observed active role. Opening Default Apps does not establish it. |
| Carrier messaging | Requires signed `com.apple.developer.carrier-messaging-app` and `isConfiguredForCarrierMessaging`. The framework enforces Apple's device, account, region and selected-default requirements. SMS and MMS sends require a currently viable selected subscription. |
| Carrier inbox | Consumes SMS/MMS/RCS incoming streams and commits native payload plus query metadata to a separate private SQLite store before UI publication. SMS/MMS text, MMS parts and RCS text/file messages are exposed. Richer RCS payloads remain preserved; outbound RCS chat features are not advertised. |
| Failure recovery | Undownloaded MMS has an explicit retry action. Draft/client IDs are durable send idempotency keys; failed or interrupted sends are never automatically resent. A process interrupted during sending records an unknown outcome. Sender acceptance is distinct from recipient delivery. |
| Native message notifications | OS-authorized background receipts create app-owned notifications. Tapping one opens the matching stored conversation; marking read removes only the matching notifications. Existing embedding-app notification delegates remain under their owner's control. |

The shared phone's notification preferences are persisted for autonomous carrier
receipts and refreshed synchronously on attach, setting changes and document
restore. Disabling global notifications or the message center withholds and
removes this provider's carrier notifications. Disabling banners uses passive,
silent list delivery. Disabling lock-screen notifications removes sender/body
previews from all subsequently posted carrier notifications and clears older
previews. Sound requires both the phone preference and actual OS authorization.
iOS controls background vibration and cannot offer independent banner-only
delivery without Notification Center retention; this adapter does not request
custom background haptics. None of these preference updates requests permission.

The durable carrier store is independent of the phone's UI document and its
backup/restore. Accepted outgoing payloads have their own copy, so releasing a
draft lease cannot change an in-flight message. Closing a view cannot erase an
already accepted transport result. Times exchanged with the shared UI use Unix
milliseconds. Telephony streams run only while iOS grants the process execution;
this package adds no general background mode or extension entitlement.

Apple's current Default Dialer and carrier-messaging guides describe regional
eligibility and managed entitlement requirements. The app does not register
`telephony:` as a handler: that scheme is the system calling fallback. General
carrier answer/end/hold/mute/DTMF control and an active-call catalog are not
advertised by these adapters. The separate VoIP default-calling role and default
navigation category require their own eligible implementation and entitlements.

For a device, use `--target aarch64-apple-ios`. The resulting bundle is unsigned;
installation requires the developer's own signing identity and provisioning
profile. The script does not change accounts, certificates or entitlements.

`.github/workflows/ios.yml` uses the pinned Rust/FerroUI revisions and runs on
manual dispatch or a main commit containing `[verify-mobile]` or `[verify-ios]`. It compiles the
standard and feature-enabled simulator executables and native bridges, and
type-checks the Swift provider against the physical iPhone SDK. It installs the
feature-enabled bundle on a newly created simulator, verifies visible Start,
Wallet, Phone, People, Messages and Settings controls and valid layout inside
UIKit's safe area, opens the app routes through `simctl openurl`, returns Home, and uploads
JSON/screenshot evidence. A small public XCTest runner accepts only SpringBoard's
exact `Open in “Windows Phone”?` confirmation; the shell independently verifies
fresh observations and the real delegate's accepted URI. Other system prompts
are untouched. XCTest results and the final state are retained on failure.
The simulator is removed afterward. A production launch has no smoke
timer; the harness explicitly supplies `PHONE_IOS_SMOKE=1`. A separate
`PHONE_IOS_DEVICE_TEST=1` opt-in additionally requires a simulator build. It
creates, reads, updates and deletes one uniquely named contact in that disposable
simulator; writes and restores brightness; and checks that unprovisioned carrier
roles and unavailable hardware remain unavailable. It places no calls and sends
no messages. Contacts permission is granted only on that newly created test
simulator, and all changes are removed with the simulator afterward.

Portable tests (`cargo test -p phone-ios --lib`) exercise completion ownership,
single delivery, stale callbacks, suspension and close cancellation. Linux
tests cannot validate Apple SDK compilation, real permissions, tactile output,
camera hardware or system media controls. The simulator compile and UI smoke
are separate gates; physical-device behavior still requires device testing.

## Platform references

- [Apple: Default apps updates](https://developer.apple.com/documentation/updates/defaultapps)
- [Apple: Preparing your app to be the default dialer](https://developer.apple.com/documentation/livecommunicationkit/preparing-your-app-to-be-the-default-dialer-app)
- [Apple: Creating a carrier messaging app](https://developer.apple.com/documentation/telephonymessagingkit/creating-a-carrier-messaging-app)
- [Apple: Preparing a default navigation app](https://developer.apple.com/documentation/mapkit/preparing-your-app-to-be-the-default-navigation-app)
- [Apple: Contacts](https://developer.apple.com/documentation/contacts)
- [Apple: MessageUI](https://developer.apple.com/documentation/messageui)
- [Apple: MKMapItem](https://developer.apple.com/documentation/mapkit/mkmapitem)
- [Apple: UIActivityViewController](https://developer.apple.com/documentation/uikit/uiactivityviewcontroller)
- [Apple: Requesting location authorization](https://developer.apple.com/documentation/corelocation/requesting-authorization-to-use-location-services)
- [Apple: UNUserNotificationCenter](https://developer.apple.com/documentation/usernotifications/unusernotificationcenter)
- [Apple: AVAudioSession interruptions](https://developer.apple.com/documentation/avfaudio/avaudiosession/interruptionnotification)
- [Apple: Audio route changes](https://developer.apple.com/documentation/avfaudio/responding-to-audio-route-changes)
