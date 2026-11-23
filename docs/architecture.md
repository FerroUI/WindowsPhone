# Architecture

## One control tree across hosts

The application has eight Rust crates with reusable domain, host, state and UI layers and a pinned FerroUI submodule. The browser is a real FerroUI application: its retained control tree, layout, input, styling and rendering are authored in Rust. HTML and JavaScript host the WebAssembly runtime and provide browser-only facilities; they are not a second implementation of the Start screen or core apps.

| Layer | Owns | Does not own |
| --- | --- | --- |
| `phone-host` | Host request/environment contracts, URI validation, native paths, atomic files, background tasks and power observations. | UI controls or application state. |
| `phone-domain` | Serializable data, identifiers, app catalog, tile sizes and packing, validation, pure calculator/calendar behavior. | Controls, browser objects or disk I/O. |
| `phone-state` | `PhoneViewModel`, `Action` dispatch, ReactiveUI notification, navigation history, recent-app state, DynamicData caches and persistence requests. | Widget construction or direct browser API calls. |
| `phone-controls` | Reusable templates, design tokens, palette, icons, typography and command controls. | App-specific data. |
| `phone-fonts` | Licensed embedded Selawik assets and shared `EmbeddedFontCollection` registration. | Theme selection or app data. |
| `phone-ui` | Shell/app composition, transient edit/selection/menu state, input arbitration and platform-surface markers. | Account credentials or network protocols. |
| Hosts | Runtime startup, serialization storage and `HostRequest` adapters. | A second authoritative phone document. |

The browser and desktop hosts construct a `PhoneViewModel` and the same `phone_ui::create_view`. The browser host currently provides the richer platform adapters.

## State and MVVM

`PhoneData` contains the durable document: settings, tiles, contacts, conversations, mail, appointments, notes, alarms, photos, media metadata, notifications, bookmarks, call history, Office documents/folders, browser tabs/history and the installed-app set. Its versioned JSON parser validates input before an import replaces existing data. Media bytes are separate from the serializable document.

`PhoneViewModel` contains a real FerroUI `ReactiveObject` and a `ReactiveCommand`. Actions pass through the command path and a reentrancy queue. A command updates the relevant collection, synchronizes projections and raises the revision property. Contacts, tiles and notifications use keyed DynamicData `SourceCache` projections.

The view subscribes to the ReactiveUI change stream and observes fine-grained FerroUI signals for transient UI state. A `Session` retains pivot positions, selected rows, edit fields, menus, open folders, lock/drawer state and media presentation. A route change clears transient UI that belongs to the previous view; saved app pivots remain available when returning.

Persistence occurs for document mutations. Navigating between screens and updating the clock do not serialize the document. Host callbacks are invoked after releasing the document borrow so a platform callback can safely inspect state. Whole-document replacement increments an ephemeral document epoch: the shared UI then cancels input, clears previews and invalidates drafts, selections and transfer identifiers from the previous dataset. Visible system overlays take Back priority over app-local state. Synchronous Office imports reject command reentrancy before queuing anything and return the identifier only after the document is committed.

### System preferences and hardware state

Additional `Settings` fields use Serde defaults so older phone documents retain their original data. Normalization bounds the two volume scales, validates the lock timeout and status providers, removes duplicate quick-status providers, and rejects unknown app notification keys. Date display order and the first calendar weekday are preferences; stored appointment dates remain ISO dates. The supported regional presets do not change the English interface language.

`HardwareState` owns only transient volume-panel signals, an expiry deadline and a held-pointer flag. Pressing a volume button selects the active sound channel; the expanded panel exposes independent ringer/notification and media/app sliders. Mute retains the stored level. Slider callbacks distinguish a user change from the value being reconciled into a control, preventing mounting a muted slider from overwriting its saved volume or unmuting it. The panel remains open while a pointer is interacting with it, and closes after inactivity. Durable levels and mute preferences stay in `Settings`.

`HostEnvironment` contains platform and permission states plus optional battery, charging, external-power, power-saving, online and browser-origin storage observations. Updating those values notifies the view without saving the phone document; unavailable readings remain unknown. Battery Saver affects this app's decorative motion and live-tile cycling. Storage Sense and Data Sense report measured JSON document sizes; imported media bytes, host disk capacity, per-app power consumption and carrier traffic are outside that measurement.

The lock view reads one detailed-status provider and five unique quick-status choices from settings. Action Center groups notifications by app. Its ingestion and visible list both respect the global notification switch and the app's Action Center preference. Hiding an existing group preserves its stored items until a read or explicit dismissal removes them. Per-app banner, sound and vibration flags are persisted preferences. Foreground Calendar/alarm device banners respect global/app/lock policies and granted host permission; independent host sound/vibration channels remain incomplete. Global foreground-alarm behavior is separate from Action Center filtering.

Locking hides and disables the underlying app page and suppresses app dialogs, status banners and the switcher. The session retains form fields and callbacks, so unlocking restores an unsaved form. The power key closes the transient hardware and Action Center panels without discarding that draft. The lock screen can still present its own permitted Action Center and volume controls.

### Calendar and foreground alerts

Calendar dates and start times use canonical local wall-clock strings. New all-day, recurrence, exception, reminder and availability fields have Serde defaults, so old appointments remain nonrepeating entries. Pure calendar modules validate years 1–9999, expand an interval without replaying a series from its first date, and find the next ongoing/upcoming occurrence. Monthly and yearly rules skip impossible dates. Queries are bounded to 366 days and 4,096 returned occurrences; the document allows 4,096 events/series and 512 excluded dates per series. These are local implementation limits, not the complete Exchange recurrence or time-zone model.

`CalendarAction::SaveOccurrence` validates an exception and its standalone replacement before allocating an identifier or publishing either change. Deleting one occurrence adds an exclusion to the series; whole-series edits use the ordinary validated save command. A successful mutation persists once. Calendar's day/week/month/year views, detail page, choice pages and editor live in small `rsx!` modules. `CalendarSession` holds draft signals, current view/date, selected occurrence and one remembered day-scroll offset. Native `DatePicker` and `TimePicker` selected values bind two ways to draft signals; save snapshots and validates them. Cancel and Back discard transient editing without changing the document.

`AlarmSchedule` and `ReminderSchedule` are pure Rust foreground schedulers shared by both hosts. They track delivered occurrences and pending snoozes without serializing ticking state. Alarm weekdays and Calendar occurrences use local time; snooze deadlines use the absolute host clock. A one-time alarm is disabled after its first delivery. Series exclusions, deletion or disabling a reminder invalidate pending delivery. A separate schedule revision rechecks same-minute edits without resetting delivered/snoozed state; a due snoozed reminder refreshes its subject, location and end time from the current event. The shared native alert queue covers underlying app input while retaining unsaved forms; dismissal or snooze reveals the same draft. Back snoozes an alarm for two minutes. Opening a Calendar alert navigates first, then selects the actual occurrence so route cleanup cannot discard the target.

Only `HostRequest::AlarmAlert` crosses the alert/audio boundary. The browser keeps one reusable Web Audio context and one active-alert pulse timer, scales its synthetic tone from the ringer channel, and uses supported vibration. It stops audible voices while hidden and on dismissal. Browser activation/audio restrictions still apply. Desktop ticks and alert controls work, but its sound/vibration adapter is absent. Missed scheduled alarm or appointment occurrences are skipped; pending snoozes become due on the next running tick even if their deadline passed during a background interval. Snoozes and delivered-occurrence history are transient and clear on reload. The schedulers cannot wake a suspended or closed app. The demonstration alarm starts disabled.

### Communication workflows

Mail and Messaging have small view modules with their own transient selection/search/context state. Their selection rails, held-row context actions and application bars dispatch domain commands. `MailBatch` and `ConversationBatch` deduplicate and validate the full identifier selection before mutating anything, update related notification projections, and persist once when data actually changes. Repeating an unchanged operation does not save again. Mail deletion moves an item to the local deleted folder; a second deletion is permanent, and restore returns it to the inbox.

Message bubbles expose copy, forward and delete commands. Forwarding opens a prefilled compose form; copy uses a host request and retains the host's clipboard permission boundary. A conversation's unknown phone number can create a local contact and link it back to that thread. Unsent text-entry drafts live in session state; a message handed to `sms:` is recorded as a draft because the host cannot confirm delivery. Email composition uses `mailto:` without inventing a synchronized sent mailbox. Compose forms label their primary action `open message` or `open email` to make that boundary visible.

## Controls and theme

`phone-controls` installs a **trimmed FerroUI WinUI theme** and an additional Metro palette/style group. The build script declares the needed control roots instead of retaining every theme template. A palette change replaces one bounded style group; an unchanged palette is a no-op.

Reusable `ControlTheme` / `FuncControlTemplate` definitions cover flat buttons, tiles, application-bar actions, toggle buttons and list containers. Their `ContentPresenter` binds content, brushes, padding, borders, typography and alignment using actual `TemplateBinding` objects. Input, focus, automation and command behavior stay on the native FerroUI control classes. Text boxes retain the WinUI editing template with square Windows Phone colors and borders.

The trimmed roots also include `DatePicker`, `TimePicker` and their presenters. The `phone-picker` class applies square borders, palette colors and phone typography while preserving native selector panels, keyboard handling, accept/cancel and binding behavior. These remain styled WinUI popup presenters; the original phone's full-screen picker composition and transition are separate fidelity work.

Picker keyboard focus is an owned control behavior attached once through RSX `with`. Native presenter initialization can run before its visual tree is attached, so one dispatcher operation after popup opening transfers focus to an eligible native selector. Date order follows the visible template columns; time starts in the hour selector. Existing focus inside the popup is preserved. Closing, retemplating, unloading or disposing the control invalidates pending work; weak references and disposed popup subscriptions prevent stale focus requests or accumulated handlers. This adds no frame polling or persistent-state writes.

The phone slider supplies square rails/thumbs in cached horizontal and vertical templates, retaining native `PART_Track`, decrease/increase buttons, `Thumb`, range bindings and automation. The visible thumb has a larger transparent input area. This replaces presentation without duplicating range arithmetic or pointer handling.

Page entry, app-bar/folder reveal and live-tile replacement use finite native transform/opacity animations. Their completion and unload paths release subscriptions and restore owned transforms; no perpetual per-tile animation timer runs. Pressed transitions are shared. Reduced motion and Battery Saver disable decorative motion through the common palette/motion gate.

Tile press feedback projects the content through a native `Rotate3DTransform` and scale, leaving the Button's input rectangle and command behavior intact. Press position determines the bounded angle; release returns it over 120 ms, while hold, cancel and unload reset transient ownership. The current 6-degree maximum and timing are tuning values, not measurements of original devices.

Shared actionable rows assign an explicit automation name from their title and help text from their subtitle. Native focus and editing behavior remain on the control, and text-box template foregrounds are styled through focus, hover and disabled states.

The icon catalog consists of cached vector geometries. Raster photographs use lazy host loading and are rendered by FerroUI. Camera Roll uses a native virtualized `ListBox` of three-photo rows and declarative recycling item templates. Source and template identities survive unrelated clock/menu renders, and the decoded working set follows the realized viewport.

The Start and lock wallpaper pickers reuse a bounded native `ListBox` with a stable items source and recycling template. They request the thumbnail image variant rather than eagerly decoding every full-sized photograph. Both selection flows save only the chosen source identifier.

Images have separate 320-pixel thumbnails and 1,200-pixel full-view variants. A header parser validates encoded length and dimensions before calling a native codec; source images are limited to 16 megapixels and 16 MiB of encoded data. The decoded cache has a 24 MiB soft target and evicts the least recently used idle entry. Active native controls and mounted image leases retain their sources, even if the visible working set exceeds that target. Completion resolves all mounted leases before scheduling a render, preventing concurrent loads from evicting one another into a reload loop. Requests are deduplicated by source, failed loads are suppressed until registration/reset, deleted assets reject late completions, and backup restore clears caches and host request generations.

Typography requests Segoe WP, then Segoe UI, the bundled Microsoft Selawik collection and Inter. `PhoneFontBuilderExt::with_phone_font()` registers the same four unmodified Selawik 1.01 weights in both hosts. Their SIL license and release-archive hash are recorded in `phone-fonts/NOTICE.md`; the browser distribution includes the license text. Selawik is an open Segoe UI replacement with documented upstream kerning/hinting differences, so this does not assert original Segoe WP metrics. The visual system uses square surfaces, large lightweight headings, monochrome actions and bottom command bars. Exact original metric tuning and device comparisons remain tracked in [parity](parity.md).

## Tiles, navigation and gestures

The domain layer packs tile spans on a four- or six-cell grid. Small, medium and wide tiles occupy deterministic spans; stable identifiers separate layout from app identity. Folders retain complete child tiles, including their identifiers, sizes and content targets. Secondary tiles can open saved content such as browser favorites through the same navigation/host-request path. Settings and tile operations are persisted, while drag ownership and transient edit affordances remain UI state.

Live-tile cadence derives from one transient session signal, coalesced into four-second clock buckets while Start or a folder is visible and unobscured. A nested reactive scope confines cadence invalidation to the Start subtree. App pages, overlays, editing, disabled live tiles, Battery Saver and reduced motion pause that signal; returning to Start or replacing the phone document synchronizes its timestamp. No domain action, persistence write or per-tile timer is involved. Photos stagger their frames; a People cadence changes one cell rather than rebuilding the mosaic. Keyed child content preserves the other mounted controls between ticks, and folder previews reuse real child presentation. Document snippets consume only a bounded prefix. Holding a tile exposes temporary drop feedback after a dwell; empty Start/folder areas provide extraction and append destinations without persisting intermediate pointer positions.

Transparent tiles and expanded-folder children sample one viewport-sized image plane from their actual layout positions. A shared native translation responds to the ScrollViewer offset without rebuilding the tile tree or writing state. Travel is capped at one quarter of the viewport, avoiding the old fixed-height crop on long Start layouts. Reduced motion and Battery Saver keep the image stationary on screen; branded opaque tiles retain their own backgrounds. The parallax constant still needs original-device comparison.

The app list records native group-header offsets after layout for letter jumps. A sticky letter uses the next header's position for push-off; scrolling updates a native transform and changes a reactive signal only when the active letter changes. Search removes grouping and resets scroll. Alphabet cells derive their square size from the phone width and use palette-aware inactive colors. The existing always-available letter jump differs from the original installed-app threshold.

The navigation journal separates Start, the app list, apps and detail routes. Recent apps retain destinations for switching. Overlays such as Action Center, menus and the app switcher close before ordinary Back navigation. Host browser history participates through the browser bridge.

The task switcher keeps up to six immutable native page snapshots, each limited to 270 × 560 pixels. A snapshot is taken on the UI thread before the old page is reconciled away, only when its layout is valid and the page is unobscured. The capture cache does not publish reactive notifications or persist image data. Dropping a cache entry releases its reference while a mounted card can finish using its image. Browser-native iframe, camera and video content remains outside the captured FerroUI tree.

Gesture handling has one controller for a pointer sequence. It arbitrates unclaimed native scrolling, shell navigation, top-edge Action Center gestures, lock dismissal and held-tile dragging. Text editors keep their own editing input. Cancelled capture, detachment, navigation and keyboard cancellation release transient gesture ownership. The pure motion classifier can be checked independently from rendering.

Partial Action Center pulls expose quick actions; a further pull expands it. A one-finger horizontal gesture dismisses the app group. Two-finger clear-all requires both contacts to complete horizontal strokes in the same direction; a stationary or opposing contact does not clear everything. Native visual translation supplies reversible drag feedback without modifying durable state. Document routes stay outside Office hub pivot navigation, preserving native worksheet horizontal scrolling.

This structure avoids separate touch and mouse implementations mutating the same tile model concurrently. It does not yet reproduce every original inertia, tilt, connected transition or multipointer gesture; those are explicit fidelity gaps.

## Browser state

`BrowserData` owns stable tab identifiers, per-tab address-entry journals, visit frequency/history and the address-bar shortcut. Actions validate and normalize HTTP(S) addresses before changing state. The model bounds tabs to 32, journal entries to 128 per tab and distinct history URLs to 512. Back followed by a new address truncates only that tab's forward branch. Closing the last tab leaves a blank one.

Native collections, chrome and navigation modules compose the tab list, frequent/history/favorites views and compact commands. Host acknowledgements record only app-initiated navigation and avoid duplicating explicit Back/Forward entries. Cross-origin iframe-internal navigation and page titles remain inaccessible. Opening a blank tab or closing the final page issues `ClearBrowser` to stop the previous host document; temporarily showing native tabs/history merely hides the surface. Reattaching a retained browser page after resize preserves its collection and menu state.

## Host service contract

Views request platform behavior through the `phone-host::HostRequest` enum. [Host integration](host-integration.md) records the platform matrix, ownership, restore transactions and verification limits. The browser adapter implements the following families:

| Request family | Browser adapter |
| --- | --- |
| Navigation | Validated HTTP(S) URLs, sandboxed iframe surface, external browser-tab links. |
| Phone/message/mail | `tel:`, `sms:` and `mailto:` handoff. The destination app owns sending or calling. |
| Photos/camera | File picker, normalized imported images, camera permission and live preview/capture. |
| Audio/video | IndexedDB media files, HTML media playback, volume, play/pause/stop and media-session actions. |
| Location/weather | Explicit geolocation request; OpenStreetMap handoff or current Open-Meteo conditions. |
| Share | Text/URL Web Share with clipboard fallback. Office document sharing requires file-capable Web Share; unsupported browsers receive an error suggesting export. |
| Data | Validated phone/media backup with immutable-key staging and fallible save before live-state replacement. |
| Contacts/calendar | Shared bounded vCard/iCalendar codecs; file import, downloads and file-capable sharing. |
| Environment | Battery/online/notification permission and origin storage estimates; explicit permission/persistence commands. |
| Office documents | Native file selection, bounded DOCX/XLSX/PPTX/TXT/CSV import into the shared Rust model, and exported document downloads. |
| Clipboard | User-requested text copying from native app commands. |
| Notifications/alarms | In-app notices and optional browser notifications; shared Rust alarm/reminder timing; synthetic foreground alert sound/vibration through a small host adapter. |

Named FerroUI controls (`BrowserViewport`, `CameraViewport`, `MediaViewport`) mark where browser-native surfaces belong. The host observes their current bounds and visibility and positions the iframe/video elements accordingly. These are platform content surfaces, not alternate DOM versions of app controls. Leaving the camera view releases its tracks.

Before creating the runtime, the browser host installs a resize-entry adapter for `canvas.ferroui-canvas` elements inside `#out`. It corrects a device-pixel content box only when it differs by more than one pixel from the CSS content size multiplied by the advertised device-pixel ratio. Normal rounding, unrelated entries, callback context and the observer instance are preserved. This keeps the canvas backing size, native layout and input coordinates consistent when viewport emulation reports the physical display's density instead. The startup splash becomes hidden after its opacity fade so its loading status leaves the accessibility tree.

The desktop host uses native FerroUI storage-provider pickers for Office, contacts, calendar files, photos, music, video and compatible media backups. App-owned media loads through the shared image cache; audio/video playback is handed to the default media app. Sharing a contact, appointment or media asset saves an attachment copy. URI handoff and clipboard remain native. Windows/macOS/Linux power adapters feed transient observations. Restore preserves a recovery snapshot and saves the candidate before changing live data. Live camera, location, system notification delivery, embedded playback and alarm sound/vibration remain unavailable on the desktop host.

## Storage and recovery

The browser stores the phone document in local storage and binary media in a versioned IndexedDB object store. A cross-tab check rejects a write if another tab has changed the stored document. Storage errors are surfaced instead of being reported as successful saves. Recovery data is preserved where storage capacity permits.

A backup wraps the versioned phone document together with its media manifest and encoded bytes. Import validates document data before applying it and checks the media manifest. The current browser import limit is 256 MB for the backup envelope; the domain document has its own 16 MB limit. Large media operations can require substantial transient memory because JSON/base64 representations and decoded assets coexist.

Browser-imported images are oriented and reduced to a maximum dimension of 1,600 pixels before being stored as JPEG; desktop imports keep admitted encoded bytes in app-owned storage. They are application copies, not a lossless archive of the original selected files. Audio and video formats remain subject to the browser's codecs. Export backups if data must survive clearing browser storage or moving to another device.

The offline shell cache is versioned by build. Precache requests use `cache: "reload"` to bypass stale HTTP-cache entries, and a new service worker calls `skipWaiting()` only after the complete packaged file list has been cached successfully. Activation removes older Windows Phone shell caches and claims existing clients. It does not force a page reload: an open page keeps its loaded runtime and current editing session. Shell-cache cleanup does not clear phone data in local storage or media in IndexedDB.

Fetch handling looks only in the active build's cache. Canonical directory navigation falls back to that cache's `index.html`; uncached same-origin GETs use the network, while external and non-GET requests are left to the browser. The browser suite verifies an offline canonical-directory reload, a rendered Start screen and native Search/Back input. Installation through the browser UI, interrupted updates and multi-client upgrade recovery require separate verification.

## Local documents, Office and Files

The `phone-domain::documents` module owns `OfficeDocument`, `DocumentFolder`, Word paragraphs, sparse workbook sheets and PowerPoint text slides. `DocumentAction` passes through the same ReactiveUI command queue as the other phone data. The state layer edits a candidate library, validates names, parent references, cycles, uniqueness and size limits, then publishes the complete mutation. Multi-document copy, move and deletion validate the entire deduplicated selection; a stale identifier, failed destination or name check does not consume identifiers or partially change the library. Unchanged rename/move operations preserve timestamps and do not save. Deleting an open or pinned document repairs navigation and secondary tiles through the existing cleanup path.

Office and Files share the persistent library. The hub presents recent and places views; Files supplies hierarchy, search, sorting, selection and destination selection. Word uses a native virtualized `ListBox` with cached source/template identities, while the Excel editor realizes a bounded 12-row by 6-column window into a sparse worksheet. Edits use native text controls and explicit save/cancel forms, so clock updates do not replace a user's draft. The paragraph editor, worksheet controls and slide editor remain small shared Rust view modules.

The Office New page exposes Word and Excel. PowerPoint opens supported existing presentations for text/notes editing and navigation; presentation construction remains a domain/interchange capability, with no native new-deck or add/delete-slide commands. This follows the original mobile feature boundary documented in [reference.md](reference.md).

Excel range commands parse at most 10,000 selected cells within the worksheet bounds. AutoSum inserts one of five supported aggregate formulas into an empty cell outside the source range. Find searches stored text/formulas across sheets in row/column order and wraps once. Stable single-key sorting moves complete selected rows, can retain a header, and puts empty cells last; formula-containing selections are rejected before mutation because reference relocation is not implemented. These typed native commands do not claim the original touch selection handles, full formatting or multi-key sort behavior.

Files Properties creates a bounded read-only snapshot. An individual file reports the byte length of its actual generated OOXML copy, location, modified date and content counts. Folder properties report recursive file/folder counts and explicitly labeled document-text size, not host disk allocation. `ShareDocuments` validates the whole selected set and generates actual package payloads for the host; supported imported documents require the explicit mobile-copy sharing action. Folder copy/move is excluded from the native UI, consistent with the original guide.

The document engine creates actual OPC/OOXML ZIP packages for DOCX, XLSX and PPTX. The ZIP reader checks central/local directory agreement, entry names, CRCs, methods, compression ratios and resource limits before parsing XML; it never extracts filesystem paths. Import is limited to 8 MiB of encoded input, 1,024 package entries, 4 MiB per expanded part and 16 MiB total expanded data. DTD/entity declarations, encryption, macros, embedded active objects and unsupported structures are rejected. Unused custom XML metadata is not imported, and mapped Word content controls remain unsupported.

The editable model is bounded to 512 documents, 256 folders, 2 MiB of text per document and 8 MiB of library text. A Word document has at most 2,000 paragraphs. Workbooks have at most 16 sheets and 10,000 populated cells per sheet within A1:IV10000; presentations have at most 200 slides. Formula evaluation uses its own parser with cycle detection, depth limits and an evaluation budget. It does not evaluate JavaScript, fetch data or resolve external workbook links.

`ImportDocuments` and `ExportDocument` are host requests. The browser uses a user-initiated file picker, passes bytes to Rust for validation, and downloads exported bytes as a file. The desktop uses FerroUI storage-provider pickers and the same import/export engine. Imported supported OOXML documents retain a compatibility notice; their exports receive a `-mobile-copy` suffix because desktop layout, fonts, themes and metadata are not preserved. Rich unsupported structures are rejected with an explanation. OneDrive and Office 365 remain explicit web handoffs, not authenticated synchronization.

## Build and deployment

The workspace imports FerroUI's dependency pins and build profiles through its synchronization script. Browser compilation uses the framework's Emscripten target and exact `wasm-bindgen` version. The browser build wrapper rejects app filenames reserved by the runtime, packages the runtime with app assets, prepares the static site, and statically links its JavaScript module graph without executing browser code.

One GitHub Actions workflow performs focused state/host checks, builds the browser, verifies its first rendered native frame, and deploys the Pages artifact. Longer interaction and screenshot checks are opt-in through `workflow_dispatch.verify_ui` or a `[verify-ui]` commit message; their screenshots are retained as a separate verification artifact. Documentation-only changes skip compilation. Toolchain and Cargo caches keep subsequent builds smaller. A deployed artifact comes from the same pinned framework and application source revision.

The Cargo cache retains the full target directory, including FerroUI's local path dependencies. Its compatibility prefix includes the pinned framework, Rust version, lockfile, manifests and build configuration; a source-commit suffix allows a successful build to refresh it. An additional fallback cache prefix retains dependencies across application manifest changes while keeping the OS, architecture, compiler and pinned framework boundary; Cargo fingerprints determine which changed inputs need rebuilding. Saving occurs before browser interaction checks. Clean tracked files and watched asset directories in the pinned framework receive stable commit timestamps, so a fresh checkout does not make unchanged framework inputs newer than cached Cargo fingerprints. Application source timestamps are left alone, and a modified framework checkout is rejected by the normalization helper.

An optional manual `reuse_run` avoids compilation when only tests or documentation have changed. Before selecting an immutable artifact ID, the workflow verifies a completed main-branch Pages run from the same repository and compares its runtime input trees with the current commit. It then repeats host checks, module linking and the full browser interaction suite. A runtime-source difference fails the reuse path, and publication still requires successful browser verification. Manual runs can set `publish=false` to retain diagnostic artifacts without deployment.

The browser suite locates controls through their live native bounds and hit tests, then sends trusted pointer, keyboard and touch input through Chromium. A hit on a control or its descendant permits a click; a hit on an ancestor does not make a clipped child clickable. Read-only text assertions check content presence separately, because a text block's center can lie between rendered glyphs. The 23-route screenshot gallery uses host navigation and complements the selected native interaction checks; [recorded verification](parity.md#recorded-verification) states the measured coverage.

Compilation, domain tests and smoke interactions establish different things. Compilation checks API/type compatibility; domain tests check state invariants; browser smoke checks detect startup and selected interaction failures. None alone establishes complete Windows Phone parity or hardware qualification.
