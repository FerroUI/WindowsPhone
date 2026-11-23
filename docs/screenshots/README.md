# Screenshot provenance

The images in this directory are application captures. They are not original Windows Phone reference screenshots and do not establish pixel equality with original hardware.

## Native iOS (5b41a55)

These unmodified captures come from successful [iOS run 45223785634](https://github.com/FerroUI/WindowsPhone/actions/runs/45223785634) at [`5b41a557c83c59ea864c878199065f4dba07f5c9`](https://github.com/FerroUI/WindowsPhone/commit/5b41a557c83c59ea864c878199065f4dba07f5c9). The [UI artifact](https://github.com/FerroUI/WindowsPhone/actions/runs/45223785634/artifacts/14845270139) has ZIP SHA-256 `f5c5218acba898a162f5bbe540fb971411967a13692abcc792f5ea70e463d6bd`. The [iOS record](../verification/5b41a55-ios.json) retains all three original observations, image hashes, actual URL acceptance and XCTest results.

| Repository image | Artifact source | Pixels | Captured state |
| --- | --- | --- | --- |
| [iOS Start](platform-start-ios.png) | `start.png` | 828 × 1792 | Native UIKit/Metal application after mounting its shared FerroUI controls. |
| [iOS Wallet](platform-wallet-ios.png) | `wallet.png` | 828 × 1792 | Wallet root after the real UIKit URL callback accepted its activation. |

At scale 2, the native client measured **414 × 896 points**, with **48 top / 34 bottom** safe-area insets and a **414 × 814** inner host, PhoneScreen and view-model viewport. The private simulator mounted Start, opened Wallet through `simctl openurl` and returned Home. A public XCTest helper accepted the exact system URL confirmation; it did not inject app state. Fresh observations had the expected visible controls and accepted URIs.

All three source images were reviewed. Wallet's search, empty-state text and commands fit the native surface. The status icons and Back/Start/Search strip remain present: their raw pixel regions are identical in Start, Wallet and restored Home, as recorded with region hashes. The original files remain fully opaque and unmodified. These images qualify the stated simulator states, not physical hardware, notification/location consent, tactile output, codec playback, distribution signing or original Windows Phone pixel fidelity.

## Native Android (895b47e)

These unmodified images come from successful [Android UI run 45222094681](https://github.com/FerroUI/WindowsPhone/actions/runs/45222094681) at [`895b47e2cb969fd7069fd056f180a6b1366d9e93`](https://github.com/FerroUI/WindowsPhone/commit/895b47e2cb969fd7069fd056f180a6b1366d9e93). The [UI artifact](https://github.com/FerroUI/WindowsPhone/actions/runs/45222094681/artifacts/14842370879) has ZIP SHA-256 `58b51142f0530168d8a62413401103066b6e486dcda411b688b3755de173f7d8`. The [runtime record](../verification/895b47e-android-ui.json) retains all six App/Launcher route observations, capture hashes and visual findings.

| Repository image | Artifact source | Pixels | Captured state |
| --- | --- | --- | --- |
| [Android Start](platform-start-android.png) | `app-start.png` | 720 × 1440 | Ordinary Android application after real Home URI activation, with fictional first-run tiles. |
| [Android Wallet](platform-wallet-android.png) | `app-wallet.png` | 720 × 1440 | Actual Wallet root, empty local wallet, search and add/import/more commands. |

An isolated Android 36 x86_64 emulator rendered the native FerroUI tree. At scale 2, the client measured **360 × 720**, system insets **68 top / 24 bottom**, and the inner host, PhoneScreen and view-model viewport **360 × 628**. Both App and Launcher packages completed Start → Wallet → Home with fresh mounted-control observations and exact-component foreground checks. The original current user and HOME holder were preserved; neither package acquired HOME authority in this gate.

All six captures were visually reviewed. The native content and phone navigation remain separate from Android's status/gesture areas. Wallet's controls and empty-state text fit; its partially visible next pivot and Start's partial lower tile row belong to their scrolling surfaces. These are native emulator captures, distinct from the browser screenshots below. They establish the named runtime states, not physical/OEM behavior, HOME consent, hardware permissions, media playback or original-device pixel fidelity.

## Portable platform browser (01c4d69)

These three unmodified captures come from [run 45211399089](https://github.com/FerroUI/WindowsPhone/actions/runs/45211399089) at [`01c4d697c6b32049c8b919899fbb0416f5b61144`](https://github.com/FerroUI/WindowsPhone/commit/01c4d697c6b32049c8b919899fbb0416f5b61144). The [UI artifact](https://github.com/FerroUI/WindowsPhone/actions/runs/45211399089/artifacts/14837427101) contains the packaged browser interaction evidence; its ZIP SHA-256 is `a94d201ce82a19f9dab8b9b9e7e061626a40598fe387c7ad8e65da17a87722b3`.

| Repository image | Artifact source | Pixels | Captured state |
| --- | --- | --- | --- |
| [Device apps](platform-device-browser.png) | `platform-browser-device-services.png` | 1280 × 936 | Observed browser services and unsupported Home/shell roles, with user-facing availability text. |
| [Desktop browser Start](platform-start-browser.png) | `01-start-desktop.png` | 1280 × 936 | Shared phone Start in the ordinary desktop browser frame. |
| [Mobile browser Start](platform-start-mobile-browser.png) | `13-start-mobile-dpr2.png` | 780 × 1688 | Shared phone Start at a 390-by-844 CSS viewport and device pixel ratio 2. |

All three selected captures were visually reviewed. Device apps shows supported browser actions and the unavailable Home/shell roles, without offering an installed-app catalog. Its paragraph and buttons are readable, and refresh/navigation remain visible. Desktop Start uses the phone frame; mobile Start fills its viewport at 2× pixel density. Partly visible lower tiles belong to the scrolling viewport.

The suite passed **66 groups**, produced **125 captures**, and recorded **zero console/runtime errors** in **189.052 seconds**. [Unmodified results](../verification/01c4d69-results.json) and the [publication record](../verification/01c4d69-browser.json) retain per-image hashes, byte sizes and direct HTTP observations. The duration is verification time, not a rendering-performance result. Browser permission prompts, physical vibration, share delivery, native app launching and original-device visual equality are outside this capture set.

## Current app expansion (7bc4107)

These eight unmodified PNGs come from the [UI artifact of run 45181127412](https://github.com/FerroUI/WindowsPhone/actions/runs/45181127412/artifacts/14822143207), built from [`7bc41077ae1dd8762071cf8af2f1602ea9390ff7`](https://github.com/FerroUI/WindowsPhone/commit/7bc41077ae1dd8762071cf8af2f1602ea9390ff7). Chromium rendered the packaged FerroUI app with isolated local fixtures and native input. Standard desktop captures are **1280 × 936 pixels**, containing a **400 × 820** phone surface. The narrow Wallet capture is **320 × 740 pixels**. Each file's original dimensions, byte length and SHA-256 are retained in the [publication record](../verification/7bc4107-publication.json).

| Repository image | Artifact source | Captured state |
| --- | --- | --- |
| [Start](expansion-start.png) | `01-start-desktop.png` | The retained desktop Start layout and fictional local tiles. |
| [Excel](expansion-excel.png) | `office-excel-typed-formulas-and-history.png` | Typed formula editing and undo/redo in the native worksheet. |
| [OneNote](expansion-onenote.png) | `onenote-restored-section-and-checklist.png` | Restored section membership and checked/unchecked tasks. |
| [Music](expansion-music.png) | `music-persisted-playlist.png` | A persisted playlist with the playlists category selected. |
| [Weather](expansion-weather.png) | `weather-daily-forecast.png` | The daily forecast and Fahrenheit display from a controlled response fixture. |
| [Podcasts](expansion-podcasts.png) | `podcasts-episode-detail.png` | Imported feed episode details and native playback actions. |
| [Wallet](expansion-wallet.png) | `wallet-narrow-barcode.png` | The longest supported Code 39 payload and native commands at a 320-pixel width. |
| [Maps](expansion-maps.png) | `maps-saved-place-directions.png` | Swapped origin/destination with persistent from/to labels and walking selected. |

The full suite passed **64 groups**, produced **124 captures**, and recorded **zero console/runtime errors** in **213.079 seconds**. The [retained results](../verification/7bc4107-results.json) have SHA-256 `636170f4f276fa5ad1890c3a043319100085646b5eae2b5bff1046d83413670c`; the artifact ZIP has SHA-256 `a2a34b5430be84f4497e390c668deab10cdd5b28c47bc0a247d7b9b8f470cd05`. The duration is verification time, not startup or rendering performance.

All eight selected images were visually reviewed. Wallet's longest Code 39 payload and bottom commands fit the 320-pixel screen. Music retains its selected playlists category on the detail page. Maps has persistent from/to labels, clear swapped values and visible travel/actions after the temporary status banner is dismissed through its native control. Excel and OneNote retain readable editing/checklist layouts. Start's partial lower row belongs to its scrolling viewport.

Weather's forecast is controlled fixture data, and Podcasts' local feed fixture does not prove network streaming or audible playback. The Maps fixture prepares but does not launch external directions. These isolated application captures do not establish original-device pixel/motion equality, hardware or OS consent behavior, authenticated services, or an existing public profile's worker upgrade. The public build is confirmed separately by direct HTTP observations. Earlier captures below retain their own revision attribution.

<!-- BEGIN SDK SCREENSHOT EVIDENCE CEC2D1D -->

## Earlier SDK app verification (cec2d1d)

These six unmodified PNGs come from the [verification artifact of run 45163782836](https://github.com/FerroUI/WindowsPhone/actions/runs/45163782836/artifacts/14812709655), built from [`cec2d1d214310e33ef4908949fd3d6fe647630a9`](https://github.com/FerroUI/WindowsPhone/commit/cec2d1d214310e33ef4908949fd3d6fe647630a9). Chromium rendered the packaged FerroUI application with isolated local fixtures and native input. Desktop images are **1280 × 936 pixels**; mobile Start uses a **390 × 844 CSS viewport**, captured at **780 × 1688 pixels with DPR2**.

| Repository image | Artifact source | Captured state |
| --- | --- | --- |
| [SDK Start](sdk-start-desktop.png) | `01-start-desktop.png` | Desktop Start using the shared system and independent app crates. |
| [SDK mobile Start](sdk-start-mobile.png) | `13-start-mobile-dpr2.png` | Mobile Start at 2× density with fully readable tile labels. |
| [Tic tac toe](sdk-tic-tac-toe.png) | `sdk-tic-tac-toe-completed-game.png` | A completed two-player win after native local app discovery/open checks. |
| [Minesweeper playing](sdk-minesweeper-playing.png) | `sdk-minesweeper-playing.png` | Safe opening, a visible vector flag and the corrected `1 move` counter. |
| [Minesweeper after reload](sdk-minesweeper-restored.png) | `sdk-minesweeper-restored.png` | The same saved board and flag after a real page reload. |
| [Disabled SDK game](sdk-store-disabled.png) | `sdk-game-store-disabled.png` | Store details show **enable**, preserved saved-data size and separate reset. |

The full suite passed **53 groups**, produced **104 captures**, and recorded **zero console/runtime errors** in **165.364 seconds**. Its [retained results](../verification/cec2d1d-results.json) have SHA-256 `ce1834fbc7a06bb4df7cf7d9034c56fe880ebe18dd60c6c86bc0bd67d638869a`; the artifact ZIP has SHA-256 `ae3045a225ef578da14fe22ca2505004a215a1f99c25dba40b3e2e1c69c58d10`. The same run passed browser deployment and complete Windows/macOS/Linux desktop links. [The publication record](../verification/cec2d1d-publication.json) records the separate test and HTTP evidence.

All six selected images were visually inspected. Flags and completed X/O symbols remain visible; the singular minefield label is correct before and after reload. Start's partial lower row is scrollable viewport continuation, and the Photos entry frame is a finite animation. The Minesweeper instructions and statistics below the board remain in its ScrollViewer. Store labels and saved-data controls have no observed overlap or truncation in the selected frame.

The associated native-input assertions additionally verify local game discovery/open, disabled-app exclusion, no search-related data writes, exact game progress after reload, Start links and Store lifecycle. These still frames do not establish animation traces, original-device pixel equality, hardware/permission behavior or an existing public browser profile's upgrade. Historical images below keep their original revision-specific links.

<!-- END SDK SCREENSHOT EVIDENCE CEC2D1D -->

<!-- BEGIN SDK SCREENSHOT EVIDENCE 73DC39F -->

## SDK app verification (73dc39f)

The following unmodified PNGs come from the [`ui-verification` artifact of run 45155078320](https://github.com/FerroUI/WindowsPhone/actions/runs/45155078320/artifacts/14808786559), built from runtime [`73dc39f1b0072166c7f57d54e2fbb687e252c0ba`](https://github.com/FerroUI/WindowsPhone/commit/73dc39f1b0072166c7f57d54e2fbb687e252c0ba). Chromium rendered the packaged FerroUI application using isolated local fixtures. These images are not captures of an original Windows Phone device, a newly inspected public browser profile or native Windows/macOS hardware. Desktop captures are **1280 × 936 pixels**; mobile Start uses a **390 × 844 CSS viewport** with a **780 × 1688 capture at DPR2**.

| Repository image | Artifact source | Captured state |
| --- | --- | --- |
| [SDK Start](https://github.com/FerroUI/WindowsPhone/blob/69849dbe5793490c4030673fc74c0d7b90950d77/docs/screenshots/sdk-start-desktop.png) | `01-start-desktop.png` | Desktop Start after extracting the shared system and independent app crates. |
| [SDK mobile Start](https://github.com/FerroUI/WindowsPhone/blob/69849dbe5793490c4030673fc74c0d7b90950d77/docs/screenshots/sdk-start-mobile.png) | `13-start-mobile-dpr2.png` | The retained mobile Start layout at 2× browser density. |
| [Tic tac toe](https://github.com/FerroUI/WindowsPhone/blob/69849dbe5793490c4030673fc74c0d7b90950d77/docs/screenshots/sdk-tic-tac-toe.png) | `sdk-tic-tac-toe-completed-game.png` | A completed local two-player win in the independently registered SDK app. |
| [Minesweeper playing](https://github.com/FerroUI/WindowsPhone/blob/69849dbe5793490c4030673fc74c0d7b90950d77/docs/screenshots/sdk-minesweeper-playing.png) | `sdk-minesweeper-playing.png` | A safely opened minefield with a native flag action and visible elapsed clock. |
| [Minesweeper after reload](https://github.com/FerroUI/WindowsPhone/blob/69849dbe5793490c4030673fc74c0d7b90950d77/docs/screenshots/sdk-minesweeper-restored.png) | `sdk-minesweeper-restored.png` | The saved board and flag state reopened after an actual page reload. The separate restore assertion follows this capture. |
| [Disabled SDK game](https://github.com/FerroUI/WindowsPhone/blob/69849dbe5793490c4030673fc74c0d7b90950d77/docs/screenshots/sdk-store-disabled.png) | `sdk-game-store-disabled.png` | Store details after disabling Tic tac toe, with the Enable command available. Assertions separately verify removed pins and preserved progress. |

The suite passed **53 check groups**, produced **104 captures**, and recorded **zero console/runtime error entries** in **153.227 seconds**. That duration describes the test run, not startup or rendering performance. The original `results.json` has SHA-256 `9800e148edc5dc0293a27b56ea22e44b05959fb375de4ab2fed1d19ac1646e28`; the complete artifact ZIP SHA-256 is `506fcbbed0261dbd46c913cf5c00a5a11c57aeecb50e66ee57ac10047e63a866`.

The run's browser build and Pages deployment jobs succeeded. The same run's Windows native job stopped on deprecated coordinate accessors; that platform-only correction is tracked separately in the [native verification run](https://github.com/FerroUI/WindowsPhone/actions/runs/45159375726). [The parity record](../parity.md#browser-verification-73dc39f) keeps the browser and native evidence distinct.

All six historical images were visually inspected; their links are pinned to the earlier documentation commit. The flagged minefield cell contains **271 amber pixels** in both gameplay and reload captures; completed Tic tac toe cells contain **943 accent-colored X pixels** and **1,104 white O pixels** under the documented sampling bounds. The prior font-dependent flag and dimmed completed symbols failed those criteria. The readable vectors are noninteractive overlays; native Buttons still own availability, input and accessible labels.

The instruction paragraph below Minesweeper's board exceeds the initial viewport but belongs to the same ScrollViewer; its complete instructions and statistics are reachable by scrolling. Start's partially visible lower tiles also belong to its scroll viewport. These still frames do not measure animation curves; the Photos content has a finite vertical entrance. The later `9c392ef` source corrects the displayed singular counter labels (`1 move`, `1 second`, `1 mine left`). These PNGs are unmodified `73dc39f` captures and retain the previous wording.


The images demonstrate their visible states. The accompanying native-input and state assertions establish the tested gameplay, persistence and Store/Start behavior; screenshots alone do not establish those transitions. No OS file picker, physical camera, real notification prompt, authenticated service or original-device pixel comparison is claimed by this capture set.

<!-- END SDK SCREENSHOT EVIDENCE 73DC39F -->

## Earlier browser verification (b4c63a3)

The following unmodified PNGs come from the [`ui-verification` artifact of run 45138808865](https://github.com/FerroUI/WindowsPhone/actions/runs/45138808865/artifacts/14802170105), built from deployed runtime [`b4c63a36ed7d7d35168085409033711a61951103`](https://github.com/FerroUI/WindowsPhone/commit/b4c63a36ed7d7d35168085409033711a61951103). Chromium rendered the packaged FerroUI WebAssembly application in isolated test fixtures. These are not captures from a newly inspected public browser profile. Desktop images are 1280 × 936 pixels; mobile Start uses a 390 × 844 CSS viewport and a 780 × 1688 backing capture at device-pixel-ratio 2.

| Repository image | Artifact source | Captured state |
| --- | --- | --- |
| [People contact](host-people-vcard.png) | `people-native-vcard-contact.png` | An actual vCard imported through the production Wasm callback, with native contact share/copy commands exposed. |
| [Calendar series](host-calendar-series.png) | `calendar-native-icalendar-series.png` | An actual local-time iCalendar series imported through the production Wasm callback, with explicit series share/copy commands. |
| [Storage Sense](host-storage.png) | `host-native-storage-observations.png` | Native Settings displays controlled origin-usage/quota observations from the production observer and exposes an explicit persistence command. The values are fixture data, not host disk measurements. |
| [Device notification permission](host-notifications.png) | `host-native-notification-permission.png` | Native Settings displays the injected unrequested permission state and an explicit request command. No actual permission request or notification delivery occurred. |
| [Start](host-start-desktop.png) | `01-start-desktop.png` | Native desktop Start layout in the `b4c63a3` packaged runtime. |
| [Mobile Start](host-start-mobile.png) | `13-start-mobile-dpr2.png` | Native Start at 2× browser density in the `b4c63a3` packaged runtime. |

The full suite passed **49 check groups**, produced **100 captures**, and recorded **zero console/runtime errors** in **148.364 seconds**. The original [`results.json`](../verification/b4c63a3-results.json) has SHA-256 `6d29304ef7632f0188cbe18c53490a13ac759d0207f626d1c0617cd729f78755`; the complete artifact ZIP SHA-256 is `f004f37fa1ec121d1aedebe3b2581be86d42a8046200c5b1ac71ee9666938f2c`. Both the [build](https://github.com/FerroUI/WindowsPhone/actions/runs/45138808865/job/137571455356) and [Pages deployment](https://github.com/FerroUI/WindowsPhone/actions/runs/45138808865/job/137575350649) jobs succeeded. The duration measures suite execution, not application startup or frame rate.

All six images were visually inspected for coherent layout and reachable native commands. Their associated assertions establish the tested byte interchange, menu availability and observation behavior. They do not show OS share-sheet delivery, system file pickers, notification permission prompts, actual device battery readings or external-player playback. Local Linux unit verification and the unexecuted Windows/macOS/mobile boundaries are recorded in [the earlier host-integration verification](../parity.md#earlier-host-integration-verification-b4c63a3) and [local results](../verification/b4c63a3-local.json). No new cloud-browser interaction was performed in this pass; existing-profile upgrade recovery and original-device pixel parity remain unverified.

The separate [publication record](../verification/b4c63a3-publication.json) confirms the public HTTP index build marker `b4c63a3` and service-worker cache revision `windows-phone-b4c63a3` after deployment. It establishes the served revision without presenting the isolated test captures as screenshots from that public browser session.

## Previous browser verification (6a7cf14)

The following unmodified PNGs come from the [`ui-verification` artifact of run 45131005447](https://github.com/FerroUI/WindowsPhone/actions/runs/45131005447/artifacts/14797962911), built from deployed runtime [`6a7cf1422bc1153fb668f754b71d3b0bf4f4fff6`](https://github.com/FerroUI/WindowsPhone/commit/6a7cf1422bc1153fb668f754b71d3b0bf4f4fff6). Chromium rendered the packaged FerroUI WebAssembly application with native input. These are isolated test fixtures, not captures of the public browser profile. Desktop captures are 1280 × 936 pixels; mobile Start uses a 390 × 844 CSS viewport at device-pixel-ratio 2.

| Repository image | Artifact source | Captured state |
| --- | --- | --- |
| [Start](start-current-desktop.png) | `01-start-desktop.png` | Native Start layout in the `6a7cf14` runtime. |
| [Mobile Start](start-current-mobile.png) | `13-start-mobile-dpr2.png` | Mobile layout and rendered backing pixels at 2× density. |
| [Month](calendar-month-native.png) | `calendar-month-inline-appointments.png` | Inline appointments for the selected date. |
| [Day](calendar-day-native.png) | `calendar-day-overlapping-appointments.png` | All-day content and independently hittable overlapping appointment columns. |
| [Occurrence editor](calendar-editor-native.png) | `calendar-native-occurrence-editor.png` | A single occurrence draft after accepting 10:00 in the native time picker. |
| [Time picker](calendar-time-picker-native.png) | `calendar-native-time-picker.png` | Focused hour selector after native Arrow Down input advances 09:00 to 10:00. |
| [Alarm editor](alarm-editor-native.png) | `alarm-native-editor.png` | Repeating alarm draft with a selected snooze duration. |
| [Alarm alert](alarm-alert-native.png) | `alarm-native-ringing.png` | Foreground alert covering an unsaved Office form. |
| [Excel range](excel-range-native.png) | `office-excel-range-sort-and-autosum.png` | Selected range, sorted values and aggregate results. |
| [Files Properties](files-properties-native.png) | `files-native-file-properties.png` | Actual exported package bytes and correct singular paragraph wording. |
| [Shared wallpaper](start-wallpaper-native.png) | `shell-wallpaper-tall-layout-bottom.png` | Continuous photo sampling across a tall Start layout. |

The suite passed **45 check groups**, produced **96 captures**, and recorded **zero console/runtime errors** in **128.451 seconds**. The original [`results.json`](../verification/6a7cf14-results.json) is retained in the repository; its SHA-256 is `129e401f3fbc9b86b0990b5f29b8cb2af9544d0d6fe72d075a123f814c90b64d`. The complete artifact ZIP SHA-256 is `d1cf4149a5b85019639b55f0c2f3894e0776047a0cbb5f074e2bdfe5bb497a19`. Images establish the visible state at capture time; the associated native-input and state assertions establish the tested interactions and persistence.

## Publication and live-preview limitation

This is a historical record of the earlier `6a7cf14` publication attempt. That run's build and Pages deployment jobs succeeded. On 23 November 2026, a direct HTTP response from [the public app](https://ferroui.com/WindowsPhone/) contained `data-phone-build="6a7cf14"`, with `Last-Modified: Mon, 23 Nov 2026 07:33:53 GMT`; the public `app-sw.js` declared `windows-phone-6a7cf14`.

During that earlier check, the existing cloud-browser profile continued to display build `528c982` after reloads and in a fresh tab, including the app's `nooffline` diagnostic URL. The cloud browser's URL security policy blocked access to Chromium's internal service-worker page, so that profile's stale preview could not be resolved. No then-current published-app screenshot or completed interactive upgrade check was claimed. The `6a7cf14` images above came from its passing isolated CI run; multi-client and existing-profile update recovery still need separate verification. This profile was not rechecked for `b4c63a3`.

## Earlier published app

[Published Start](start-published-desktop.jpg) and [published All apps arrow](all-apps-published-desktop.jpg) were captured from the public [GitHub Pages app](https://ferroui.com/WindowsPhone/) after [run 45081854486](https://github.com/FerroUI/WindowsPhone/actions/runs/45081854486) deployed runtime [`528c982686ee765476bb2f8cbeaf6c9d857819b4`](https://github.com/FerroUI/WindowsPhone/commit/528c982686ee765476bb2f8cbeaf6c9d857819b4). The page's `data-phone-build` attribute confirmed `528c982`; the corrected arrow rendered and one native All apps click opened the app list, followed by Start returning to the tile screen. These unmodified JPEGs are 1363 × 936 pixels and preserve the browser session's phone profile.

`start-published-mobile.jpg`, `start-desktop.png` and `phone-history.png` are older application captures retained for the existing documentation. The earlier `cec2d1d` captures and historical `73dc39f` SDK captures are listed above; the `b4c63a3` and `6a7cf14` mobile images retain their earlier revision attribution.

## Earlier full refinement verification

The following unmodified PNGs were copied from the [`ui-verification` artifact of run 45080425037](https://github.com/FerroUI/WindowsPhone/actions/runs/45080425037/artifacts/14777585472), built from runtime [`4f1b5d51191718f0ae6301de206a042ef825b8f3`](https://github.com/FerroUI/WindowsPhone/commit/4f1b5d51191718f0ae6301de206a042ef825b8f3). Chromium rendered the packaged FerroUI WebAssembly application. Desktop captures use a 1280 × 936 CSS viewport; mobile Start uses 390 × 844 at device-pixel-ratio 2. Native pointer and keyboard interaction produced the captured states; the test data is fictional and isolated from the public browser profile.

| Repository image | Artifact source | Captured state |
| --- | --- | --- |
| [Mobile Start](start-refined-mobile.png) | `13-start-mobile-dpr2.png` | Mobile layout at 2× device-pixel ratio. |
| [Word](refinement-word.png) | `office-word-persisted-and-pinned.png` | A formatted native paragraph survives reload and reopens through its pinned document tile. |
| [Excel](refinement-excel.png) | `office-excel-formula-and-worksheets.png` | Native cells and separate worksheets after a range-formula edit. |
| [PowerPoint](refinement-powerpoint.png) | `office-powerpoint-slide-and-notes.png` | A native slide and its persisted speaker notes. |
| [Files](refinement-files.png) | `files-native-copy-move-rename.png` | A local folder after native copy, move and rename actions. |
| [Volume](refinement-volume.png) | `system-volume-expanded-muted.png` | Expanded ringer/media controls with independent levels and stable media mute. |
| [Live folder](refinement-folder.png) | `start-folder-expanded-children.png` | Miniature live content and expanded hittable child tiles. |
| [Lock over a draft](refinement-lock-draft.png) | `system-lock-covers-office-draft.png` | The lock screen covers a still-unsaved Office form. |

The full artifact contains 72 captures, native inspection JSON and `results.json` for 33 passing browser check groups, with zero recorded console/runtime errors. Its ZIP SHA-256 is `897b245141ae3a98bf8dde2e9c5280646fb4f73d8dc9fdc0ecb0cf6e9d49946d`. [Recorded verification](../parity.md#recorded-verification) describes the workflow coverage and remaining limitations. A screenshot demonstrates the visible state at capture time; persistence, interactions and import/export are established by the corresponding assertions rather than by the image alone.

## Final icon correction

[The forward-arrow capture](refinement-forward-arrow.png) is an unmodified local native Skia capture of the subsequent one-line All apps icon correction (`Icon::Next` → `Icon::Forward`). The capture-only renderer completed successfully. The subsequent `528c982` release passed 48 Rust domain/state tests, two host-boundary tests, packaged module linking and its native first-frame check, then deployed successfully. The public capture above confirms the released glyph. This evidence is separate from the 33-group browser suite above; those earlier images retain the prior glyph wherever the footer is visible.
