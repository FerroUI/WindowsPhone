# Screenshot provenance

The images in this directory are application captures. They are not original Windows Phone reference screenshots and do not establish pixel equality with original hardware.

## Current browser verification

The following unmodified PNGs come from the [`ui-verification` artifact of run 45138808865](https://github.com/FerroUI/WindowsPhone/actions/runs/45138808865/artifacts/14802170105), built from deployed runtime [`b4c63a36ed7d7d35168085409033711a61951103`](https://github.com/FerroUI/WindowsPhone/commit/b4c63a36ed7d7d35168085409033711a61951103). Chromium rendered the packaged FerroUI WebAssembly application in isolated test fixtures. These are not captures from a newly inspected public browser profile. Desktop images are 1280 × 936 pixels; mobile Start uses a 390 × 844 CSS viewport and a 780 × 1688 backing capture at device-pixel-ratio 2.

| Repository image | Artifact source | Captured state |
| --- | --- | --- |
| [People contact](host-people-vcard.png) | `people-native-vcard-contact.png` | An actual vCard imported through the production Wasm callback, with native contact share/copy commands exposed. |
| [Calendar series](host-calendar-series.png) | `calendar-native-icalendar-series.png` | An actual local-time iCalendar series imported through the production Wasm callback, with explicit series share/copy commands. |
| [Storage Sense](host-storage.png) | `host-native-storage-observations.png` | Native Settings displays controlled origin-usage/quota observations from the production observer and exposes an explicit persistence command. The values are fixture data, not host disk measurements. |
| [Device notification permission](host-notifications.png) | `host-native-notification-permission.png` | Native Settings displays the injected unrequested permission state and an explicit request command. No actual permission request or notification delivery occurred. |
| [Start](host-start-desktop.png) | `01-start-desktop.png` | Native desktop Start layout in the current packaged runtime. |
| [Mobile Start](host-start-mobile.png) | `13-start-mobile-dpr2.png` | Native Start at 2× browser density in the current packaged runtime. |

The full suite passed **49 check groups**, produced **100 captures**, and recorded **zero console/runtime errors** in **148.364 seconds**. The original [`results.json`](../verification/b4c63a3-results.json) has SHA-256 `6d29304ef7632f0188cbe18c53490a13ac759d0207f626d1c0617cd729f78755`; the complete artifact ZIP SHA-256 is `f004f37fa1ec121d1aedebe3b2581be86d42a8046200c5b1ac71ee9666938f2c`. Both the [build](https://github.com/FerroUI/WindowsPhone/actions/runs/45138808865/job/137571455356) and [Pages deployment](https://github.com/FerroUI/WindowsPhone/actions/runs/45138808865/job/137575350649) jobs succeeded. The duration measures suite execution, not application startup or frame rate.

All six images were visually inspected for coherent layout and reachable native commands. Their associated assertions establish the tested byte interchange, menu availability and observation behavior. They do not show OS share-sheet delivery, system file pickers, notification permission prompts, actual device battery readings or external-player playback. Local Linux unit verification and the unexecuted Windows/macOS/mobile boundaries are recorded in [the current host-integration verification](../parity.md#earlier-host-integration-verification-b4c63a3) and [local results](../verification/b4c63a3-local.json). No new cloud-browser interaction was performed in this pass; existing-profile upgrade recovery and original-device pixel parity remain unverified.

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

`start-published-mobile.jpg`, `start-desktop.png` and `phone-history.png` are older application captures retained for the existing documentation. Current mobile evidence is the `b4c63a3` capture above; the `6a7cf14` mobile image retains its earlier revision attribution.

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
