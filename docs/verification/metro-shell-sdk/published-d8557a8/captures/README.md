# Published main: four browser capture pairs

These eight original files preserve four requested appearance checks from main commit `d8557a8a576c34c82dff761ba99ebe82f4f3059c`, [browser run 45964709343](https://github.com/FerroUI/WindowsPhone/actions/runs/45964709343), artifact `15183619231`. The existing run reports **112 passed checks, 198 captures, and no errors**. Its 406,000ms elapsed value covers the whole suite and is not a responsiveness measurement.

The independently verified ZIP SHA-256 is `6143525949e98c0136f18149ae6b3a455013dc0e71f0c90e99f5ab1d1c0cbb9a`. Every PNG and inspection JSON here matches its downloaded ZIP member byte for byte. Nothing was cropped, re-encoded, or reserialized. [Provenance and per-file hashes](provenance.json) include the exact selected control geometry, existing relevant assertions, and review boundaries. The parent evidence directory owns aggregate results and manifests.

## What the four frames show

| Original frame and inspection | Observed appearance and retained geometry |
| --- | --- |
| [Settings, 320px, light](metro-settings-320-light.png) · [inspection](metro-settings-320-light.json) | The applications pane, five application rows, secondary labels, and bottom navigation glyphs are readable. `SettingsPivot` is x0/y64/320×622 with selected index 1. Its applications header starts at x20; the adjacent system header starts at x218 and extends into the right clip while its visible text remains readable. The pane scroll region starts at y128. |
| [Office, 320px, dark](metro-office-320-dark.png) · [inspection](metro-office-320-dark.json) | The places pane shows phone, OneDrive, Office 365, and OneNote, with yellow folder glyphs and visible app-bar/navigation commands. `OfficePivot` is x0/y64/320×562 with selected index 1. Places/recent headers remain enabled and hit-testable; the pane scroll region starts at y128. No row-text overlap was observed. |
| [Weather, pending progress](metro-progress-weather-moving.png) · [inspection](metro-progress-weather-moving.json) | The 400px phone interior is shown within a 1280×936 desktop frame. `WeatherProgress` is x460/y383/360×4, aligned to the 20px content inset. The inspector contains five 4×4px dots. Loading text is x460/y403/360×22 and current-location text starts at x460/y449. The final-candidate footer alignment remains present. |
| [Action Center, chronological groups](action-center-chronological-groups.png) · [inspection](action-center-chronological-groups.json) | Outlook precedes Messaging and Phone; Newest mail precedes Earlier mail inside Outlook. Headers, bodies, time labels, and close glyphs are visible. The three group grids share x458 and width364, with y254/484/619 and heights210/115/115. |

## Comparison with the final candidate

Local Git objects confirm that main `d8557a8` and candidate `d515348c699c22110def2dded8f7b8aea4b9cd03` have the same complete source tree, `ad870fb6a20b630070b6b9e0287dd0eec665f22c`. Selected stable inspection properties for these four frames match the earlier candidate artifact exactly. The comparison covers the controls listed in provenance, not every element or every pixel. Clock labels and moving-dot positions are not cross-run equality requirements.

No concrete appearance regression was observed in this limited four-frame review. The Settings/Office layout and Action Center group geometry remain consistent with the [final candidate hub/notification review](../../candidate-d515348/hubs-notifications/README.md); Weather retains the alignment described in the [final candidate progress review](../../candidate-d515348/progress/README.md).

## Scope and qualifications

These are FerroUI controls rendered by the browser build. “Native” in inspection types or existing check names describes that control/input path. This review does not establish original Windows Phone hardware, font, raster, timing, or complete OS parity.

The existing suite reports passing assertions for narrow hub headings, the Weather pending-operation progress lifecycle, and notification chronology/dismissal cancellation. The four still-frame pairs retained here do not independently demonstrate full motion sequences, group swipes, two-finger clearing, private-lock cleanup, or native device-provider notification cancellation. The Action Center data in this fixture is document-backed. Device-provider behavior remains covered by its separately scoped evidence.

The progress PNG and inspection JSON were sampled at separate instants during animation; their exact dot positions need not agree. The historical assertion name containing “original five-dot motion” is preserved verbatim in provenance as test output and does not turn this review into an original-device comparison. No additional test run or manual deployed-site interaction was performed for this archive.
