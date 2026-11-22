# Windows Phone

A Windows Phone 8.1 Update experience built with **FerroUI, Rust and declarative `rsx!` views**. The Start screen, apps, controls, navigation and data model run in FerroUI; the browser host supplies storage, camera, media, sharing and web integration.

[Open the app](https://ferroui.github.io/WindowsPhone/) · [Original references](docs/reference.md) · [Architecture](docs/architecture.md) · [Feature and fidelity status](docs/parity.md)

The reference release is **Windows Phone 8.1 Update (2014)**, including live folders and the original `system` / `applications` Settings pivots. This is a working app recreation, not an operating system or an emulator for XAP/APPX applications. **Complete visual and feature parity has not been established.** The [parity document](docs/parity.md) records the implemented workflows and the remaining differences.

## Explore

- **Start:** live content and notification counts, small/medium/wide tiles, pin/unpin, resize, reorder, folders that preserve their child tiles, secondary content tiles, accent colors, light/dark backgrounds, photos behind transparent tile interiors, and the optional extra tile columns.
- **Phone navigation:** swipe between Start and the alphabetical Apps list; search and letter jump; Back/Start/Search buttons; recent-app cards; lock view; Action Center and notification deep links.
- **Personal apps:** contacts and favorites, conversation drafts, call history, sample mail and mail composition, appointments, alarms, notes, and a calculator.
- **Media:** import photos, browse the Camera Roll, select favorites, zoom, share, and delete; capture from the camera; import audio and video and play them through browser media APIs.
- **Web and services:** browser address entry and favorites, web search, OpenStreetMap handoff, current weather after location permission, local Cortana-style search and commands, and links to online documents.
- **Games and Store:** a playable tic-tac-toe game and management of the included app collection. The original cloud Store and Xbox services are not connected.
- **Persistence:** phone data remains in the current browser; media uses IndexedDB. Export and restore a JSON backup, including imported media. The app includes a manifest and service worker for installation and offline shell use.

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
| Change app section | Select a pivot heading or swipe horizontally on a supported pivot surface. |
| See more commands | Use the application bar's ellipsis. |
| Import personal content | Use the relevant app's add command; file selection stays under your control. |

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

The desktop host uses the same shell, views, state and theme and persists its phone document to the user's data directory, retaining unreadable data for recovery. It hands HTTP(S), phone, message and mail requests to the operating system's registered apps and exports a document backup. Camera, imported media, sharing, location and backup restoration currently use the browser host.

### Checks

```bash
cargo test --locked -p phone-domain -p phone-state
cargo check --locked -p phone-ui
node --test scripts/*.test.mjs
rustc --edition=2021 --test crates/phone-ui/src/apps/games/logic.rs -o target/game-tests
target/game-tests
```

The pure game test explores every human move sequence against the phone's minimax player. The domain and state tests cover persistent-data validation, tile layout, navigation and mutations. These checks do not establish pixel parity or validate every physical device.

## GitHub Pages

The `Build and publish` workflow runs on application changes pushed to `main` and can also be started manually. It uses the pinned FerroUI browser toolchain, restores dependency caches, runs focused state/host checks, builds one browser target, validates the packaged module graph and first native frame, uploads the static app, and deploys it to GitHub Pages. Documentation-only pushes do not rebuild. There is no separate lint workflow or platform matrix.

Rendered desktop/touch smoke checks and their screenshot artifact are opt-in: enable `verify_ui` for a manual run, or include `[verify-ui]` in the pushed commit message. This keeps routine publication lean while retaining an explicit visual verification path.

Set the repository's **Settings → Pages → Build and deployment → Source** to **GitHub Actions** if it is not already configured. The workflow also requests Pages enablement. The intended URL for this repository is:

**https://ferroui.github.io/WindowsPhone/**

The successful deployment run is the source of truth for publication; a source commit alone does not establish that the site is live.

## Project layout

| Crate | Responsibility |
| --- | --- |
| `phone-domain` | Serializable phone document, app catalog, tile packing, calendar and calculator logic, validation. |
| `phone-state` | ReactiveUI view model and commands, navigation journal, DynamicData projections, persistence boundary. |
| `phone-controls` | Trimmed WinUI base, Metro control templates, palette, typography, vector icons and reusable app bars. |
| `phone-ui` | Shared `rsx!` shell and app views, session state, gestures, images and host-facing UI hooks. |
| `phone-browser` | FerroUI browser startup, WebAssembly exports, browser services and static/PWA assets. |
| `phone-desktop` | Native window startup, local document persistence/recovery, URI handoff and document backup export. |

Read [architecture](docs/architecture.md) for the state, rendering and service boundaries, and [parity](docs/parity.md) before assuming original phone capabilities are available.

## Fonts and attribution

The original design uses Segoe WP. The app requests Segoe UI where the host can supply it and includes FerroUI's Inter family as a portable fallback. It does not redistribute proprietary Segoe WP fonts; text metrics consequently differ on some hosts. Original interface research and source links are collected in [the reference document](docs/reference.md). Third-party component and bundled asset notices are in [NOTICE.md](NOTICE.md) and the FerroUI submodule.

Windows Phone, Lumia, Microsoft, Cortana and related names identify the referenced products. This project is an independent recreation and is not an official Microsoft product.
