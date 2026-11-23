# Host integration

The phone's native FerroUI views and view model are shared. Platform adapters implement the host operations that a normal application can perform. These adapters do not confer Windows Phone kernel privileges, cellular transport, restored Microsoft services, or background alarm scheduling.

## Reusable boundary

`phone-host` is independent of FerroUI and `phone-domain`. It owns the serialized `HostRequest`, `HostEnvironment`, `HostPlatform` and `HostPermission` contracts; validated URI construction; native data directories, atomic replacement, default-handler activation and power observations; notification/location adapters; bounded HTTPS weather; safe export filenames; picker admission rules; and bounded background work. `phone-state` re-exports the contracts. The extracted `phone-sdk` exposes typed app services and file chooser futures; independently compiled built-in and extension apps use that same boundary.

The UI still uses declarative RSX, shared phone control templates and the existing WinUI-derived theme. Observations update the ReactiveUI revision without modifying the saved phone document. DynamicData projections remain responsible for stable collection identity. Host power-saving observations pause decorative motion and live-tile cycling without changing the user's saved Battery Saver preference.

| Facility | Browser host | Windows/macOS/Linux desktop host |
| --- | --- | --- |
| App UI and local data | Shared Rust/RSX controls and view model | Same controls and view model |
| Contacts | Bounded vCard import, file download, file-capable Web Share | Same codec with FerroUI Open/Save pickers; sharing saves an attachment copy |
| Calendar | Bounded iCalendar import, file download, file-capable Web Share | Same codec with native pickers; sharing saves an attachment copy |
| Office | Shared OOXML/TXT/CSV import; OOXML download and file-capable sharing | Same codec; single-copy Save picker and multiple-copy Folder picker for attachment workflows |
| Photos | Picker imports normalized JPEG copies; camera capture where permitted | App-owned imports, live camera preview and JPEG capture; shared image cache loads stored bytes |
| Music/video | HTML media player, playback controls, Media Session integration | Embedded FerroUI native player; play/pause/stop and app volume/mute; explicit external-player handoff |
| Share photo/media | Browser sharing support determines available targets | Save the actual asset bytes through a native picker, then attach in another app |
| Backup | Portable JSON/media envelope; staged immutable assets before document commit | Compatible envelope; native pickers, fresh assets, recovery snapshot and atomic document replacement |
| Battery | Optional Battery Status observations | Linux sysfs, Windows `GetSystemPowerStatus`, macOS IOKit |
| Device notifications | Explicit permission request; permitted foreground Calendar/alarm banners use worker/page notification APIs | Win32 notification-area banners, macOS UserNotifications, Linux Freedesktop Notifications |
| Foreground alarm audio | Browser alert adapter | Separate looping native player, generated PCM tone, ringer volume and stop/snooze |
| Storage | Origin usage/quota estimates, observed persistence, explicit persistence request | Application data files; no browser quota/persistence prompt |
| Calling/mail/text | Validated URI handoff; the destination app owns the operation | Validated default-handler activation with the same ownership boundary |
| Location/weather | Explicit browser geolocation and Open-Meteo fetch | Explicit WinRT/CoreLocation/GeoClue2 acquisition; bounded Open-Meteo HTTPS request |
| SDK file chooser | Synchronous gesture dispatch, bounded File read and typed result | FerroUI single-file picker, bounded cooperative read, explicit cancel/error/result |

Windows/macOS/Linux adapter source is present. Compilation and pure unit tests do not demonstrate real permission dialogs, camera hardware, audio devices, codecs, picker behavior or default-handler delivery on all three operating systems. Native providers need the platform dependencies and application identity described in [native host packaging](../crates/phone-desktop/platform/README.md). Android and iOS currently use the browser host; dedicated native mobile launch targets have not been added.

## File and media lifetime

The desktop uses FerroUI's `IStorageProvider` rather than platform-specific picker widgets. `picker_io` bounds reads and yields between 256 KiB chunks. Pure asset, codec and backup work runs through a file worker with a bounded 32-job queue. Blocking OS permission, notification, location and network calls use a separate worker with a bounded four-job queue, so a pending consent dialog does not stall file processing. A standard Rust `Future` wakes the caller's executor. UI controls and the shared image-cache decoder remain on the UI dispatcher. Cancelled queued jobs are skipped; already running bounded operations finish and their results are checked against session/generation ownership.

The SDK chooser admits at most 16 canonical file extensions and a requested size of 1 byte through 16 MiB. The native provider must return at most one file, and its actual suffix is checked after filtering. Native reads check the SDK request lifetime between chunks. `None` means user cancellation; errors remain errors. Closing an app, changing its availability, replacing the phone document or dropping its future prevents a late completion from delivering bytes into a new scope. FerroUI exposes no native dialog cancellation token, so abandoning a result does not itself dismiss an already displayed OS modal. The native MIME value is a suffix hint, not a content-type validation result.

Native media keys are identifiers, not paths. The asset store accepts only `media:` followed by 1–128 ASCII letters, digits, underscores or hyphens, beginning with a letter/digit. App-owned files use those keys and admitted media extensions. Manifests and payload reads are bounded; symlink payloads are rejected. Selected source files are copied into app storage. Deleting an imported asset does not delete the user's original selected file.

The browser stores media in IndexedDB. The shared image cache still admits bounded headers/dimensions before native decoding and keeps separate thumbnail/full-view variants. Browser imports normalize images to oriented JPEG copies; desktop imports preserve admitted encoded image bytes and render through the same bounded cache. These are different import paths and should not be described as lossless, identical image processing.

Camera captures use FerroUI's actual `CameraCapture`/`CameraSession` services and `TexturePresenter` for shared frames. A capture is limited to 16 MiB encoded and 16,777,216 decoded pixels. Rotated/mirrored JPEG captures are normalized on the file worker because the pinned renderers ignore EXIF orientation. Canonical captures preserve the original compressed bytes. Only one capture/save is admitted at a time. Camera Roll changes only after the asset is written and the image-cache decoder accepts it; stale and failed captures remove their staged asset.

Embedded playback uses one host-owned `MediaPlayer`. `phone_sdk::HostSurface` lets app viewports mount native texture presenters without importing platform providers. Presenters are weakly tracked; replacing a reactive view releases that presentation without stopping the current session. Native player/session state drives the playing indicator, and the timer avoids resubmitting unchanged frames. New selections invalidate older loads. Leaving Camera, locking the phone, replacing its data or closing the window stops camera use; window shutdown/document replacement also release player resources. Renderer/import failures stop the affected source and request an explicit restart after recovery.

Windows uses FerroUI Media Foundation providers, macOS AVFoundation/IOSurface, and Linux GStreamer with portal/PipeWire for camera access. Linux selects EGL/OpenGL ES 3 for its DMA-BUF media bridge. It never falls back from a denied camera portal to direct V4L2 access. Missing GStreamer 1.24 app/video/allocators or EGL/GLES development packages cause the framework to compile its media-unavailable bridge. A software-rendered shell also cannot supply shared native media textures; neither configuration counts as working native media. Minimizing or hiding a desktop window sends the SDK suspension event and stops camera use. Ordinary focus loss is excluded because an OS consent dialog may take focus while a camera start is pending.

Desktop external playback reports a launch request and leaves in-app playback marked inactive. The external media app owns codecs, transport controls, volume and its playback lifetime. The phone cannot confirm delivery to an arbitrary default handler.

Desktop document sharing exports actual Office copies. Single-document sharing uses Save; multiple documents use a chosen folder, at most 128 selected documents and 64 MiB total exported bytes, with unique bounded filenames. Existing destination files are preserved and partial failure reports completed copies. This is an attachment-copy workflow, not a system share sheet or proof of delivery to another application.

## Restore transaction

Both hosts validate the supported phone version, document and manifest before replacing live data. The portable envelope limits are 256 MiB encoded JSON, 16 MiB phone JSON, 10,000 media entries, 16 MiB per image, 128 MiB per audio/video item, and 192 MiB aggregate decoded media. Native format admission may reject a codec/extension that a browser can preserve. JSON/base64 processing can hold multiple representations in memory, so these limits are upper bounds rather than performance promises.

Each supplied media item receives a fresh immutable key. Photo, track, Start-wallpaper and independent lock-wallpaper references are remapped together. Existing assets are never overwritten by matching identifiers from a backup. Missing assets are reported.

`PhoneViewModel::replace_data_after_save` validates the candidate and calls a fallible host save operation before replacing the document, navigation, projections or document epoch. Browser quota/concurrent-tab errors and desktop disk errors therefore leave the current view model intact. A successful transaction does not repeat the save through the ordinary callback. Failed commits remove staged new assets; post-commit image cleanup never removes committed assets.

The desktop preserves the latest pre-restore phone document beside the active data file immediately before replacement. Its old immutable media remain available to that recovery document. Application-owned document writes use a same-directory `create_new` staging file, flush its bytes, and replace the target. Windows replacement uses `MoveFileExW`; Unix uses rename. Picker-selected exports retain the host provider's own write semantics.

Application data uses `%LOCALAPPDATA%` on Windows, `~/Library/Application Support` on macOS, and `$XDG_DATA_HOME` or `~/.local/share` on Linux. The former data location is copied forward when appropriate; migration failures preserve the previous location. Backups use the actual active path, including recovery-mode locations.

## Notifications, media keys and permission ownership

Reading `HostEnvironment` does not request permissions. The Settings commands `RequestNotificationPermission` and `RequestPersistentStorage` are explicit user actions. Unsupported, unknown, prompt, granted and denied notification states remain distinguishable.

A foreground Calendar/alarm alert issues a device banner only when it becomes the active alert, global notifications and that app's banners are enabled, host permission is granted, and in-app lock privacy allows it. The host banner is silent because foreground alert adapters own sound and any supported haptics. No background wake-up or delivery while the app is closed is implemented.

Native notification observations are real host calls. Linux confirms a Freedesktop Notifications server; its protocol has no standard per-app permission prompt. Windows queries `SHQueryUserNotificationState` and reports allowed only while the shell accepts notifications. Other Windows interruption states remain unknown and are observed again on activation/periodically. macOS reads `UNNotificationSettings` and requests authorization only after the explicit command. Missing service/identity, denied consent and API errors are never translated to a successful grant. A successful delivery result means the OS accepted the request; user policy can still suppress a visible banner. macOS notification activation focuses the app; Windows/Linux notification activation routing is not yet implemented.

macOS notification/location adapters require the supplied `.app` identity and camera/location usage descriptions. Their Objective-C callbacks retain their own state through bounded waits and never retain pointers into a Rust stack frame. Windows location consent starts on the foreground UI dispatcher, and the request uses an explicit age/timeout bound. Unix acquisition runs separately from file work. Location failures never substitute default or IP-derived coordinates. Weather is fetched only for an explicit request, over HTTPS to a fixed Open-Meteo endpoint with no redirects, a 15-second timeout and a 64 KiB response limit.

Desktop foreground alarm audio uses a generated two-second PCM alert and the native media provider, separately from ordinary music/video. It respects the saved ringer volume and stops with the alert. The original ringtone catalog, native haptics, OS media-key integration and OS wake/scheduled background alarm delivery remain unimplemented.

The browser prefers its active service-worker notification registration and uses page notifications where supported. Clicking a notification can only focus/open this application's fixed URL. The worker cannot obtain an arbitrary navigation destination from notification data.

The Media Session adapter owns the current media element's listeners, metadata, playback/position state and play/pause/stop/next/previous/seek handlers. Switching players detaches previous listeners. Unsupported individual actions are tolerated. Playback generations prevent an older asynchronous source load from replacing a newer selection; Pause cancels a pending replacement. These are browser host integrations, not Windows Phone's original audio stack.

## Interchange and validation

[People and Calendar interchange](pim-interchange.md) describes the exact supported vCard/iCalendar subset. Imports are local copies with fresh identifiers, not account synchronization. Unsupported semantic content is rejected rather than silently flattened. Browser file sharing prepares a complete file synchronously before invoking the host share sheet, preserving user activation; cancellation does not fall back to copying personal data.

Focused tests cover fallible restore commit order, no-write environment updates, unknown battery states, weighted Linux batteries, atomic file failure, worker wake/cancellation/isolation, native asset boundaries and backup codecs, file-name and chooser limits, validated coordinates/weather, notification text handling, all eight camera orientation transforms, encoded JPEG output, and generated PCM structure. Browser tests use simulated adapters for permission/storage/media APIs. The full rendered suite additionally exercises real Rust byte callbacks and native menus/settings; it does not open host permission dialogs, capture hardware, send notifications, play sound, send content to another app or prove platform behavior.

## Primary API references

- [Notifications Standard](https://notifications.spec.whatwg.org/).
- [Storage Standard](https://storage.spec.whatwg.org/).
- [Media Session specification](https://www.w3.org/TR/mediasession/).
- [Windows power status](https://learn.microsoft.com/en-us/windows/win32/api/winbase/ns-winbase-system_power_status).
- [Windows file replacement](https://learn.microsoft.com/en-us/windows/win32/api/winbase/nf-winbase-movefileexw).
- [Windows notification state](https://learn.microsoft.com/en-us/windows/win32/api/shellapi/nf-shellapi-shqueryusernotificationstate).
- [Windows desktop location](https://learn.microsoft.com/en-us/windows/apps/develop/maps-and-location/get-location).
- [Apple UserNotifications](https://developer.apple.com/documentation/usernotifications/unusernotificationcenter).
- [Apple location authorization](https://developer.apple.com/documentation/corelocation/requesting-authorization-to-use-location-services).
- [Freedesktop Notifications](https://specifications.freedesktop.org/notification/latest-single/).
- [GeoClue2 Client](https://www.freedesktop.org/software/geoclue/docs/gdbus-org.freedesktop.GeoClue2.Client.html).
- [Open-Meteo forecast API](https://open-meteo.com/en/docs).
- [Apple power-source snapshots](https://developer.apple.com/documentation/iokit/1523839-iopscopypowersourcesinfo).
- [Linux power-supply class](https://docs.kernel.org/power/power_supply_class.html).
- [Pinned FerroUI source](../FerroUI), including platform storage providers and shared controls.
