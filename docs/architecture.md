# Architecture

## One control tree across hosts

The application has six reusable Rust crates and a pinned FerroUI submodule. The browser is a real FerroUI application: its retained control tree, layout, input, styling and rendering are authored in Rust. HTML and JavaScript host the WebAssembly runtime and provide browser-only facilities; they are not a second implementation of the Start screen or core apps.

| Layer | Owns | Does not own |
| --- | --- | --- |
| `phone-domain` | Serializable data, identifiers, app catalog, tile sizes and packing, validation, pure calculator/calendar behavior. | Controls, browser objects or disk I/O. |
| `phone-state` | `PhoneViewModel`, `Action` dispatch, ReactiveUI notification, navigation history, recent-app state, DynamicData caches and persistence requests. | Widget construction or direct browser API calls. |
| `phone-controls` | Reusable templates, design tokens, palette, icons, typography and command controls. | App-specific data. |
| `phone-ui` | Shell/app composition, transient edit/selection/menu state, input arbitration and platform-surface markers. | Account credentials or network protocols. |
| Hosts | Runtime startup, serialization storage and `HostRequest` adapters. | A second authoritative phone document. |

The browser and desktop hosts construct a `PhoneViewModel` and the same `phone_ui::create_view`. The browser host currently provides the richer platform adapters.

## State and MVVM

`PhoneData` contains the durable document: settings, tiles, contacts, conversations, mail, appointments, notes, alarms, photos, media metadata, notifications, bookmarks, call history and the installed-app set. Its versioned JSON parser validates input before an import replaces existing data. Media bytes are separate from the serializable document.

`PhoneViewModel` contains a real FerroUI `ReactiveObject` and a `ReactiveCommand`. Actions pass through the command path and a reentrancy queue. A command updates the relevant collection, synchronizes projections and raises the revision property. Contacts, tiles and notifications use keyed DynamicData `SourceCache` projections.

The view subscribes to the ReactiveUI change stream and observes fine-grained FerroUI signals for transient UI state. A `Session` retains pivot positions, selected rows, edit fields, menus, open folders, lock/drawer state and media presentation. A route change clears transient UI that belongs to the previous view; saved app pivots remain available when returning.

Persistence occurs for document mutations. Navigating between screens and updating the clock do not serialize the document. Host callbacks are invoked after releasing the document borrow so a platform callback can safely inspect state.

## Controls and theme

`phone-controls` installs a **trimmed FerroUI WinUI theme** and an additional Metro palette/style group. The build script declares the needed control roots instead of retaining every theme template. A palette change replaces one bounded style group; an unchanged palette is a no-op.

Reusable `ControlTheme` / `FuncControlTemplate` definitions cover flat buttons, tiles, application-bar actions, toggle buttons and list containers. Their `ContentPresenter` binds content, brushes, padding, borders, typography and alignment using actual `TemplateBinding` objects. Input, focus, automation and command behavior stay on the native FerroUI control classes. Text boxes retain the WinUI editing template with square Windows Phone colors and borders.

The icon catalog consists of cached vector geometries. Raster photographs use lazy host loading and are rendered by FerroUI. Camera Roll uses a native virtualized `ListBox` of three-photo rows and declarative recycling item templates. Source and template identities survive unrelated clock/menu renders, and the decoded working set follows the realized viewport.

Images have separate 320-pixel thumbnails and 1,200-pixel full-view variants. A header parser validates encoded length and dimensions before calling a native codec; source images are limited to 16 megapixels and 16 MiB of encoded data. The decoded cache has a 24 MiB soft target and evicts the least recently used idle entry. Active native controls and mounted image leases retain their sources, even if the visible working set exceeds that target. Completion resolves all mounted leases before scheduling a render, preventing concurrent loads from evicting one another into a reload loop. Requests are deduplicated by source, failed loads are suppressed until registration/reset, deleted assets reject late completions, and backup restore clears caches and host request generations.

Typography requests Segoe UI and falls back to bundled Inter. This is a practical portability choice, not a claim of original Segoe WP metrics. The visual system uses square surfaces, large lightweight headings, monochrome actions and bottom command bars. Exact original metric tuning and device comparisons remain tracked in [parity](parity.md).

## Tiles, navigation and gestures

The domain layer packs tile spans on a four- or six-cell grid. Small, medium and wide tiles occupy deterministic spans; stable identifiers separate layout from app identity. Folders retain complete child tiles, including their identifiers, sizes and content targets. Secondary tiles can open saved content such as browser favorites through the same navigation/host-request path. Settings and tile operations are persisted, while drag ownership and transient edit affordances remain UI state.

The navigation journal separates Start, the app list, apps and detail routes. Recent apps retain destinations for switching. Overlays such as Action Center, menus and the app switcher close before ordinary Back navigation. Host browser history participates through the browser bridge.

The task switcher keeps up to six immutable native page snapshots, each limited to 270 × 560 pixels. A snapshot is taken on the UI thread before the old page is reconciled away, only when its layout is valid and the page is unobscured. The capture cache does not publish reactive notifications or persist image data. Dropping a cache entry releases its reference while a mounted card can finish using its image. Browser-native iframe, camera and video content remains outside the captured FerroUI tree.

Gesture handling has one controller for a pointer sequence. It arbitrates unclaimed native scrolling, shell navigation, top-edge Action Center gestures, lock dismissal and held-tile dragging. Text editors keep their own editing input. Cancelled capture, detachment, navigation and keyboard cancellation release transient gesture ownership. The pure motion classifier can be checked independently from rendering.

This structure avoids separate touch and mouse implementations mutating the same tile model concurrently. It does not yet reproduce every original inertia, tilt, connected transition or multipointer gesture; those are explicit fidelity gaps.

## Host service contract

Views request platform behavior through the `HostRequest` enum. The browser adapter implements the following families:

| Request family | Browser adapter |
| --- | --- |
| Navigation | Validated HTTP(S) URLs, sandboxed iframe surface, external browser-tab links. |
| Phone/message/mail | `tel:`, `sms:` and `mailto:` handoff. The destination app owns sending or calling. |
| Photos/camera | File picker, normalized imported images, camera permission and live preview/capture. |
| Audio/video | IndexedDB media files, HTML media playback, volume, play/pause/stop and media-session actions. |
| Location/weather | Explicit geolocation request; OpenStreetMap handoff or current Open-Meteo conditions. |
| Share | Web Share, file sharing where supported, or clipboard fallback. |
| Data | Phone JSON and media backup import/export. |
| Notifications/alarms | In-app notices, optional browser notifications and foreground alarm checks. |

Named FerroUI controls (`BrowserViewport`, `CameraViewport`, `MediaViewport`) mark where browser-native surfaces belong. The host observes their current bounds and visibility and positions the iframe/video elements accordingly. These are platform content surfaces, not alternate DOM versions of app controls. Leaving the camera view releases its tracks.

The desktop host retains the shared core UI and local document persistence/recovery, provides URI handoff to installed applications, and exports document backups. It reports unavailable camera, imported-media, location, sharing and backup-restore requests. Browser backups include media; desktop document backups do not contain IndexedDB assets from a browser profile.

## Storage and recovery

The browser stores the phone document in local storage and binary media in a versioned IndexedDB object store. A cross-tab check rejects a write if another tab has changed the stored document. Storage errors are surfaced instead of being reported as successful saves. Recovery data is preserved where storage capacity permits.

A backup wraps the versioned phone document together with its media manifest and encoded bytes. Import validates document data before applying it and checks the media manifest. The current browser import limit is 256 MB for the backup envelope; the domain document has its own 16 MB limit. Large media operations can require substantial transient memory because JSON/base64 representations and decoded assets coexist.

Imported images are oriented and reduced to a maximum dimension of 1,600 pixels before being stored as JPEG. They are application copies, not a lossless archive of the original selected files. Audio and video formats remain subject to the browser's codecs. Export backups if data must survive clearing browser storage or moving to another device.

## Build and deployment

The workspace imports FerroUI's dependency pins and build profiles through its synchronization script. Browser compilation uses the framework's Emscripten target and exact `wasm-bindgen` version. The browser build wrapper rejects app filenames reserved by the runtime, packages the runtime with app assets, prepares the static site, and statically links its JavaScript module graph without executing browser code.

One GitHub Actions workflow performs focused state/host checks, builds the browser, verifies its first rendered native frame, and deploys the Pages artifact. Longer interaction and screenshot checks are opt-in through `workflow_dispatch.verify_ui` or a `[verify-ui]` commit message; their screenshots are retained as a separate verification artifact. Documentation-only changes skip compilation. Toolchain and Cargo caches keep subsequent builds smaller. A deployed artifact comes from the same pinned framework and application source revision.

The Cargo cache retains the full target directory, including FerroUI's local path dependencies. Its compatibility prefix includes the pinned framework, Rust version, lockfile, manifests and build configuration; a source-commit suffix allows a successful build to refresh it. Saving occurs before browser interaction checks. Clean tracked files and watched asset directories in the pinned framework receive stable commit timestamps, so a fresh checkout does not make unchanged framework inputs newer than cached Cargo fingerprints. Application source timestamps are left alone, and a modified framework checkout is rejected by the normalization helper.

An optional manual `reuse_run` avoids compilation when only tests or documentation have changed. Before selecting an immutable artifact ID, the workflow verifies a completed main-branch Pages run from the same repository and compares its runtime input trees with the current commit. It then repeats host checks, module linking and the full browser interaction suite. A runtime-source difference fails the reuse path, and publication still requires successful browser verification.

Compilation, domain tests and smoke interactions establish different things. Compilation checks API/type compatibility; domain tests check state invariants; browser smoke checks detect startup and selected interaction failures. None alone establishes complete Windows Phone parity or hardware qualification.
