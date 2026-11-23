# Native host services

The desktop application uses the same app crates and SDK as the browser host.
Applications request services through `HostRequest`; this host owns consent,
native resources, file I/O, and observed service results. Native preview controls
are supplied through `phone_sdk::HostSurface` rather than platform code in the
Camera or Video app.

## Implemented adapters

| Service | Windows | macOS | Linux |
| --- | --- | --- | --- |
| Embedded audio/video | FerroUI Media Foundation provider | FerroUI AVFoundation provider | FerroUI GStreamer provider |
| OS media keys and now playing | Per-window System Media Transport Controls | MediaPlayer remote commands and now-playing center | MPRIS 2 player on the session bus |
| Camera preview and JPEG capture | FerroUI Media Foundation camera | FerroUI AVFoundation camera | FerroUI camera portal and PipeWire |
| Frame presentation | Shared OpenGL textures | IOSurface texture importer | Shared OpenGL textures |
| Device notification | Win32 notification-area banner | UserNotifications | Freedesktop Notifications on the session bus |
| One-shot location | WinRT Geolocator | CoreLocation | GeoClue2 on the system bus |
| Current weather | Bounded HTTPS request to Open-Meteo after explicit location acquisition | Same | Same |
| Files and document/media copies | FerroUI storage provider | FerroUI storage provider | FerroUI storage provider |

The media player supports imported local audio/video and explicitly selected
HTTPS streams, play, pause, stop, seek, ten-second forward skips, repeat restart,
and the shared SDK queue. Native duration/position and the app's media volume
and mute settings drive the visible state. The More
menu also exposes an explicit handoff to the device's external media app; that
handoff reports an accepted launch request, not observed external playback.
Stream URLs pass the shared public HTTPS admission rules; codec, network access,
buffering and redirects belong to the native media provider. Streaming does not
use the bounded SDK HTTP-response downloader and does not download an offline copy.

Native OS controls publish the current title, artist, observed playback state,
position/duration and queue capabilities. They route play, pause, stop,
next/previous and seek commands back to the same player. Linux additionally
accepts volume and Raise. Next/previous from the OS preserve paused playback.
End-of-track advancement and repeat use the SDK queue, including app playlists
and Podcasts. Windows uses the real application HWND; macOS requires the supplied
application bundle identity; Linux requires a working session bus and a desktop
that exposes MPRIS players. Capability remains unknown until registration and
becomes unavailable if the OS rejects it or the connection fails.

The reusable `phone-host::native::media_session` layer contains no UI or decoder
references. Native callbacks enqueue at most 32 commands with a source-generation
stamp. Stale positions, commands for a replaced document/source, nonfinite
positions and invalid volume requests are discarded. Source paths and network
URLs are never published as OS metadata. Closing or externally handing off
playback releases native handlers and now-playing state. A private-bus protocol
test checks real MPRIS discovery, properties, controls, stale-track rejection and
service teardown without operating the person's desktop session.

Camera requests start only after the app's command. Leaving Camera, locking the
phone, replacing the phone document, or closing the window stops the session and
cancels pending starts. Replacing a view during a reactive render does not stop
the shared camera. Captures are bounded to 16 MiB encoded and 16,777,216 pixels.
Rotated/mirrored pixels are normalized on the file worker because the pinned
renderers do not apply EXIF orientation; canonical captures preserve compressed
bytes. A capture is added to Camera Roll only after storage and decoding succeed.

Minimizing/hiding the desktop window sends the SDK suspension event and stops
the camera. Ordinary focus loss does not cancel a start: operating-system
permission dialogs can take focus while their request is in progress.

Foreground alarms use a separate looping native player and a generated PCM
alert, respecting ringer volume and stop/snooze commands. Notifications are
silent to avoid playing the alert twice. This is not the original Windows Phone
ringtone catalog. Desktop haptics and wake-from-sleep
alarm scheduling are not implemented by these adapters.

File and image work uses one bounded worker; blocking OS consent, notification,
location, and HTTP operations use a separate bounded worker. Native dialogs run
through FerroUI's UI-thread storage contracts. The SDK chooser returns one file,
explicit cancellation, or an error, with a maximum requested size of 16 MiB.
Picker filters are checked again before the selected file is read. Its MIME
value is an advisory extension hint because `IStorageFile` exposes no MIME API.

Document sharing creates actual copies through the host's save/folder picker.
Multiple documents use bounded, platform-safe unique filenames; existing files
are preserved, and partial failures report how many copies completed. The app
then asks the user to attach those copies in their sharing application. This
workflow does not claim native share-sheet delivery.

## Windows build prerequisites

Install the MSVC C++ build tools, Windows SDK and a compatible LLVM/libclang.
The pinned ANGLE dependency generates bindings against the installed MSVC
standard library. CI uses the Windows 2022 image with VS2022/MSVC 14.44 and LLVM
20.1.8. MSVC 14.52 requires Clang 22 or newer; the [Microsoft compatibility
table](https://github.com/microsoft/STL/wiki/Macro-_MSVC_STL_UPDATE) lists the
supported pairs. Set `LIBCLANG_PATH` to the matching installation's `bin`
directory so bindgen loads the same compiler that was checked:

```powershell
$env:LIBCLANG_PATH = 'C:\Program Files\LLVM\bin'
& "$env:LIBCLANG_PATH\clang.exe" --version
'#include <string>' | & "$env:LIBCLANG_PATH\clang.exe" --target=x86_64-pc-windows-msvc -x c++ -std=c++17 -fsyntax-only -
```

The workflow runs this header check before compiling Rust, and scopes Windows
caches by runner, compiler and framework pin. It keeps ANGLE/media enabled and
does not override the standard library's compiler-version guard.

## Windows notification and location state

Win32 notification-area banners have no per-app consent prompt. The host queries
`SHQueryUserNotificationState` and reports allowed only while the shell accepts
notifications; other interruption states remain unknown until the next
observation. Notification acceptance still does not guarantee visible delivery.
Location access uses the actual WinRT consent operation on the foreground UI
dispatcher, followed by a bounded one-shot position request.

## macOS packaging

Run on macOS from a checkout with its FerroUI submodule:

```sh
crates/phone-desktop/platform/macos/package.sh
open target/WindowsPhone.app
```

The script builds the release executable and creates an ad-hoc signed `.app`
with bundle identifier `com.ferroui.WindowsPhone`, camera and location usage
descriptions, and a minimum system version of macOS 11. FerroUI's macOS native
bridge is statically linked; its system frameworks remain system dependencies.
Distribution signing, sandbox entitlements when enabled, and notarization are
deployment responsibilities. `CARGO_TARGET_DIR` is supported.

UserNotifications requires bundle identity. A plain `cargo run` executable
reports notifications unsupported and asks for the packaged app when location
is requested. It does not claim consent was granted. The host observes current
notification settings on activation and periodically; it requests authorization
only through the explicit command.

## Linux prerequisites

FerroUI's pinned native media build requires GStreamer 1.24 development packages
for app/video/allocators plus EGL/GLES development libraries. Codec support
depends on the GStreamer plugins installed on the target system. Camera preview
requires a working camera portal and PipeWire. The host deliberately selects
`LinuxCameraProfile::Portal`; it does not silently bypass a denied portal by
opening a V4L2 device.

The Linux host selects EGL/OpenGL ES 3 for the media bridge's DMA-BUF texture
imports. Software rendering remains a shell fallback; it cannot provide native
media textures and reports that limitation when media is requested.

When the required native development dependencies are unavailable, FerroUI
compiles its documented media-unavailable bridge. Playback and camera then
report an unsupported-provider error; a successful Rust build alone does not
establish media support on that machine.

Install `com.ferroui.WindowsPhone.desktop` from this directory's `linux` folder
in the user's applications directory, and place `phone-desktop` on `PATH`, or
set the desktop entry's `Exec` to the installed executable. GeoClue needs that
desktop identity, its system daemon, and the desktop's authorization agent.
Missing services, denial, and timeout are returned as errors. No fallback
coordinates or IP-derived location are substituted.

Device notifications use `org.freedesktop.Notifications`. Availability is
confirmed through `GetServerInformation`; Linux exposes no standard per-app
permission prompt through that protocol. Do Not Disturb or desktop policy may
suppress a notification even after the server accepts it.

## Verification scope

Unit tests cover filename/path boundaries, picker admission limits, coordinate
validation, bounded weather responses, notification escaping, file-worker
isolation, native camera orientation for all eight EXIF transformations, and
PCM alert structure. They do not request permissions, capture a camera, send a
notification, play sound, or contact a location/weather service.

The MPRIS protocol integration test is opt-in and must use a fresh test bus:

```sh
dbus-run-session -- env PHONE_MPRIS_TEST_SESSION=1 cargo test -p phone-host private_bus_media_controls -- --ignored
```

These tests verify the transport boundary, not hardware media keys or codec
playback. macOS command-center eligibility and Windows shell visibility still
need OS/device testing with the packaged application.

Windows and macOS provider behavior requires running the packaged application
on those operating systems. Linux media also needs the development/runtime
dependencies described above. Hardware-specific controls, background execution,
notification activation routes, original tones, and OS-level Phone integration
remain separate parity work; these adapters do not establish 100% Windows Phone
behavior.

## Primary API references

- [Pinned FerroUI portable-media sample](https://github.com/wieslawsoltes/FerroUI/tree/db2e8859cd4b67bac870839ea70503b8babf541f/samples/PortableMedia)
- [Microsoft notification-area API](https://learn.microsoft.com/en-us/windows/win32/api/shellapi/ns-shellapi-notifyicondataw)
- [Microsoft notification state](https://learn.microsoft.com/en-us/windows/win32/api/shellapi/nf-shellapi-shqueryusernotificationstate)
- [Microsoft desktop location guidance](https://learn.microsoft.com/en-us/windows/apps/develop/maps-and-location/get-location)
- [Microsoft per-window media transport controls](https://learn.microsoft.com/en-us/windows/win32/api/systemmediatransportcontrolsinterop/nf-systemmediatransportcontrolsinterop-isystemmediatransportcontrolsinterop-getforwindow)
- [Microsoft media transport state and timeline](https://learn.microsoft.com/en-us/windows/apps/develop/media-playback/system-media-transport-controls)
- [Apple UserNotifications](https://developer.apple.com/documentation/usernotifications/unusernotificationcenter)
- [Apple location authorization](https://developer.apple.com/documentation/corelocation/requesting-authorization-to-use-location-services)
- [Apple remote media commands](https://developer.apple.com/documentation/mediaplayer/mpremotecommandcenter)
- [Apple now-playing information](https://developer.apple.com/documentation/mediaplayer/mpnowplayinginfocenter)
- [MPRIS 2 player specification](https://specifications.freedesktop.org/mpris/latest/Player_Interface.html)
- [Freedesktop Notifications specification](https://specifications.freedesktop.org/notification/latest-single/)
- [GeoClue2 Client API](https://www.freedesktop.org/software/geoclue/docs/gdbus-org.freedesktop.GeoClue2.Client.html)
- [Open-Meteo forecast API](https://open-meteo.com/en/docs)
