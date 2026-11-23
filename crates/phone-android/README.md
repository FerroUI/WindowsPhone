# Android application and HOME launcher

`phone-android` is the Android entry for the same `phone-native` host, app catalog,
state, services and FerroUI controls used on desktop and iOS. It produces a real
native `cdylib` and two Android packages:

| Package | Application ID | Entry behavior |
| --- | --- | --- |
| App | `org.ferroui.windowsphone` | Opens as an ordinary Android application. |
| Launcher | `org.ferroui.windowsphone.launcher` | Adds `MAIN/HOME/DEFAULT` and can become HOME after the user selects it in Android's chooser. |

Both packages have an ordinary launcher icon. Installing a launcher package does
not select it as HOME. Its platform state reports `Launcher` only after Android
confirms that it holds the role. Android continues to own the kernel, navigation
and status bars, permission UI, lock screen, other processes and app lifecycle;
the public SDK does not grant this application a replacement Android system UI.

An embedding owner can pass `AndroidPlatformPlugin::embedded()` to
`NativeHost::compose` and mount its view in an existing FerroUI Android TopLevel.
That provider reports `Embedded` independently of whether the package holds HOME;
it cannot claim a system-shell role. The owner forwards resume, suspend and close
events. Closing cancels native work, releases media controls and revokes outstanding
file handoffs, allowing a subsequent host to start cleanly on the same UI thread.

Use `phone-android` with `default-features = false` in an owner application. The
default `standalone` feature exports `JNI_OnLoad` and installs this package's
application; disabling it lets the owner supply its own
`ferroui_android::android_application!` entry. Derive the owner's Android activity
from `org.ferroui.phone.PhoneActivity` so private permission and foreground-activity
forwarding remain connected, include this package's Java/provider/service resources,
and use `AndroidPlatformPlugin::data_path()` for the matching asset sandbox. The
owner still installs its FerroUI theme/fonts and chooses the view placement. Keep
the `org.ferroui.phone` Java namespace and `PHONE_LAUNCHER` BuildConfig flag when
adapting the Gradle package. Native notification activation resolves the actual
owner activity. Holding HOME does not change an embedded host's mode.

## Ownership and framework reuse

`application.rs` uses `ferroui_android::android_application!`, `AppBuilder.use_android`
and FerroUI's activity application lifetime. A retained `Rc<NativeHost>` owns the
shared app runtime. Its main-view factory reattaches the retained root after the
old FerroUI activity detaches its content. The manifest uses `singleTask`, and
common configuration changes are handled by the existing activity. Resuming and
suspending forward to `NativeHost::lifecycle`; HOME activations go through the
shared, validated `ferroui-phone://home` activation route. The manifest does not
advertise arbitrary external URI schemes.

FerroUI provides the renderer, input, insets, Back handling, activity lifecycle,
permission and Storage Access Framework pickers. `phone-native` supplies app-owned
assets, import/export, persistence and the shared native service implementation.
The state path is `Context.getFilesDir()/windows-phone/phone.json`; media lives in
the adjacent `assets` directory. Platform dialogs use the current mounted TopLevel.

The Android media providers are FerroUI's existing native GL importer/frame
producer, Media3 video/audio player and CameraX capture provider. Packaging applies
the framework's three media `packaging.gradle` files and includes its Java sources.
The Android plugin supplies OS integrations through the shared `PlatformPlugin`
contract; apps depend on typed host requests rather than Android classes.

## Implemented platform integrations

| Integration | Native behavior and limits |
| --- | --- |
| Device applications and shortcuts | Active HOME can discover and launch current-profile `LauncherApps` shortcuts. App details and uninstall use fresh catalog membership and Android confirmation. Queries visible `MAIN/LAUNCHER` activities in the current profile; validates exported/enabled/permission-accessible components, skips itself and revalidates membership before launch. No `QUERY_ALL_PACKAGES` request. |
| HOME | API29+ `RoleManager.ROLE_HOME` request, API26–28 home settings fallback, real HOME-to-Start routing, shared Back navigation and package-change catalog refresh. A user-added Android Quick Settings tile opens Start. Native app/Quick Settings preferences can hide this window’s system bars only while HOME is actually held; Android edge gestures remain available. |
| Settings and quick controls | 25 typed observed controls; public settings panels, user-consented settings writes, torch, stream volume, rotation, timeout and sync. Settings lists contain only resolvable public pages. |
| Sharing | Native text `ACTION_SEND` chooser. Completion means Android opened the chooser, not that another person received content. |
| Haptics | `View.performHapticFeedback`, respecting device/user settings without requesting a vibration privilege. |
| Notifications | Android channels, explicit Android 13+ notification permission and native foreground alert posting. Denial remains denial; opening system settings does not imply a grant. |
| Location | Explicit fine/coarse runtime consent, approximate-location support, one native fix, freshness/range checks, cancellation and a 15-second acquisition timeout. No background location permission. |
| Environment | Actual battery, charging, external-power, power-saver and validated-network observations. Missing observations stay unknown. |
| Media controls | Native `MediaSession`, metadata, elapsed position/duration, play/pause/stop, previous/next and bounded seek commands. |
| File handoff | Expiring read-only `content:` URI grants for individual owned media assets, with MIME and `ACTION_VIEW`. |
| Device notification center | Permission-gated `NotificationListenerService`, current-profile active notifications, fresh opaque action identities, remote-input replies and clearable-only single/batch dismissal. Android redactions and action authentication remain authoritative. |
| Phone | Qualified default-dialer role, `TelecomManager.placeCall`, real `InCallService` state, native incoming/ongoing call screen and notifications, answer/reject/end/hold/resume/mute/speaker/DTMF, actual account selection and bounded call history. Calls survive shell suspension or close. |
| Contacts | Bounded native provider search, stable lookup identities, native account-aware insert/edit UI and explicit permission-gated deletion. |
| SMS | Qualified default-SMS components, real protected incoming broadcasts and provider storage, multipart sending, durable sent/delivery callbacks, separate SIM selection, read/delete and native transient composition drafts. |
| MMS | Pinned Apache-licensed AOSP PDU codec, actual WAP push parsing, incoming download/retry and notify response, provider parts/addresses, outgoing group MMS, carrier size limits, actual response/delivery aggregation, attachment picking and leased read-only viewing. |
| Maps | Validated geo search and map/directions handoff with coordinates and travel mode, using public Android/Maps URL contracts. |


Package enumeration and shortcuts are scoped to the current profile. Dialer, SMS,
notification-listener and settings-write access are separate Android roles or
permissions; holding HOME grants none of them. Role/permission requests occur
only from explicit commands. A `DeviceSnapshot` lists currently usable operations,
and the shared runtime rejects stale results after access is revoked. No contact,
call, SMS or notification collection is read at application startup.

SMS/MMS use Android's native provider and the selected carrier, outside portable
phone backups. RCS, eSIM provisioning, privileged global toggles, carrier account
administration, Android keyguard replacement and cross-profile access are not
provided by this public-SDK host. Carrier-specific MMS interoperability and OEM
background/lock-screen presentation require real-device qualification.

### Carrier messages, attachment ownership and privacy

Protected receivers journal bounded incoming work before acknowledgement; an
Android `JobService` processes the actual provider transaction independently of
FerroUI activity lifetime. Outgoing operations persist a client identity before
submitting a carrier request. A process crash never triggers an automatic resend.
Queued work interrupted before its transport call can remain unresolved until a
new explicit user action; it is never reported as delivered.

Multipart SMS receipts account for each segment. Only confirmed GSM receipt or
CDMA delivered status becomes `Delivered`; forwarded, accepted, deposited,
temporary and unknown statuses retain their distinct uncertainty. MMS delivery
aggregates the actual recipients and originating subscription. Callback writes
match immutable provider fields as well as the row id, so an old receipt cannot
update a different message reusing that id. Deleted provider messages also lose
their private receipt-body copies, callbacks and transfer leases. A role revocation
clears private queued message contents. Send tombstones are bounded to 4,096
operations and a 30-day idempotency window; expiry cleanup runs at native message
maintenance and before new sends.

The attachment picker is explicit and carries the shared draft `client_id` through
selection, cancellation and failure. It copies at most eight documents and 10 MiB
into random private leases. Outgoing MMS composes an independent PDU and provider
copy before returning; releasing a UI draft cannot remove an accepted carrier
transfer. A separate non-exported `FileProvider` exposes only the message-transfer
cache subtree, validates identity and expiry, and permits writes only for native
MMS download leases. Its provider-row insert/update/delete operations are denied.
Draft leases last 30 minutes, external viewing leases 15 minutes and accepted
carrier transfers at most 48 hours, with a 256 MiB reserved transfer quota. MIME,
part count, byte lengths and nested multipart depth are bounded.

App-owned background message notifications apply the persisted shared notification
preferences without asking for permission. Android notification channels and user
settings remain final. Required Telecom call presentation is independent of the
Messaging notification preference. Full-screen call entry uses Android's actual
full-screen eligibility; foreground incoming calls have the independent native
call UI, and background calls retain their native notification actions.
The Telecom-bound `InCallService` takes a non-sticky started lease and owns one actual `phoneCall` foreground
notification while a live call exists and this package remains the selected
Dialer. Ringing calls take priority; secondary calls retain individual actions.
The notification follows the primary call without a second service or forged
foreground flags, and the final call, Telecom unbind, or service destruction
removes foreground ownership and releases the started lease. Full-screen access is used only for incoming calls; ongoing calls do
not request full-screen presentation. No boot receiver starts a call service and
no self-managed-call permission is added.

### Completion, cancellation and stale work

The JNI boundary uses FerroUI's managed Java references and bounded JSON envelopes.
No Rust or UI object address is placed in a Java callback. App discovery runs on a
bounded worker executor; the UI thread polls an operation ID through a retained
dispatcher timer. There are at most eight requests, each with a timeout and eventual
expiry. Dropping a Rust future cancels its Java operation. Location callbacks keep
only a weak activity reference and are removed when its owner is destroyed.

The media session wraps the shared decoder rather than constructing another
player. Native callbacks carry an exact decimal `u64` source generation and enter
a queue of at most 32 commands. The shared host validates the generation again
immediately before execution. Metadata and timeline values are bounded; closing
the backend releases the native session, callback queue, notification and service.

A non-exported `mediaPlayback` foreground service keeps observed user-started
playback eligible while the activity is backgrounded. A lock-screen or notification
Play action promotes the service before the decoder requests audio focus; a
five-second bound stops it if playback does not begin. Pausing/stopping releases
the foreground service. The service is `START_NOT_STICKY`, and removing the app's
task requests Stop. Foreground-service failures are reported without pretending
that registration or playback was granted. Audio focus itself is already managed
by FerroUI's Media3 player.

### Owned file handoff

The plugin's private `open_file` hook accepts only a path already resolved by the
shared AssetStore. Android rechecks that it is a direct child of
`filesDir/windows-phone/assets`, an allowed media filename and an owned regular
file. The provider XML exposes only that directory, and the provider also requires
a live unpredictable URI lease. New opens check inode, device, UID and size and
use `O_NOFOLLOW`; write, insert, update and delete requests are rejected.

There are at most 32 grants. Each expires after ten minutes, is explicitly revoked,
and is forgotten on process death. Already opened descriptors cannot be revoked
retroactively, and a receiving app can retain the content the user chose to open.
Other private files, asset JSON metadata and directory-wide grants are never
published. File handoff is not a path-bearing public `PlatformRequest`.

## Build

Use the pinned Rust toolchain, JDK **17**, Gradle **8.13**, Android SDK platform
**36**, Build Tools **36.0.0** and NDK **28.2.13676358**. The native minimum is API
**26**. The Gradle Android plugin is **8.13.0**. Set `ANDROID_HOME` to an installed
SDK. The build script sources `FerroUI/scripts/android/env.sh` for the NDK/compiler
configuration; it does not download SDKs, install APKs or change the default HOME.

```sh
rustup target add aarch64-linux-android
sdkmanager 'platforms;android-36' 'build-tools;36.0.0' 'ndk;28.2.13676358'
bash scripts/build-android.sh
bash scripts/build-android.sh --launcher --no-cargo
```

Use `--abi x86_64` with the corresponding Rust target for an x86_64 emulator build.
`--release` builds the Rust release profile and the unsigned Android release
variant; distribution signing remains a deployment configuration. `--no-cargo`
repackages an existing matching native library, which Gradle checks for existence,
ELF format and ABI before packaging. Gradle dependencies are pinned, and Rust uses
the workspace lockfile.

Debug APKs appear under
`target/android/phone-arm64-v8a-debug/outputs/apk/{app,launcher}/debug/` unless
`CARGO_TARGET_DIR` changes that root. Native artifacts and Gradle outputs stay out
of the source tree.

## Validation

`cargo test --locked -p phone-android --lib` runs portable contract tests for bounded
responses, app identities, location bounds, unknown environment observations,
role/mode separation, full-width media generations, metadata and command validation. These tests do not
require Android hardware. `scripts/test-android-contracts.sh` adds JVM receipt/address/UTF-8 regressions and exact AOSP license/source-hash checks. Manifest negative tests reject exposed native callbacks or unprotected carrier services.

`.github/workflows/android.yml` runs on manual dispatch or a main commit containing
`[verify-mobile]` or `[verify-android]`. Skipped runs cannot cancel an active Android
verification. It builds the complete arm64 Rust library and both Gradle APK
flavors, then checks their **merged APK manifests**, native ABI, exported JNI entry
and 16 KiB ELF/ZIP alignment. It also checks the Android library with the standalone
entry disabled. `scripts/verify-android-package.py` performs the
artifact checks. Successful native build outputs are cached before package
validation. The workflow retains both debug APKs and decoded manifests even when
a later verification fails, so the workflow conclusion and verification JSON must
be checked before calling an artifact verified. On success it uploads the APKs and verification metadata;
it does not install them on a device or select a HOME app.

### Focused emulator gate

`.github/workflows/android-ui.yml` runs only on manual dispatch or a main commit
containing `[verify-android-ui]`. Its concurrency and build-cache namespaces are
independent from the ARM64 package job. It builds both **x86_64** debug variants,
verifies their package boundaries, then creates a private Android **36** AVD with
KVM and software graphics. It reuses FerroUI's Android environment and emulator
helpers. The AVD is stopped and its private directory removed afterward.

The standalone debug host observes the mounted UI only when
`files/windows-phone/phone-android-smoke.enabled` contains the test's 32-digit
hexadecimal nonce. Release and embedding builds exclude the observer. It writes
an atomic private JSON snapshot every 500 ms for at most ten minutes, and stops
when the marker is removed or the host closes. It records the actual layout,
route and normal activity activation outcome; it cannot inject an activation.

`scripts/smoke-android.sh` installs both variants into its own emulator and sends
explicit-component `ACTION_VIEW` intents for **Start → Wallet**, a real Android Back event to Start, and an actual `MAIN/HOME` intent back to Start. It requires
fresh nonce/sequence observations, actual named page controls, nonzero native
system insets, an inner viewport matching the inset content slot, and a
`PhoneScreen` contained in that slot. A further phase temporarily selects the
Launcher package as HOME only inside that same private AVD. It verifies Android's
actual role holder, independently observed active launcher mode and device role,
implicit HOME resolution, a real HOME key returning Wallet to Start, and Back at
Start remaining focused. The original HOME holder is restored and compared in
normal completion and failure cleanup; the current Android user must remain
unchanged. No existing device or user launcher configuration is reused or changed.
This proves role qualification through the emulator's role manager; user-visible
HOME chooser consent and denial still require separate device qualification.

A second private debug marker (`phone-device-smoke.enabled`) enables only native
observations and local codec fixtures. The gate records pre-role permissions,
selects the qualified dialer/SMS roles **only in that newly created AVD**, injects
an emulator modem SMS, verifies the provider and shared Messaging UI, rings an
emulator call, presses the actual native Answer button, and observes Telecom Active.
It requires Android's actual `phoneCall` foreground type and posted `CallStyle`
notification, ends the modem call, and verifies both are released. It then
restores and verifies the original carrier role
holders. Thirteen codec assertions cover group/Unicode/binary round trips,
truncated lengths, part-count and nested-depth bounds without sending any MMS.
This proves emulator integration, not delivery over a real carrier. Each observed page has a real screenshot and JSON record. Failure
artifacts include the last observation, screenshot, activation output, emulator
log, logcat and crash buffer; successful build outputs are cached before runtime
checks. The CI emulator step is limited to fifteen minutes plus cleanup.

```sh
rustup target add x86_64-linux-android
sdkmanager emulator 'system-images;android-36;google_apis;x86_64'
bash scripts/build-android.sh --abi x86_64
bash scripts/build-android.sh --abi x86_64 --launcher --no-cargo
timeout --signal=TERM --kill-after=30s 900 bash scripts/smoke-android.sh
```

The `phone-android-x86_64-ui` artifact retains the UI evidence even when a runtime
assertion fails. The separate `phone-android-x86_64-debug` artifact retains APKs
only after the current run successfully builds both variants. Check `result.json`
and the workflow conclusion; the presence of an artifact alone does not prove the
UI gate passed. Cached observation files are removed before each run.

Successful packaging proves native/Java compilation and the package boundary.
Device qualification still needs actual activity recreation, user-visible HOME
chooser consent/denial, denied/revoked runtime permissions, approximate location, camera and
Storage Access Framework flows, lock-screen transports, foreground-service policy,
and external-player handoff on supported Android/OEM versions.

## Platform contracts

- [Android role requests and observation](https://developer.android.com/reference/android/app/role/RoleManager)
- [Scoped package visibility declarations](https://developer.android.com/training/package-visibility/declaring)
- [Foreground service startup and promotion](https://developer.android.com/develop/background-work/services/fgs/launch)
- [Audio focus and Android 15 foreground requirements](https://developer.android.com/media/optimize/audio-focus)
- [FileProvider content URIs and temporary grants](https://developer.android.com/reference/androidx/core/content/FileProvider)
- [Android Gradle plugin 8.13 compatibility](https://developer.android.com/build/releases/agp-8-13-0-release-notes)
- [Android emulator command-line options](https://developer.android.com/studio/run/emulator-commandline)
- [Emulator hardware and graphics acceleration](https://developer.android.com/studio/run/emulator-acceleration)
- [ADB explicit activity intents](https://developer.android.com/tools/adb)

- [Default dialer and native in-call UI requirements](https://developer.android.com/develop/connectivity/telecom/dialer-app)
- [Default SMS application and protected delivery components](https://developer.android.com/reference/android/provider/Telephony)
- [Native SMS/MMS transport and explicit subscriptions](https://developer.android.com/reference/android/telephony/SmsManager)
- [SMS delivery-status encoding](https://developer.android.com/reference/android/telephony/SmsMessage#getStatus())
- [GSM TP-Status definitions, TS23.040 §9.2.3.15](https://www.etsi.org/deliver/etsi_ts/123000_123099/123040/19.00.00_60/ts_123040v190000p.pdf)
- [Notification listener and current-profile callbacks](https://developer.android.com/reference/android/service/notification/NotificationListenerService)
- [Public native contact editing](https://developer.android.com/identity/providers/contacts-provider/modify-data)
- [Launcher shortcut-host access](https://developer.android.com/reference/android/content/pm/LauncherApps)
- [Android public settings actions](https://developer.android.com/reference/android/provider/Settings)
- [AOSP MMS codec pin, license and exact source provenance](platform/vendor/aosp-mms/UPSTREAM.json)
