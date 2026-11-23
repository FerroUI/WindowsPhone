# Host integration

The phone's native FerroUI views and view model are shared. Platform adapters implement the host operations that a normal application can perform. These adapters do not confer Windows Phone kernel privileges, cellular transport, restored Microsoft services, or background alarm scheduling.

## Reusable boundary

`phone-host` is independent of FerroUI and `phone-domain`. It owns the serialized `HostRequest`, `HostEnvironment`, `HostPlatform` and `HostPermission` contracts; validated URI construction; and native primitives for application data directories, atomic file replacement, default-handler activation, power observations and bounded background work. `phone-state` re-exports the contracts so app controls continue to request services through the view model.

The UI still uses declarative RSX, shared phone control templates and the existing WinUI-derived theme. Observations update the ReactiveUI revision without modifying the saved phone document. DynamicData projections remain responsible for stable collection identity. Host power-saving observations pause decorative motion and live-tile cycling without changing the user's saved Battery Saver preference.

| Facility | Browser host | Windows/macOS/Linux desktop host |
| --- | --- | --- |
| App UI and local data | Shared Rust/RSX controls and view model | Same controls and view model |
| Contacts | Bounded vCard import, file download, file-capable Web Share | Same codec with FerroUI Open/Save pickers; sharing saves an attachment copy |
| Calendar | Bounded iCalendar import, file download, file-capable Web Share | Same codec with native pickers; sharing saves an attachment copy |
| Office | Shared OOXML/TXT/CSV import; OOXML download and file-capable sharing | Same document codec through native pickers; multi-file system sharing remains unavailable |
| Photos | Picker imports normalized JPEG copies; camera capture where permitted | Picker imports app-owned image copies; shared image cache loads native bytes |
| Music/video | HTML media player, playback controls, Media Session integration | App-owned imports and default media-app handoff; playback belongs to the external player |
| Share photo/media | Browser sharing support determines available targets | Save the actual asset bytes through a native picker, then attach in another app |
| Backup | Portable JSON/media envelope; staged immutable assets before document commit | Compatible envelope; native pickers, fresh assets, recovery snapshot and atomic document replacement |
| Battery | Optional Battery Status observations | Linux sysfs, Windows `GetSystemPowerStatus`, macOS IOKit |
| Device notifications | Explicit permission request; permitted foreground Calendar/alarm banners use worker/page notification APIs | Native in-app alerts; system notification delivery and alarm audio are not connected |
| Storage | Origin usage/quota estimates, observed persistence, explicit persistence request | Application data files; no browser quota/persistence prompt |
| Calling/mail/text | Validated URI handoff; the destination app owns the operation | Validated default-handler activation with the same ownership boundary |
| Camera/location | Explicit browser permission and supported APIs | Live camera and location adapters remain unavailable |

Windows/macOS/Linux adapter source is present. The recorded Linux compilation and unit tests do not demonstrate actual picker, battery, codec or default-handler behavior on all three operating systems. Android and iOS currently use the browser host; dedicated native mobile launch targets have not been added.

## File and media lifetime

The desktop uses FerroUI's `IStorageProvider` rather than platform-specific picker widgets. `picker_io` bounds reads and yields between 256 KiB chunks. Pure asset and backup work runs through a reusable single worker with a bounded 32-job queue. A task's standard Rust `Future` wakes its caller's executor; UI controls and image decoding remain on the UI dispatcher. Cancelled queued work is skipped.

Native media keys are identifiers, not paths. The asset store accepts only `media:` followed by 1–128 ASCII letters, digits, underscores or hyphens, beginning with a letter/digit. App-owned files use those keys and admitted media extensions. Manifests and payload reads are bounded; symlink payloads are rejected. Selected source files are copied into app storage. Deleting an imported asset does not delete the user's original selected file.

The browser stores media in IndexedDB. The shared image cache still admits bounded headers/dimensions before native decoding and keeps separate thumbnail/full-view variants. Browser imports normalize images to oriented JPEG copies; desktop imports preserve admitted encoded image bytes and render through the same bounded cache. These are different import paths and should not be described as lossless, identical image processing.

Desktop external playback reports a launch request and leaves in-app playback marked inactive. The external media app owns codecs, transport controls, volume and its playback lifetime. The phone cannot confirm delivery to an arbitrary default handler.

## Restore transaction

Both hosts validate the supported phone version, document and manifest before replacing live data. The portable envelope limits are 256 MiB encoded JSON, 16 MiB phone JSON, 10,000 media entries, 16 MiB per image, 128 MiB per audio/video item, and 192 MiB aggregate decoded media. Native format admission may reject a codec/extension that a browser can preserve. JSON/base64 processing can hold multiple representations in memory, so these limits are upper bounds rather than performance promises.

Each supplied media item receives a fresh immutable key. Photo, track, Start-wallpaper and independent lock-wallpaper references are remapped together. Existing assets are never overwritten by matching identifiers from a backup. Missing assets are reported.

`PhoneViewModel::replace_data_after_save` validates the candidate and calls a fallible host save operation before replacing the document, navigation, projections or document epoch. Browser quota/concurrent-tab errors and desktop disk errors therefore leave the current view model intact. A successful transaction does not repeat the save through the ordinary callback. Failed commits remove staged new assets; post-commit image cleanup never removes committed assets.

The desktop preserves the latest pre-restore phone document beside the active data file immediately before replacement. Its old immutable media remain available to that recovery document. Application-owned document writes use a same-directory `create_new` staging file, flush its bytes, and replace the target. Windows replacement uses `MoveFileExW`; Unix uses rename. Picker-selected exports retain the host provider's own write semantics.

Application data uses `%LOCALAPPDATA%` on Windows, `~/Library/Application Support` on macOS, and `$XDG_DATA_HOME` or `~/.local/share` on Linux. The former data location is copied forward when appropriate; migration failures preserve the previous location. Backups use the actual active path, including recovery-mode locations.

## Notifications, media keys and permission ownership

Reading `HostEnvironment` does not request permissions. The Settings commands `RequestNotificationPermission` and `RequestPersistentStorage` are explicit user actions. Unsupported, unknown, prompt, granted and denied notification states remain distinguishable.

A foreground Calendar/alarm alert issues a device banner only when it becomes the active alert, global notifications and that app's banners are enabled, host permission is granted, and in-app lock privacy allows it. The host banner is silent because the existing foreground alert adapter owns sound/vibration. No background wake-up or delivery while the app is closed is implemented.

The browser prefers its active service-worker notification registration and uses page notifications where supported. Clicking a notification can only focus/open this application's fixed URL. The worker cannot obtain an arbitrary navigation destination from notification data.

The Media Session adapter owns the current media element's listeners, metadata, playback/position state and play/pause/stop/next/previous/seek handlers. Switching players detaches previous listeners. Unsupported individual actions are tolerated. Playback generations prevent an older asynchronous source load from replacing a newer selection; Pause cancels a pending replacement. These are browser host integrations, not Windows Phone's original audio stack.

## Interchange and validation

[People and Calendar interchange](pim-interchange.md) describes the exact supported vCard/iCalendar subset. Imports are local copies with fresh identifiers, not account synchronization. Unsupported semantic content is rejected rather than silently flattened. Browser file sharing prepares a complete file synchronously before invoking the host share sheet, preserving user activation; cancellation does not fall back to copying personal data.

Focused tests cover fallible restore commit order, no-write environment updates, unknown battery states, weighted Linux batteries, atomic file failure, worker wake/cancellation, native asset boundaries and backup codecs. Browser tests use simulated adapters for permission/storage/media APIs. The full rendered suite additionally exercises real Rust byte callbacks and native menus/settings; it does not open host permission dialogs, send content to another app or prove hardware behavior.

## Primary API references

- [Notifications Standard](https://notifications.spec.whatwg.org/).
- [Storage Standard](https://storage.spec.whatwg.org/).
- [Media Session specification](https://www.w3.org/TR/mediasession/).
- [Windows power status](https://learn.microsoft.com/en-us/windows/win32/api/winbase/ns-winbase-system_power_status).
- [Windows file replacement](https://learn.microsoft.com/en-us/windows/win32/api/winbase/nf-winbase-movefileexw).
- [Apple power-source snapshots](https://developer.apple.com/documentation/iokit/1523839-iopscopypowersourcesinfo).
- [Linux power-supply class](https://docs.kernel.org/power/power_supply_class.html).
- [Pinned FerroUI source](../FerroUI), including platform storage providers and shared controls.
