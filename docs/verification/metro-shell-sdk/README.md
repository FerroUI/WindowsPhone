# Metro shell and SDK follow-up

Date: 27 November 2026. Base source: [`0f7010ef41b23056ab54b7efeea182a0860ac0d2`](https://github.com/FerroUI/WindowsPhone/commit/0f7010ef41b23056ab54b7efeea182a0860ac0d2). FerroUI remains pinned to `ec86b9c0204b8dee243eb300135cb0834f0a84b3`.

[PR #6](https://github.com/FerroUI/WindowsPhone/pull/6) merged as [`d8557a8`](https://github.com/FerroUI/WindowsPhone/commit/d8557a8a576c34c82dff761ba99ebe82f4f3059c). Its complete tree is identical to accepted candidate `d515348c699c22110def2dded8f7b8aea4b9cd03` and CI checkout `75d8b175260015c8eb61c82ab87d525bbba15221`: `ad870fb6a20b630070b6b9e0287dd0eec665f22c`. The [identity record](candidate-d515348/identity.json) and [candidate CI record](candidate-d515348/ci.json) preserve the source and execution bindings.

## Implemented behavior

- Start/Apps release keeps a bounded outgoing image next to the live destination on one paging clock. Input stays blocked until completion; lock, resize, cancellation and replacement release both transforms and the image. Rejected swipes still settle back. Native page identity includes the document epoch, so restoring while already on Start mounts a fresh capture owner; ordinary revisions retain the current page.
- Action Center uses newest-item chronology across local and native groups. One-finger dismissal and parallel two-finger clear retain document identity and recheck live native authorization. Group dismissal also retains captured package/key ownership; clear-all obtains current clearable keys at gesture completion. Locked private content cannot be cleared through its hidden controls.
- `progress(label)` provides native determinate range semantics and a five-dot indeterminate strip. Its compositor ownership ends when hidden, disabled, detached or motion-suppressed. Weather and Podcasts expose actual pending operation state. Weather's loading/status/location footer shares its existing content inset, and its active loading label uses the application's 18 px status style; this is application consistency, not a measured original-font claim.
- Settings and Office use native cyclic Pivot panes with separate finite vertical scrollers and fixed headers. Office search keeps native text editing outside the Pivot and Back restores its selected hub section.
- `with_route_pivot_count` is a backwards-compatible SDK extension. Explicit route callbacks take precedence over legacy session callbacks, including zero; custom swipe opt-in remains separate. Settings, Office and People detail surfaces no longer change hidden hub sections.

## Source and fidelity boundaries

The five-dot trajectory is adapted from the Microsoft Windows Phone Toolkit **WP7** `PerformanceProgressBar` template at `4e4fd5d36c769c004f7ccc7daf78a5ffb8ef18df`, with MS-PL attribution in [NOTICE](../../../NOTICE.md). It is public Toolkit evidence, not a claim to the private WP8.1 control implementation. The existing 350 ms shell-paging curve is an application policy, not a newly measured original-device value. Held-drag neighboring-page preview, full layered Panorama, original-selector physics and original font/raster comparisons remain open. Reconstructing an Office hub after search retains selection, but not every destroyed ScrollViewer offset.

The progress adaptation implements the inner dot storyboard; the Toolkit wrapper's 250 ms entry delay and 750 ms exit fade remain absent. Determinate and vertical presentation, pixel snapping and reduced-motion static dots are FerroUI adaptations. Explicit effective visibility, detachment, disabled state and motion policy govern animation lifetime; arbitrary viewport clipping or opaque overlay occlusion is not a separate suspension signal. Browser compositor readback verifies translated dot positions and geometry. Animated opacity cadence is covered by native timeline tests, because the inspection API exposes native rather than compositor-effective opacity.

## Verification

New native regressions exercise page choreography/lifetime, actual shell pointer ownership, mixed local/native notification gestures, authorization changes, document replacement, native progress range/templates/animation lifetime, and actual app/SDK route policies. Rendered scenarios cover paired paging, progress while a host response is held pending, notification chronology and gestures, narrow dark/light hubs, independent pane scrolling, keyboard selection, Office search and reduced motion. Existing verification assertions are retained. The existing workflow additionally runs the Office package's tests.

**All seven candidate gates passed.** The browser job recorded **469 Rust tests passed, one ignored**, **80 JavaScript tests passed**, and **112 rendered interaction checks with 198 captures**, with no recorded browser errors. These totals belong to one job; local and native repetitions are not added into a larger unique-test count. The [complete candidate index](candidate-d515348/README.md), [unmodified browser results](candidate-d515348/browser-results.json), and [compiled-file manifest](candidate-d515348/compiled-artifact.json) retain their exact scope.

| Gate | Accepted candidate result |
| --- | --- |
| [Browser and desktop workflow](https://github.com/FerroUI/WindowsPhone/actions/runs/45959413873) | Browser checks and complete Windows, macOS and Linux host builds passed. |
| [Android ARM application/HOME packages](https://github.com/FerroUI/WindowsPhone/actions/runs/45959413886) | Host contracts, package boundaries and both package flavors passed. |
| [Android mounted UI](https://github.com/FerroUI/WindowsPhone/actions/runs/45959413903) | Emulator geometry, HOME/Back behavior, unread badge, notification admission/revocation and call ownership checks passed. |
| [iOS Simulator](https://github.com/FerroUI/WindowsPhone/actions/runs/45959414018) | Build/bridge checks, seven device checks, six URL activations and four actual navigation taps passed. |

The browser traces retain 19 paired native observations toward Apps and 21 toward Start. All paired native positions remain one 400-logical-pixel viewport apart and both input paths remain disabled during the settle. Subsequent compositor readbacks agree. The first paired observation in each direction still reports the previous completed compositor bounds, so these observations do not establish zero-frame release latency. The [motion review](candidate-d515348/motion/README.md) also records turnstiles, rapid Back, lock cancellation, reduced motion and final image/transform cleanup.

The new document-restoration regression reaches the production shell input and capture path. It fails before the document-aware page key and passes afterward; the complete local system suite then passes 68 tests. The headless bitmap backend provides dimensions without rendered pixels. Separate browser observations supply compositor evidence. The [preflight history](preflight/README.md) preserves this failing-to-passing sequence, the two earlier failed browser runs, their fixture corrections and all relevant source boundaries.

## Publication

The merged source `d8557a8a576c34c82dff761ba99ebe82f4f3059c` is undergoing the fresh main-branch [browser build, full interaction suite and Pages publication](https://github.com/FerroUI/WindowsPhone/actions/runs/45964709343). Publication and public-file identity will be recorded after that run succeeds. Native/mobile verification remains attached to the identical accepted candidate tree; the normal main workflow skips those duplicate gates.

## Evidence index

| Review | Preserved observations and qualifications |
| --- | --- |
| [Shell motion](candidate-d515348/motion/README.md) | Eight complete traces and four original PNG/inspection pairs; native state, completed compositor geometry, input ownership and cleanup. |
| [Progress](candidate-d515348/progress/README.md) | Original Weather/Podcast captures and inspections, real pending operation state, moving/static dots, restored content insets and tooltip cleanup. |
| [Hubs and Action Center](candidate-d515348/hubs-notifications/README.md) | Native panes, 320/400-pixel layouts, dark/light themes, separate Office search, grouped notifications and private-lock cleanup. |
| [Android](candidate-d515348/android/README.md) | Original native notification and navigation evidence; this emulator scenario does not perform the new horizontal notification gestures or paired Start/Apps paging. |
| [iOS](candidate-d515348/ios/README.md) | Original Simulator observations and captures. Settings is the device-services leaf, and Wallet is an entrance frame. The already-corrected blank-navigation interpretation is not repeated as a defect. |
| [Preflight](preflight/README.md) | Source-specific failed runs, local test logs, startup diagnostics and the subsequent corrections, kept distinct from acceptance. |

Per-review provenance and the [candidate checksum manifest](candidate-d515348/manifest.json) identify preserved originals. Artifact PNG/JSON/text members are copied byte for byte. GitHub job logs are connector-decoded UTF-8 text with normalized newlines; their hashes identify the archived text rather than GitHub's original log ZIP. Source presence, successful compilation, captured-image existence and sampled geometry do not establish original-device raster equality.

## Earlier baseline

The preceding base release's [main browser publication run](https://github.com/FerroUI/WindowsPhone/actions/runs/45908389279) completed successfully. Its downloaded, unmodified UI results record **102 checks**, **182 captures**, `passed: true`, `errors: []`, and **378786 ms** of suite execution. Those totals belong to `0f7010e`; they are not results for this follow-up. Suite duration does not measure application performance.
