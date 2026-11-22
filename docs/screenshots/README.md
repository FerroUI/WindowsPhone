# Screenshot provenance

The images in this directory are application captures. They are not original Windows Phone reference screenshots and do not establish pixel equality with original hardware.

## Published app

[Published Start](start-published-desktop.jpg) and [published All apps arrow](all-apps-published-desktop.jpg) were captured from the public [GitHub Pages app](https://ferroui.com/WindowsPhone/) after [run 45081854486](https://github.com/FerroUI/WindowsPhone/actions/runs/45081854486) deployed runtime [`528c982686ee765476bb2f8cbeaf6c9d857819b4`](https://github.com/FerroUI/WindowsPhone/commit/528c982686ee765476bb2f8cbeaf6c9d857819b4). The page's `data-phone-build` attribute confirmed `528c982`; the corrected arrow rendered and one native All apps click opened the app list, followed by Start returning to the tile screen. These unmodified JPEGs are 1363 × 936 pixels and preserve the browser session's phone profile.

`start-published-mobile.jpg`, `start-desktop.png` and `phone-history.png` are older application captures retained for the existing documentation. The current mobile evidence is the isolated browser-suite capture described below.

## Full refinement verification

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
