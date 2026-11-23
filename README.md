# Windows Phone

A Windows Phone 8.1 Update experience built with **FerroUI, Rust and declarative `rsx!` views**. The Start screen, apps, controls, navigation and data model run in FerroUI; portable host adapters supply storage, files, power observations and platform integrations.

[Open the app](https://ferroui.com/WindowsPhone/) · [App SDK](docs/sdk.md) · [Original references](docs/reference.md) · [Architecture](docs/architecture.md) · [Host integration](docs/host-integration.md) · [Platform and shell architecture](docs/platform-architecture.md) · [Feature and fidelity status](docs/parity.md)

The reference release is **Windows Phone 8.1 Update (2014)**, including live folders and the original `system` / `applications` Settings pivots. This is a working app recreation, not an operating system or an emulator for XAP/APPX applications. **Complete visual and feature parity has not been established.** The [parity document](docs/parity.md) records the implemented workflows and the remaining differences.

## App and platform expansion

The default build now composes **28 registered apps in 52 workspace packages**, including new Podcasts and Wallet apps. The existing app crates add Office undo/redo and typed formulas, OneNote notebooks and checklists, playlists and photo albums, saved-place directions and multi-city forecasts. The [feature matrix](docs/parity.md#app-and-platform-expansion) distinguishes implemented workflows, verification and remaining platform/service limits.

## Portable application, launcher and shell hosts

The new [platform architecture](docs/platform-architecture.md) separates `phone-runtime`, shared `phone-native` services and replaceable OS providers. Android and iOS now have native FerroUI entry points and package workflows, alongside browser, Windows, macOS and Linux. **Device apps** adds native-app discovery, search, launch and Start pinning on supported hosts. Android has separate App/HOME APK flavors; Linux has an opt-in labwc login session; supported Windows editions have reviewable Shell Launcher configuration. iOS and browser accurately retain their application/embedded role.

Native files, networking, persistence, media and camera reuse FerroUI and the shared host. Platform plugins supply actual role state, settings, sharing, haptics, permissions and OS media controls. `NativeHost::compose` accepts SDK apps and explicit service replacements for embedding. See the [mode matrix and composition example](docs/platform-architecture.md#hosting-modes-and-actual-authority).

## Preview

The browser renders the retained FerroUI control tree through WebAssembly. These unmodified captures come from the passing full browser verification of runtime [`7bc4107`](https://github.com/FerroUI/WindowsPhone/commit/7bc41077ae1dd8762071cf8af2f1602ea9390ff7), using isolated sample data and native pointer, keyboard and touch input.

![Windows Phone Start in the FerroUI browser app](docs/screenshots/expansion-start.png)

[Excel formulas/history](docs/screenshots/expansion-excel.png) · [OneNote checklist](docs/screenshots/expansion-onenote.png) · [Music playlist](docs/screenshots/expansion-music.png) · [Weather forecast](docs/screenshots/expansion-weather.png) · [Podcasts](docs/screenshots/expansion-podcasts.png) · [320-pixel Wallet](docs/screenshots/expansion-wallet.png) · [Maps directions](docs/screenshots/expansion-maps.png)

[Capture provenance](docs/screenshots/README.md) records the exact build, artifact and unmodified image sources, and retains earlier games, Calendar, Office, Files and host-integration captures. These are application captures; original-device pixel and motion parity has not been established.

## Explore

- **Start:** live content and notification counts, small/medium/wide tiles, pin/unpin, resize, reorder, folders that preserve their child tiles, secondary content tiles, accent colors, light/dark backgrounds, a shared scrolling photo behind transparent tile interiors, press-location tilt feedback, and the optional extra tile columns. Decorative motion respects reduced motion and Battery Saver.
- **Phone navigation:** swipe between Start and the alphabetical Apps list; search, measured letter jumps and sticky group headings; Back/Start/Search buttons; recent-app cards; configurable lock wallpaper and status providers; grouped Action Center notifications and deep links.
- **System controls:** separate ringer/notification and media/app volume, mute and vibration controls; a hardware-style volume panel; light/dark and high-contrast themes; reduced motion; live-tile preferences; date order and calendar week start. Battery information uses an actual host reading when available; storage views measure the saved phone document and separately show browser-origin storage estimates and persistence permission.
- **Messaging and Outlook:** search local threads and mail, select rows from the left edge or a held item, and apply bulk actions. Messaging includes contact selection, unknown-number contact creation, drafts, and copy/forward/delete commands on individual messages. Outlook includes folders, all/unread/flagged views, bulk read/flag changes, deleted items and restore. Compose actions explicitly open the host's messaging or email app; local drafts do not claim delivery.
- **Calendar:** day, week, month and year navigation; inline month appointments and an hourly day view with separate columns for overlapping appointments; native date/time controls; all-day appointments, duration, reminders, status and notes; daily/weekday/weekly/biweekly/monthly/yearly repeats with an end date and individual-occurrence editing or deletion. Start and lock details use the same occurrence engine. Import/export supported local-time iCalendar files and share a saved appointment or series.
- **Alarms and reminders:** native time/name/repeat/snooze editing, one-time and weekday alarms, foreground alert screens, configurable snooze and dismissal, and the original two-minute alarm snooze on Back. A Calendar reminder opens its occurrence. The browser plays a synthetic tone using the saved ringer level when browser audio is available; permitted foreground alert banners can also appear through device notifications. Delivery requires the app to remain running.
- **Personal apps:** contacts and favorites, atomic vCard imports, contact-file copy/sharing, call history, a calculator, and OneNote notebooks/sections with page movement, native checklists, sharing and bounded UTF-8 TXT/Markdown import. Wallet stores local membership cards, tickets and offers with favorites, archiving, expiry dates, exact Code 39 display and JSON interchange.
- **Office and Files:** a persistent document library with folders, search, sorting, multi-selection, atomic file copy/move, rename, properties, file sharing and secondary Start tiles. New creates Word or Excel documents; existing supported PowerPoint presentations offer text and speaker-note editing. Word, Excel and PowerPoint share bounded undo/redo. Excel includes typed range selection, five AutoSum operations, workbook find, range clear, bounded single-key value sorting and a typed 43-function formula engine with lazy IF/IFERROR and validated XLSX formula caches. Native editors share real DOCX/XLSX/PPTX import/export; UTF-8 TXT and CSV imports are supported too. Complex unsupported Office structures are rejected, and imported supported OOXML files export or share as explicitly named mobile copies.
- **Media:** import photos, manage named albums and favorites, rename, zoom, share and capture from the camera. Music and Video add playlists, queue ordering, artist/album metadata browsing, search, shuffle/repeat and host-observed progress. Podcasts subscribes to or imports RSS/Atom feeds, streams HTTPS episodes, remembers playback across app switches and pins shows to Start. Browser Media Session and native Windows SMTC, macOS Now Playing and Linux MPRIS integrate transport commands. Embedded desktop media/camera and the separate foreground alarm player depend on platform permissions, renderer and codecs.
- **Web and services:** native browser tabs with independent Back/Forward journals, frequent sites and history, editable/pinnable favorites, and a configurable address-bar shortcut. Maps adds saved places, home/work, recent searches, city/coordinate search and directions composition before explicit map-provider handoff. Weather saves up to ten cities, current/hourly/seven-day forecasts, units and freshness labels, with optional location-based current conditions. Web search, local Cortana-style commands and online-document links retain explicit host integrations.
- **Games and Store:** independent Tic tac toe and Minesweeper SDK apps with validated resumable games and persisted results. Tic tac toe offers a perfect phone opponent, two-player mode and unfinished-turn undo. Minesweeper offers three difficulties, safe opening areas, flags, chord reveal and best times. Games discovers registered game apps; Store enables, disables, pins and resets linked SDK apps. The original cloud Store and Xbox services are not connected.
- **App SDK:** immutable manifest registration, independent app routes, lifecycle/input hooks, typed app-owned session state, bounded JSON storage, data activation, custom live tiles, cancellable typed file/HTTPS tasks and a shared source-aware media queue/event channel. All built-in apps and the three Office editors live in separate crates. A scaffolder and compiled example show how to add another app without editing the system shell.
- **Persistence:** phone data stays in its host profile; browser media uses IndexedDB and desktop media uses app-owned files. Export and restore compatible JSON/media backups, including both wallpapers. Restore validates and saves before replacing live state; native restore preserves a recovery snapshot. The app includes installation/offline shell support. Settings can check for updates; a complete new build waits until every old app window closes, and separate hosted app scopes own separate caches.

The first run contains fictional example contacts, messages, mail, notes and photos so the phone has something useful to explore. These are local sample data, not a signed-in account or a simulated live inbox.

## Interaction

| Action | How |
| --- | --- |
| Open all apps | Swipe left from Start or use its app-list arrow. |
| Return to Start | Press the Windows button. |
| Go back | Press Back; dismiss menus and detail views before leaving an app. |
| See recent apps | Hold Back; right-click also opens its context action on a desktop. |
| Arrange Start | Hold a tile to select it; resize/unpin or drag it to another position. |
| Make a folder | Drop a held tile onto another tile. |
| Read notifications | Pull down from the status bar, or tap the status bar. |
| Adjust volume | Use the desktop phone frame's volume buttons, or Settings → ringtones+sounds → show volume controls. Expand the panel to adjust both channels. |
| Select mail or threads | Tap the left selection rail, hold a row, or choose select in the application bar. |
| Work with a message | Hold or right-click its bubble to copy, forward or delete it. |
| Restore deleted mail | Open Outlook → more → folders → deleted items, select a message, then more → move to inbox. The same command is available in its detail view. |
| Change app section | Select a pivot heading or swipe horizontally on a supported pivot surface. |
| Change Calendar period | Choose day/week/month/year with view, then swipe horizontally; tap a month in year view or a day in week/month view to inspect appointments. |
| Edit a recurring appointment | Open its details, choose edit or delete, then select this appointment or the entire series. |
| Snooze an alert | Select a snooze duration on the alert, then snooze; Back snoozes an alarm for two minutes and a Calendar reminder for five. |
| See more commands | Use the application bar's ellipsis. |
| Import contacts or appointments | Open People or Calendar, then more → import contacts or import appointments. |
| Share a contact or appointment | Open its details, then more → share contact, share appointment or share series. The desktop host saves an attachment copy. |
| Allow device banners | Open Settings → notifications+actions → Device notifications → allow device notifications. The browser owns the permission decision. |
| Keep browser data | Open Settings → storage sense → Device app storage → keep app data on this device, where the browser supports persistence requests. |
| Import other personal content | Use the relevant app's add command; file selection stays under your control. |

When the host delivers the shortcut to the app, **Home** opens Start, **Backspace/Escape** go back, **Ctrl/Cmd+F** opens search, and **Ctrl/Cmd+L** toggles the in-app lock. Editing controls keep their normal text-editing keys. Browsers can reserve some shortcuts for their own interface.

## Build

FerroUI is pinned as a Git submodule. Initialize it before running Cargo:

```bash
git clone --recurse-submodules https://github.com/FerroUI/WindowsPhone.git
cd WindowsPhone
git submodule update --init --recursive
```

The repository's Rust toolchain and dependency versions follow the pinned FerroUI revision. Do not independently upgrade `wasm-bindgen`, the Emscripten SDK, or the Rust browser target; they must agree with the framework's browser setup.

### Browser

Install Node.js 24 and the framework's pinned browser tools, then build:

```bash
bash FerroUI/scripts/browser/setup.sh "$HOME/browser-tools"
source "$HOME/browser-tools/env.sh"
bash scripts/build-browser.sh
```

The resulting static app is in `target/browser/phone-browser`. Serve that directory over HTTP; camera and installation features need a secure context, such as HTTPS or localhost:

```bash
python3 -m http.server 8080 --directory target/browser/phone-browser
```

### Desktop

```bash
cargo run --release -p phone-desktop
```

The desktop host uses the same shell, app crates, state and theme. Native FerroUI pickers support Office documents, vCard contacts, iCalendar appointments, photos, music, video, SDK file selections and compatible media backups. Embedded audio/video and camera use FerroUI's platform media providers. Native adapters supply one-shot location, explicit weather requests, device notification banners and foreground alarm audio. Sharing saves actual attachment copies through native pickers; it does not confirm delivery to another app. Restoring a backup preserves a recovery snapshot and requires a successful save before replacing live state. Windows, macOS and Linux power adapters report optional battery observations.

See [native prerequisites and macOS packaging](crates/phone-desktop/platform/README.md), including Linux GStreamer/EGL/PipeWire requirements and the macOS `.app` bundle needed for consent. Native OS media sessions are implemented; desktop haptics and background alarm wake-up remain outstanding. The [platform matrix](docs/host-integration.md) separates implemented adapters from device verification.

### Build an SDK app

```bash
python scripts/new-phone-app.py phone-app-counter --title Counter --id example.counter
cargo check -p phone-app-counter
```

Add the generated crate as a dependency of your composition, then pass its registration to `phone_ui::create_view_with_apps(vm, vec![phone_app_counter::registration()])`. The checked-in `phone-app-sdk-example` demonstrates this architecture without adding the example to the default app list. The [SDK guide](docs/sdk.md) covers declarative views, storage schemas, lifecycle, capability declarations, file tasks and live tiles.

### Checks

```bash
cargo test --locked -p phone-host -p phone-domain -p phone-state
cargo test --locked -p phone-sdk -p phone-controls -p phone-app-support -p phone-media-support
cargo test --locked -p phone-app-weather -p phone-app-maps -p phone-app-podcasts -p phone-app-wallet -p phone-app-onenote -p phone-app-photos
cargo test --locked -p phone-app-cortana -p phone-app-tic-tac-toe -p phone-app-minesweeper
cargo check --locked -p phone-desktop -p phone-app-sdk-example
node --test scripts/*.test.mjs
```

The Tic tac toe solver test explores every human move sequence against the phone's minimax player. Minesweeper tests cover safe openings, expansion, flags and restored-board validation. SDK tests cover stable reactive ownership, lifecycle ordering, capability refusal, bounded storage, file-task cancellation and stale completion after restore. Domain and state tests cover persistent-data validation, tile layout, navigation and mutations. Office cases cover supported OOXML roundtrips, bounded parsing, formula evaluation, atomic library changes and document activation. These checks do not establish pixel parity or validate every physical device.

With Chromium available, run the browser interaction and screenshot suite against a packaged build:

```bash
node scripts/ui-smoke.mjs target/browser/phone-browser target/ui-smoke
```

### Current expansion release

Runtime [`7bc4107`](https://github.com/FerroUI/WindowsPhone/commit/7bc41077ae1dd8762071cf8af2f1602ea9390ff7) passed [run 45181127412](https://github.com/FerroUI/WindowsPhone/actions/runs/45181127412) and deployed to [the public app](https://ferroui.com/WindowsPhone/). The browser gate passed **230 Rust tests**, **40 JavaScript tests** and **23 linked local modules**. Full rendered verification passed **64 interaction groups**, produced **124 captures**, and recorded **zero console/runtime errors** in **213.079 seconds**. This duration measures the suite, not startup latency or frame rate.

The expanded flows cover typed Excel formulas and Office history, OneNote section/checklist persistence, media playlists and photo albums, Podcasts feed/episode actions, Weather forecasts and units, narrow Wallet barcodes/pivots, saved-place Maps directions, and the existing shell, touch, density and offline reload checks. [Retained results](docs/verification/7bc4107-results.json), [eight reviewed captures](docs/screenshots/README.md#current-app-expansion-7bc4107) and the [publication record](docs/verification/7bc4107-publication.json) retain their exact source attribution.

The preceding [integrated run 45178205829](https://github.com/FerroUI/WindowsPhone/actions/runs/45178205829) on `6518960` passed the full browser suite and all three native jobs: **23 Linux**, **20 Windows** and **19 macOS host tests**, complete desktop build/link on each platform, and a separate real isolated Linux MPRIS protocol test. The final `7bc4107` follow-up adds persistent Maps field labels and improves that existing UI capture; native adapter code is identical. [Native evidence](docs/verification/6518960-native.json) distinguishes compile/protocol coverage from physical hardware and OS permission testing.

Public HTTP confirmed `7bc4107` in the served page and scope-owned service-worker cache revision. Settings → phone update checks for a new build. A downloaded update waits until **all old app windows and tabs close**; reopening then allows the new worker to take over. An ordinary refresh can keep the previous worker while another app window remains open.

### Earlier SDK release

The earlier [complete verification run 45163782836](https://github.com/FerroUI/WindowsPhone/actions/runs/45163782836) for [`cec2d1d`](https://github.com/FerroUI/WindowsPhone/commit/cec2d1d214310e33ef4908949fd3d6fe647630a9) passed **161 Rust tests**, **25 JavaScript tests**, linking of **21 packaged local modules**, and the native mobile first-frame check. Its full browser suite passed **53 interaction groups**, produced **104 captures**, and recorded **zero console/runtime error entries**. The suite's **165.364-second** duration measures verification time, not startup latency or frame rate.

The checks drive native input through Cortana discovery and local app-opening commands, reject disabled game discovery, verify exact saved boards and scores, reload and restore data, and exercise Start pinning and Store disable/re-enable. Cell-pixel assertions require visible flags before and after reload and full-color completed X/O marks. Existing Office, Files, Calendar, foreground alarms, live tiles, host byte callbacks, touch, density and offline checks remain in the suite. The [artifact](https://github.com/FerroUI/WindowsPhone/actions/runs/45163782836/artifacts/14812709655), [retained results](docs/verification/cec2d1d-results.json) and [six reviewed captures](docs/screenshots/README.md#earlier-sdk-app-verification-cec2d1d) identify the evidence.

<!-- BEGIN SDK LATEST RELEASE STATUS -->
The same run's [browser build](https://github.com/FerroUI/WindowsPhone/actions/runs/45163782836/job/137655455056), [Pages deployment](https://github.com/FerroUI/WindowsPhone/actions/runs/45163782836/job/137662122359) and all three native jobs succeeded. Native verification passed **16 Linux**, **14 macOS** and **15 Windows host tests**, then built and linked the complete desktop executable on each platform with real media/camera providers. Public HTTP confirmed `cec2d1d` in the index and `windows-phone-cec2d1d` in the service worker. The [publication record](docs/verification/cec2d1d-publication.json) keeps the browser, native, local and HTTP evidence distinct.
<!-- END SDK LATEST RELEASE STATUS -->

The earlier [73dc39f browser record](docs/verification/73dc39f-results.json) documents the original symbol-rendering correction, while the [9c392ef record](docs/verification/9c392ef-publication.json) preserves the singular-label release and successful Windows cold-build verification. Historical captures remain linked to their original source revision.

The [local extraction checkpoint](docs/verification/d8f3d03-local.json) at [`d8f3d03`](https://github.com/FerroUI/WindowsPhone/commit/d8f3d03c8b6604e12361aebcd9ecd7430e9f9bd1) passed **179 Rust tests** across all non-browser workspace packages, plus **25 JavaScript tests**. Local Linux media/camera compilation used unavailable-provider bridges because development libraries were absent; the separate native CI installs real platform dependencies. Neither compilation nor these tests demonstrate physical camera/audio hardware, OS consent dialogs, notification delivery, closed-app alarm wake-up or original-device visual parity. [The parity record](docs/parity.md#sdk-games-and-native-adapter-implementation) separates each revision and verification scope and retains the older release evidence.

## GitHub Pages

The `Build and publish` workflow runs on application changes pushed to `main` and can also be started manually. It uses the pinned FerroUI browser toolchain, restores dependency caches, runs focused state/host checks, builds one browser target, validates the packaged module graph and first native frame, uploads the static app, and deploys it to GitHub Pages. Documentation-only pushes do not rebuild. Routine publication has no platform matrix or separate lint workflow.

Rendered desktop/touch smoke checks and their screenshot artifact are opt-in: enable `verify_ui` for a manual run, or include `[verify-ui]` in the pushed commit message. This keeps routine publication lean while retaining an explicit visual verification path.

Native verification is also opt-in: the `verify_native` manual input or a `[native-only]` push marker runs Windows, macOS and Linux host checks without rebuilding Pages. Use the native-only path for platform-specific adapter changes; its independent concurrency group preserves an in-progress browser release. A `[verify-native]` push marker adds the native matrix to a browser release. The Linux check installs media development libraries so it compiles the full native bridges. These jobs build and link the complete desktop executable and run portable host tests; they do not open permission dialogs or exercise physical hardware. Native caches retain the compiler and exact framework pin boundary while reusing unchanged dependencies across app manifest updates. Windows additionally pins the compatible Windows 2022 runner and verifies LLVM against the MSVC standard library before compiling; its cache retains that runner boundary.

For test or documentation changes, a manual run can set `reuse_run` to a completed main-branch run with a `compiled-app` artifact. The workflow checks the source repository, workflow, commit and runtime input trees before downloading that artifact. Reuse still runs host checks, module-link validation and the full browser suite before publication; changes to application sources or the FerroUI pin require a normal build.

Set `publish` to `false` for a manual verification run that retains its test artifacts without deploying to Pages. Push-triggered runs publish after their checks succeed.

Set the repository's **Settings → Pages → Build and deployment → Source** to **GitHub Actions** if it is not already configured. The workflow also requests Pages enablement. The app is published through the organization's Pages domain at:

**https://ferroui.com/WindowsPhone/**

The successful deployment run is the source of truth for publication; a source commit alone does not establish that the site is live.

## Project layout

| Crate | Responsibility |
| --- | --- |
| `phone-host` | UI-independent host contracts, permissions/observations, URI validation, native paths, atomic files, power adapters and bounded background work. |
| `phone-domain` | Serializable phone document, app catalog, tile packing, calendar and calculator logic, validation. |
| `phone-state` | ReactiveUI view model and commands, navigation journal, DynamicData projections, persistence boundary. |
| `phone-controls` | Trimmed WinUI base, Metro control templates, palette, typography, vector icons and reusable app bars. |
| `phone-fonts` | Embedded Microsoft Selawik font collection, four weights, licensing and shared host registration. |
| `phone-sdk` | Manifest/registry contracts, contexts, typed session state, bounded app storage, lifecycle hooks, typed host tasks and reusable page helpers. |
| `phone-system` | Shared `rsx!` system shell, Start/app list, lock screen, notifications, hardware panels, previews and gestures. |
| `phone-alarm-service` | Reusable foreground alarm/reminder scheduling and alert/editor state. |
| `crates/apps/phone-app-*` | Separate built-in apps, Word/Excel/PowerPoint editors, shared app support, independent games and SDK example. |
| `phone-ui` | Default app composition and compatible host-facing facade; no embedded app implementations. |
| `phone-browser` | FerroUI browser startup, WebAssembly exports, browser services and static/PWA assets. |
| `phone-runtime` | Explicit providers, platform capabilities, bounded request/lifecycle ownership and activation. |
| `phone-native` | Shared native persistence/recovery, pickers, documents, networking, media/camera and embedding composition. |
| `phone-platform-desktop` | Windows/macOS/Linux app catalogs, activation, settings and observed shell-session policy. |
| `phone-desktop` | Window/renderer/lifetime composition and app/launcher/shell modes. |
| `phone-android` | Native application and HOME flavors, Android OS/services/media and embeddable bootstrap. |
| `phone-ios` | UIKit/Metal application, iOS OS/services/media and simulator bundle. |

Read [architecture](docs/architecture.md) for the state, rendering and service boundaries, and [parity](docs/parity.md) before assuming original phone capabilities are available.

## Fonts and attribution

The original design uses Segoe WP. The theme requests Segoe WP or Segoe UI when the host provides a licensed system copy, then falls back to bundled **Microsoft Selawik 1.01** and FerroUI's Inter. The `phone-fonts` crate embeds unmodified Light, Regular, Semibold and Bold Selawik files under the SIL Open Font License 1.1. Microsoft describes Selawik as an open replacement for Segoe UI and records outstanding kerning and hinting differences; the font is not a claim of Segoe WP pixel parity. See [font provenance and license](crates/phone-fonts/NOTICE.md). Original interface research and source links are collected in [the reference document](docs/reference.md). Third-party component and bundled asset notices are in [NOTICE.md](NOTICE.md) and the FerroUI submodule.

Windows Phone, Lumia, Microsoft, Cortana and related names identify the referenced products. This project is an independent recreation and is not an official Microsoft product.
