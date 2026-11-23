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
| Device applications | Queries visible `MAIN/LAUNCHER` activities in the current profile; validates exported/enabled/permission-accessible components, skips itself and revalidates membership before launch. No `QUERY_ALL_PACKAGES` request. |
| HOME | API 29+ `RoleManager.ROLE_HOME` request; API 26–28 home settings fallback. The result is observed when the app resumes. |
| Settings | Explicit application, network, notification and default-app pages. |
| Sharing | Native text `ACTION_SEND` chooser. Completion means Android opened the chooser, not that another person received content. |
| Haptics | `View.performHapticFeedback`, respecting device/user settings without requesting a vibration privilege. |
| Notifications | Android channels, explicit Android 13+ notification permission and native foreground alert posting. Denial remains denial; opening system settings does not imply a grant. |
| Location | Explicit fine/coarse runtime consent, approximate-location support, one native fix, freshness/range checks, cancellation and a 15-second acquisition timeout. No background location permission. |
| Environment | Actual battery, charging, external-power, power-saver and validated-network observations. Missing observations stay unknown. |
| Media controls | Native `MediaSession`, metadata, elapsed position/duration, play/pause/stop, previous/next and bounded seek commands. |
| File handoff | Expiring read-only `content:` URI grants for individual owned media assets, with MIME and `ACTION_VIEW`. |

Package enumeration does not include other work/private profiles. This version
does not register as an SMS handler, dialer, notification listener, accessibility
service, device administrator or privileged system package. Such roles are separate
contracts, not consequences of holding HOME. Android permission decisions and the
platform's background-execution policy remain authoritative.

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
require Android hardware.

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

Successful packaging proves native/Java compilation and the package boundary.
Device qualification still needs actual activity recreation, HOME consent and
reentry, denied/revoked runtime permissions, approximate location, camera and
Storage Access Framework flows, lock-screen transports, foreground-service policy,
and external-player handoff on supported Android/OEM versions.

## Platform contracts

- [Android role requests and observation](https://developer.android.com/reference/android/app/role/RoleManager)
- [Scoped package visibility declarations](https://developer.android.com/training/package-visibility/declaring)
- [Foreground service startup and promotion](https://developer.android.com/develop/background-work/services/fgs/launch)
- [Audio focus and Android 15 foreground requirements](https://developer.android.com/media/optimize/audio-focus)
- [FileProvider content URIs and temporary grants](https://developer.android.com/reference/androidx/core/content/FileProvider)
- [Android Gradle plugin 8.13 compatibility](https://developer.android.com/build/releases/agp-8-13-0-release-notes)
