# Screenshot provenance

The images in this directory are application captures. They are not original Windows Phone reference screenshots and do not establish pixel equality with original hardware.

## Published app

`start-published-desktop.jpg` and `start-published-mobile.jpg` were captured from the public [GitHub Pages app](https://ferroui.com/WindowsPhone/). They preserve the browser session's phone profile. `start-desktop.png` and `phone-history.png` are earlier application captures retained for the existing documentation.

## Refinement baseline

The following unmodified PNGs were copied from the `ui-verification` artifact of [run 45076430807](https://github.com/FerroUI/WindowsPhone/actions/runs/45076430807), built from runtime [`cbee88221ea04a2bb7bf8c98429440f6fe20b12e`](https://github.com/FerroUI/WindowsPhone/commit/cbee88221ea04a2bb7bf8c98429440f6fe20b12e). Chromium rendered the packaged FerroUI WebAssembly application at a 1280 × 936 CSS viewport. Native pointer and keyboard interaction produced the captured states; the test data is fictional and isolated from the public browser profile.

| Repository image | Artifact source | Captured state |
| --- | --- | --- |
| [Word](refinement-word.png) | `office-word-persisted-and-pinned.png` | A formatted native paragraph survives reload and reopens through its pinned document tile. |
| [Excel](refinement-excel.png) | `office-excel-formula-and-worksheets.png` | Native cells and separate worksheets after a range-formula edit. |
| [Files](refinement-files.png) | `files-native-copy-move-rename.png` | A local folder after native copy, move and rename actions. |
| [Volume](refinement-volume.png) | `system-volume-expanded-muted.png` | Expanded ringer/media controls with independent levels and stable media mute. |

The full artifact contains 66 captures, native inspection JSON and `results.json` for 30 passing browser check groups. [Recorded verification](../parity.md#recorded-verification) describes the workflow coverage and remaining limitations. A screenshot demonstrates the visible state at capture time; persistence, interactions and import/export are established by the corresponding assertions rather than by the image alone.
