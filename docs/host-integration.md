# Host integration

The phone's native FerroUI views and view model are shared. Platform adapters implement the host operations that a normal application can perform. These adapters do not confer Windows Phone kernel privileges, cellular transport, restored Microsoft services, or background alarm scheduling.

Native Android/iOS hosts and replaceable platform services are described in [Platform and shell architecture](platform-architecture.md). It is the current source for the hosting-mode matrix, mobile capabilities, operation lifetimes and embedding API; the tables below detail the existing shared file/media and desktop/browser behavior.

## Reusable boundary

`phone-host` is independent of FerroUI and `phone-domain`. It owns the serialized `HostRequest`, `HostEnvironment`, `HostPlatform` and `HostPermission` contracts; validated URI construction; native data directories, atomic replacement, default-handler activation and power observations; notification/location and OS media-session adapters; bounded HTTPS reads; safe export filenames; picker admission rules; and bounded background work. `phone-state` re-exports the contracts. The extracted `phone-sdk` exposes typed app services, file chooser and HTTPS futures, and a shared playback queue; independently compiled built-in and extension apps use that same boundary.

The UI still uses declarative RSX, shared phone control templates and the existing WinUI-derived theme. Observations update the ReactiveUI revision without modifying the saved phone document. DynamicData projections remain responsible for stable collection identity. Host power-saving observations pause decorative motion and live-tile cycling without changing the user's saved Battery Saver preference.

| Facility | Browser host | Windows/macOS/Linux desktop host |
| --- | --- | --- |
| App UI and local data | Shared Rust/RSX controls and view model | Same controls and view model |
| Contacts | Bounded vCard import, file download, file-capable Web Share | Same codec with FerroUI Open/Save pickers; sharing saves an attachment copy |
| Calendar | Bounded iCalendar import, file download, file-capable Web Share | Same codec with native pickers; sharing saves an attachment copy |
| Office | Shared OOXML/TXT/CSV import; OOXML download and file-capable sharing | Same codec; single-copy Save picker and multiple-copy Folder picker for attachment workflows |
| Photos | Picker imports normalized JPEG copies; camera capture where permitted | App-owned imports, live camera preview and JPEG capture; shared image cache loads stored bytes |
| Music/video | HTML media player, shared queue, seek/repeat/shuffle and Media Session integration | Embedded FerroUI native player; shared queue, seek/repeat, app volume/mute and explicit external-player handoff |
| OS media controls | Media Session metadata, timeline and supported transport actions | Windows per-window SMTC, macOS MediaPlayer commands/now playing, Linux MPRIS 2 |
| Share photo/media | Browser sharing support determines available targets | Save the actual asset bytes through a native picker, then attach in another app |
| Backup | Portable JSON/media envelope; staged immutable assets before document commit | Compatible envelope; native pickers, fresh assets, recovery snapshot and atomic document replacement |
| Battery | Optional Battery Status observations | Linux sysfs, Windows `GetSystemPowerStatus`, macOS IOKit |
| Device notifications | Explicit permission request; permitted foreground Calendar/alarm banners use worker/page notification APIs | Win32 notification-area banners, macOS UserNotifications, Linux Freedesktop Notifications |
| Foreground alarm audio | Browser alert adapter | Separate looping native player, generated PCM tone, ringer volume and stop/snooze |
| Storage | Origin usage/quota estimates, observed persistence, explicit persistence request | Application data files; no browser quota/persistence prompt |
| Calling/mail/text | Validated URI handoff; the destination app owns the operation | Validated default-handler activation with the same ownership boundary |
| Location/weather | Explicit browser geolocation; bounded city search and forecasts | Explicit WinRT/CoreLocation/GeoClue2 acquisition; bounded Open-Meteo search/forecast reads |
| SDK file chooser | Synchronous gesture dispatch, bounded File read and typed result | FerroUI single-file picker, bounded cooperative read, explicit cancel/error/result |
| SDK HTTPS GET | Credential-free CORS fetch, no redirects, streamed size admission and cancellation | Credential-free direct HTTPS, public-IP resolution, no redirects and bounded worker reads |
| Offline updates | Scope-owned precache; downloaded updates wait for old app windows to close; explicit Settings check | Install a newer application build through the platform's distribution process |

Windows/macOS/Linux adapter source is present. Compilation and pure unit tests do not demonstrate real permission dialogs, camera hardware, audio devices, codecs, picker behavior, system media-key routing or default-handler delivery on all three operating systems. The Linux MPRIS adapter also has an opt-in private-bus protocol test, which exercises actual D-Bus calls without interacting with the person's desktop. Native providers need the platform dependencies and application identity described in [native host packaging](../crates/phone-desktop/platform/README.md). Android and iOS now have dedicated FerroUI native launch targets, sandbox-aware shared services and packaging/verification workflows. See the platform document and per-platform README files for their supported operations and qualification limits.

## File and media lifetime

The shared native host uses FerroUI's `IStorageProvider` rather than platform-specific picker widgets. `picker_io` bounds reads and yields between 256 KiB chunks. Pure asset, codec and backup work runs through a file worker with a bounded 32-job queue. Blocking OS permission, notification, location and network calls use a separate worker with a bounded four-job queue, so a pending consent dialog does not stall file processing. A standard Rust `Future` wakes the caller's executor. UI controls and the shared image-cache decoder remain on the UI dispatcher. Cancelled queued jobs are skipped; already running bounded operations finish and their results are checked against session/generation ownership.

The SDK chooser admits at most 16 canonical file extensions and a requested size of 1 byte through 16 MiB. The native provider must return at most one file, and its actual suffix is checked after filtering. Native reads check the SDK request lifetime between chunks. `None` means user cancellation; errors remain errors. Closing an app, changing its availability, replacing the phone document or dropping its future prevents a late completion from delivering bytes into a new scope. FerroUI exposes no native dialog cancellation token, so abandoning a result does not itself dismiss an already displayed OS modal. The native MIME value is a suffix hint, not a content-type validation result.

Native media asset keys are identifiers, not paths. The asset store accepts only `media:` followed by 1–128 ASCII letters, digits, underscores or hyphens, beginning with a letter/digit. App-owned files use those keys and admitted media extensions. Manifests and payload reads are bounded; symlink payloads are rejected. Selected source files are copied into app storage. Deleting an imported asset does not delete the user's original selected file. HTTPS streams own no local payload: removing the final library reference stops a matching active player and removes the entry without attempting a remote deletion.

The browser stores media in IndexedDB. The shared image cache still admits bounded headers/dimensions before native decoding and keeps separate thumbnail/full-view variants. Browser imports normalize images to oriented JPEG copies; desktop imports preserve admitted encoded image bytes and render through the same bounded cache. These are different import paths and should not be described as lossless, identical image processing.

Camera captures use FerroUI's actual `CameraCapture`/`CameraSession` services and `TexturePresenter` for shared frames. A capture is limited to 16 MiB encoded and 16,777,216 decoded pixels. Rotated/mirrored JPEG captures are normalized on the file worker because the pinned renderers ignore EXIF orientation. Canonical captures preserve the original compressed bytes. Only one capture/save is admitted at a time. Camera Roll changes only after the asset is written and the image-cache decoder accepts it; stale and failed captures remove their staged asset.

Embedded playback uses one host-owned `MediaPlayer`. `phone_sdk::HostSurface` lets app viewports mount native texture presenters without importing platform providers. Presenters are weakly tracked; replacing a reactive view releases that presentation without stopping the current session. Native player/session state drives the playing indicator, and the timer avoids resubmitting unchanged frames. New selections invalidate older loads. Leaving Camera, locking the phone, replacing its data or closing the window stops camera use; window shutdown/document replacement also release player resources. Renderer/import failures stop the affected source and request an explicit restart after recovery.

Music, Video and Podcasts use the SDK queue for previous/next, repeat, shuffle and end-of-track advancement. Host commands refer to the active source and document generation. Native resume records the provider's playback intent before scheduling its future, so a subsequent Pause supersedes it. Seek accepts only finite nonnegative seconds up to one year and clamps to a known duration; ten-second forward/back controls use the same provider seek operation. Exact pause/end positions are reported before checkpoint events. A completed end event cannot overwrite the state of a synchronously selected successor. OS next/previous preserve a paused queue; reaching the end while playing advances with playback intent intact.

Windows uses FerroUI Media Foundation providers, macOS AVFoundation/IOSurface, and Linux GStreamer with portal/PipeWire for camera access. Linux selects EGL/OpenGL ES 3 for its DMA-BUF media bridge. It never falls back from a denied camera portal to direct V4L2 access. Missing GStreamer 1.24 app/video/allocators or EGL/GLES development packages cause the framework to compile its media-unavailable bridge. A software-rendered shell also cannot supply shared native media textures; neither configuration counts as working native media. Minimizing or hiding a desktop window sends the SDK suspension event and stops camera use. Ordinary focus loss is excluded because an OS consent dialog may take focus while a camera start is pending.

Desktop external playback reports a launch request and leaves in-app playback marked inactive. The external media app owns codecs, transport controls, volume and its playback lifetime. The phone cannot confirm delivery to an arbitrary default handler.

## Network requests and streaming

`AppContext::get` requires the app's declared Network permission and returns a typed SDK task. Admission accepts at most eight pending tasks, a response limit from 1 byte through 2 MiB, and an HTTPS URL of at most 4096 bytes. URL user information, literal IP addresses, local/reserved host-name suffixes, control characters and nonstandard ports are rejected. Fragments are removed. These are explicit reads initiated by app commands; they do not grant a general-purpose authenticated account connection.

Both transports use a 15-second timeout and reject redirects, so a moved feed must use its final HTTPS address. The browser omits credentials and referrers, requires CORS, streams chunks through the requested size limit, and aborts when the SDK task becomes invalid. The browser owns DNS/private-network policy. The native resolver checks the concrete socket addresses before connecting and rejects nonpublic, loopback, private, link-local, multicast, documentation and transition ranges. The native agent disables environment proxy configuration to keep proxy credentials and alternative proxy resolution out of this credential-free boundary. Proxy-only networks can therefore fail these direct requests.

Task identifiers are not reused. Completion checks the app registration/enabled state, owning session, document epoch and response size, and delivers at most once. Closing/disabling the app, replacing phone data or dropping the task prevents late delivery. A native request already running on the system worker can continue until its bounded operation finishes; cancelling delivery does not interrupt its blocking HTTP call. An HTTP error remains an error and does not replace a saved forecast/feed with invented content.

Weather city search uses a 64 KiB bound and forecast reads use 128 KiB. The one-shot location/weather host operation retains its fixed-endpoint 64 KiB weather bound. Podcasts feed reads use the SDK's 2 MiB ceiling and parse a separately bounded feed model. The SDK boundary is reusable by other apps with the same declared permission and lifetime checks.

Audio/video streaming is a different provider operation. The initial source must pass the shared HTTPS URL admission rules, but the browser media element or FerroUI's native media provider owns network resolution, buffering, redirects and codecs. Streaming does not use the SDK downloader's response budget or native public-IP resolver, and it does not create an offline audio/video copy. Its success is reported from actual playback state. Streams that need unsupported codecs, unavailable network access or provider-rejected policies fail through the real player.

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

macOS notification/location adapters require the supplied `.app` identity and camera/location usage descriptions. Their Objective-C callbacks retain their own state through bounded waits and never retain pointers into a Rust stack frame. Windows location consent starts on the foreground UI dispatcher, and the request uses an explicit age/timeout bound. Unix acquisition runs separately from file work. Location failures never substitute default or IP-derived coordinates. The explicit `HostRequest::FetchWeather` one-shot operation uses a fixed Open-Meteo HTTPS endpoint with no redirects, a 15-second timeout and a 64 KiB response limit. The Weather app's city search and richer forecast reads use the separate SDK HTTPS limits described above.

Desktop foreground alarm audio uses a generated two-second PCM alert and the native media provider, separately from ordinary music/video. It respects the saved ringer volume and stops with the alert. The original ringtone catalog, native haptics and OS wake/scheduled background alarm delivery remain unimplemented.

The browser prefers its active service-worker notification registration and uses page notifications where supported. Clicking a notification can only focus/open this application's fixed URL. The worker cannot obtain an arbitrary navigation destination from notification data.

The browser Media Session adapter owns the current media element's listeners, metadata, playback/position state and play/pause/stop/next/previous/seek handlers. Switching players detaches previous listeners. Unsupported individual actions are tolerated. Playback generations prevent an older asynchronous source load from replacing a newer selection; Pause cancels a pending replacement.

The desktop owns native OS media sessions independently of its decoder:

| Platform | Registration and published state | Incoming commands and cleanup |
| --- | --- | --- |
| Windows | `ISystemMediaTransportControlsInterop::GetForWindow` binds SMTC to the app's actual HWND; music/video metadata, playback status, queue availability and timeline follow native observations | Button/position callbacks enqueue commands; teardown removes both event tokens, clears display metadata and disables the controller before releasing the WinRT apartment |
| macOS | The packaged application registers `MPRemoteCommandCenter` targets and updates `MPNowPlayingInfoCenter` metadata, elapsed time, duration and playback state on the main thread | Seven retained command targets feed a bounded native queue; weak callback captures avoid retaining closed sessions; teardown removes targets and clears now-playing state |
| Linux | A unique `org.mpris.MediaPlayer2.WindowsPhone.instance…` name exposes MPRIS 2 root/player interfaces on the session bus; metadata, state, seekability, volume and next/previous capabilities follow the active player | Play/pause/stop/next/previous/seek/volume and Raise are supported; `OpenUri` is rejected; dropping the owner closes command admission and releases the bus connection |

Native media capability starts unknown and becomes available only after real session registration succeeds. Missing services, window/bundle identity and API failures remain unavailable results. Linux registration is asynchronous and a disconnected bus is detected. Failed registration may be retried by a new playback request rather than by every timer tick. Platform UI ownership is enforced: the reusable session object cannot be moved to another thread.

Callbacks enqueue at most 32 source-generation-stamped commands and never capture FerroUI controls. The UI dispatcher rejects commands for replaced sources/documents, unsupported queue directions and invalid seek/volume values. Metadata text is bounded and control characters are removed; actual source paths and URLs are not exposed through OS source fields. MPRIS track identifiers are opaque generation paths, so stale `SetPosition` calls cannot seek a replacement track. Completed seek serials generate `Seeked`; ordinary progress does not. Replacing a source, deleting its final library reference, external handoff and window/document teardown clear the relevant native state. These adapters integrate the current desktop's media services; the original Windows Phone audio stack is not restored.

## Browser offline installation and updates

The deployment script precaches one build's application assets, including its matching JS and WASM, into a cache owned by the full service-worker registration scope and build version. Installation waits for the complete precache batch. A failed download leaves the currently active worker and its existing cache available. The new worker does not call `skipWaiting`, send takeover messages or force a page reload.

A successful update waits until every window controlled by the old worker has closed. Settings **phone update** explicitly checks for a newer worker and distinguishes unchanged, downloading and downloaded states; the downloaded notice asks the user to close all Windows Phone windows and reopen the app. First installation can claim the current page because no earlier worker owns it. Activation removes only older caches in the same installation scope before claiming clients. Other application caches, separate phone installations on the same origin and ambiguous legacy caches remain untouched. Saved phone data and media are not stored in these application-asset caches.

Transient registration failures clear the cached registration attempt, allowing retry or an explicit Settings check to recover. A successfully installed worker becoming redundant through supersession is not reported as a failed installation. An unfinished installation failure reports that the current app remains available. This is a browser offline-asset update mechanism; it does not install a new native executable or provide operating-system firmware updates.

## Interchange and validation

[People and Calendar interchange](pim-interchange.md) describes the exact supported vCard/iCalendar subset. Imports are local copies with fresh identifiers, not account synchronization. Unsupported semantic content is rejected rather than silently flattened. Browser file sharing prepares a complete file synchronously before invoking the host share sheet, preserving user activation; cancellation does not fall back to copying personal data.

Focused tests cover fallible restore commit order, no-write environment updates, unknown battery states, weighted Linux batteries, atomic file failure, worker wake/cancellation/isolation, native asset boundaries and backup codecs, file-name and chooser limits, validated coordinates/weather, HTTP URL/IP/size/lifetime admission, notification text handling, all eight camera orientation transforms, encoded JPEG output, and generated PCM structure. Media tests cover finite/bounded metadata and timelines, source/capability admission, stale command rejection and queue limits. Browser tests use simulated adapters for permission/storage/media APIs. Offline-update tests cover failed precache preservation, waiting-worker version consistency, scope-owned retirement, retry recovery and truthful update notices.

The opt-in `private_bus_media_controls` integration test discovers the real Linux MPRIS service, reads its properties, invokes transport/seek/volume methods, rejects stale track identifiers and arbitrary `OpenUri`, and verifies bus-name release after teardown. Run it only on a fresh test bus:

```sh
dbus-run-session -- env PHONE_MPRIS_TEST_SESSION=1 cargo test -p phone-host private_bus_media_controls -- --ignored
```

The [native CI matrix for `6518960`](https://github.com/FerroUI/WindowsPhone/actions/runs/45178205829) passed host tests and complete desktop build/link on Windows, macOS and Linux. This includes the WinRT SMTC implementation and the Objective-C MediaPlayer bridge; Linux additionally passed the isolated-bus MPRIS test. These results do not establish hardware key delivery, shell now-playing visibility or codec playback. Windows shell behavior and macOS command-center eligibility still require runtime/device testing with the packaged application. The full rendered suite exercises real Rust byte callbacks and native-style menus/settings; it does not open host permission dialogs, capture hardware, send notifications, play sound, send content to another app or prove physical-platform behavior. Revision-specific scope and counts are recorded in [integrated native verification](parity.md#integrated-native-verification-6518960).

## Primary API references

- [Notifications Standard](https://notifications.spec.whatwg.org/).
- [Storage Standard](https://storage.spec.whatwg.org/).
- [Media Session specification](https://www.w3.org/TR/mediasession/).
- [Service Workers specification](https://w3c.github.io/ServiceWorker/).
- [Fetch Standard](https://fetch.spec.whatwg.org/).
- [Windows power status](https://learn.microsoft.com/en-us/windows/win32/api/winbase/ns-winbase-system_power_status).
- [Windows file replacement](https://learn.microsoft.com/en-us/windows/win32/api/winbase/nf-winbase-movefileexw).
- [Windows notification state](https://learn.microsoft.com/en-us/windows/win32/api/shellapi/nf-shellapi-shqueryusernotificationstate).
- [Windows desktop location](https://learn.microsoft.com/en-us/windows/apps/develop/maps-and-location/get-location).
- [Windows per-window media transport controls](https://learn.microsoft.com/en-us/windows/win32/api/systemmediatransportcontrolsinterop/nf-systemmediatransportcontrolsinterop-isystemmediatransportcontrolsinterop-getforwindow).
- [Windows media transport state and timeline](https://learn.microsoft.com/en-us/windows/apps/develop/media-playback/system-media-transport-controls).
- [Apple UserNotifications](https://developer.apple.com/documentation/usernotifications/unusernotificationcenter).
- [Apple location authorization](https://developer.apple.com/documentation/corelocation/requesting-authorization-to-use-location-services).
- [Apple remote media commands](https://developer.apple.com/documentation/mediaplayer/mpremotecommandcenter).
- [Apple now-playing information](https://developer.apple.com/documentation/mediaplayer/mpnowplayinginfocenter).
- [MPRIS 2 player specification](https://specifications.freedesktop.org/mpris/latest/Player_Interface.html).
- [Freedesktop Notifications](https://specifications.freedesktop.org/notification/latest-single/).
- [GeoClue2 Client](https://www.freedesktop.org/software/geoclue/docs/gdbus-org.freedesktop.GeoClue2.Client.html).
- [Open-Meteo forecast API](https://open-meteo.com/en/docs).
- [Apple power-source snapshots](https://developer.apple.com/documentation/iokit/1523839-iopscopypowersourcesinfo).
- [Linux power-supply class](https://docs.kernel.org/power/power_supply_class.html).
- [Pinned FerroUI source](../FerroUI), including platform storage providers and shared controls.
