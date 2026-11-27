# Metro shell and SDK follow-up

Date: 27 November 2026. Base source: [`0f7010ef41b23056ab54b7efeea182a0860ac0d2`](https://github.com/FerroUI/WindowsPhone/commit/0f7010ef41b23056ab54b7efeea182a0860ac0d2). FerroUI remains pinned to `ec86b9c0204b8dee243eb300135cb0834f0a84b3`.

## Implemented behavior

- Start/Apps release keeps a bounded outgoing image next to the live destination on one paging clock. Input stays blocked until completion; lock, resize, cancellation and replacement release both transforms and the image. Rejected swipes still settle back.
- Action Center uses newest-item chronology across local and native groups. One-finger dismissal and parallel two-finger clear retain document identity and recheck live native authorization. Group dismissal also retains captured package/key ownership; clear-all obtains current clearable keys at gesture completion. Locked private content cannot be cleared through its hidden controls.
- `progress(label)` provides native determinate range semantics and a five-dot indeterminate strip. Its compositor ownership ends when hidden, disabled, detached or motion-suppressed. Weather and Podcasts expose actual pending operation state.
- Settings and Office use native cyclic Pivot panes with separate finite vertical scrollers and fixed headers. Office search keeps native text editing outside the Pivot and Back restores its selected hub section.
- `with_route_pivot_count` is a backwards-compatible SDK extension. Explicit route callbacks take precedence over legacy session callbacks, including zero; custom swipe opt-in remains separate. Settings, Office and People detail surfaces no longer change hidden hub sections.

## Source and fidelity boundaries

The five-dot trajectory is adapted from the Microsoft Windows Phone Toolkit **WP7** `PerformanceProgressBar` template at `4e4fd5d36c769c004f7ccc7daf78a5ffb8ef18df`, with MS-PL attribution in [NOTICE](../../../NOTICE.md). It is public Toolkit evidence, not a claim to the private WP8.1 control implementation. The existing 350 ms shell-paging curve is an application policy, not a newly measured original-device value. Held-drag neighboring-page preview, full layered Panorama, original-selector physics and original font/raster comparisons remain open. Reconstructing an Office hub after search retains selection, but not every destroyed ScrollViewer offset.

The progress adaptation implements the inner dot storyboard; the Toolkit wrapper's 250 ms entry delay and 750 ms exit fade remain absent. Determinate and vertical presentation, pixel snapping and reduced-motion static dots are FerroUI adaptations. Explicit effective visibility, detachment, disabled state and motion policy govern animation lifetime; arbitrary viewport clipping or opaque overlay occlusion is not a separate suspension signal. Browser compositor readback verifies translated dot positions and geometry. Animated opacity cadence is covered by native timeline tests, because the inspection API exposes native rather than compositor-effective opacity.

## Verification

New native regressions exercise page choreography/lifetime, actual shell pointer ownership, mixed local/native notification gestures, authorization changes, document replacement, native progress range/templates/animation lifetime, and actual app/SDK route policies. Rendered scenarios cover paired paging, progress while a host response is held pending, notification chronology and gestures, narrow dark/light hubs, independent pane scrolling, keyboard selection, Office search and reduced motion. Existing verification assertions are retained. The existing workflow additionally runs the Office package's tests.

Exact candidate and publication results will be recorded after execution. Source presence is not a passing-test or original-device-parity claim.

The preceding base release's [main browser publication run](https://github.com/FerroUI/WindowsPhone/actions/runs/45908389279) completed successfully. Its downloaded, unmodified UI results record **102 checks**, **182 captures**, `passed: true`, `errors: []`, and **378786 ms** of suite execution. Those totals belong to `0f7010e`; they are not results for this follow-up. Suite duration does not measure application performance.
