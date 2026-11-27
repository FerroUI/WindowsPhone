# Metro Panorama and shell implementation record

Date: **27 November 2026**. Target: **Windows Phone 8.1 Update (2014)**.

This pass continues the [previous shell/SDK release](../metro-shell-sdk/README.md), whose published runtime is `d8557a8a576c34c82dff761ba99ebe82f4f3059c`. Its starting repository revision is `888af935a88813d240b897f038857debc04dbac8`. The framework remains pinned to `ec86b9c0204b8dee243eb300135cb0834f0a84b3`.

## Implementation scope

| Area | Change | Evidence boundary |
| --- | --- | --- |
| Panorama SDK | A cached native Carousel template with independent title, background and item layers; cyclic selection; finite vertical section viewports; next-header preview; native keyboard and touch arbitration; reduced-motion fallback. | Public Microsoft architecture and preview guidance establish the structure. Title metrics, parallax rates and the native completion curve remain adaptations. Equal-width vertical sections are supported; wide horizontal sections are not yet implemented. |
| People | All, favorites and supported device contacts use the shared Panorama. Search has a separate native editor; contact detail/search Back returns to the correct section. Local lists retain independent grouped-list positions. | Existing local/device data and provider actions are preserved. The original retired social feeds are not recreated. iOS smoke observes the real Panorama and device pane, not a renamed stand-in for the former root editor. |
| Mini application bar | Original 30-reference-unit collapsed extent, equivalent to 25 logical units; existing native command controls, expanded labels and shared dismissal; expansion overlays a stable Panorama viewport. | MiniSize is a documented source constant. Landscape relocation and exact original expansion timing remain incomplete. |
| Held Start/Apps preview | A gesture-owned, bounded neighbor image follows the live page before release; rejected pans settle together; committed slides preserve normal tile ink while suspending input. A valid visible preview-to-live Start handoff suppresses only that visit's decorative tile entrance. | A transient native tree uses an isolated model/session and is disposed after capture. No source route, lifecycle or persistent document is changed to obtain the image. This is an application rendering strategy, not the original OS page cache. |
| Activity progress | A separate native builder preserves the public Toolkit wrapper's 250 ms linear entry and 750 ms linear exit. Dots continue until exit completes; interrupted transitions reverse safely. Weather and Podcasts retain real operation slots. | The source is the public WP7 Toolkit. The durations are fades, not delays. Native range/opacity/visibility bindings remain application-owned. |
| Office navigation | Separate transient recent/places/search positions survive reconstructed pages; native layout clamps restored offsets; new queries reset search; document replacement invalidates old bindings. | This does not expand the supported Office document model or claim desktop Office fidelity. |

## Source facts and adaptations

The [Panorama overview](https://learn.microsoft.com/en-us/previous-versions/windows/apps/ff941104(v=vs.105)), [architecture](https://learn.microsoft.com/en-us/previous-versions/windows/apps/ff941126(v=vs.105)) and [guidelines](https://learn.microsoft.com/en-us/previous-versions/windows/apps/hh202912(v=vs.105)) describe independent layers, a single wrapping title, sections that track the finger, cyclic navigation, a 24-pixel next-section preview, vertical lists within narrow sections and separate editor pages. [MiniSize](https://learn.microsoft.com/en-us/previous-versions/windows/apps/hh312716(v=vs.105)) defines the 30-unit reference extent.

The implementation scales reference values by `400 / 480`. The Panorama title font of 170 reference units, 174-unit title band, −34-unit top adjustment and ordinary title/background travel ratios of 0.16/0.40 are explicit adaptations, not recovered private WP8.1 template constants. Native Carousel drag completion retains its own threshold, duration range and easing. A substitute font is not original Segoe WP raster evidence.

The native fractional panel owns its cyclic offset inside a finite clipping viewport. An additional horizontal ScrollViewer would feed its bounded offset back into the panel at wrap and during resize, so the template does not introduce that competing owner. For two-section cycles, the alternate visible slot uses the pinned framework's [native Pivot redirect pattern](https://github.com/wieslawsoltes/FerroUI/blob/ec86b9c0204b8dee243eb300135cb0834f0a84b3/src/FerroUI.Controls/pivot/pivot_header_manager.rs) and [RedirectVisual](https://github.com/wieslawsoltes/FerroUI/blob/ec86b9c0204b8dee243eb300135cb0834f0a84b3/src/FerroUI.Base/rendering/composition/redirect_visual.rs). It follows the existing live compositor subtree, including paint and child-animation changes, without a duplicate control tree or bitmap cache. Native tests inspect actual server source identity and invalidation; rendered pixel evidence remains a separate gate.

The activity state machine is adapted from Microsoft's [PerformanceProgressBar.cs](https://github.com/microsoftarchive/WindowsPhoneToolkit/blob/4e4fd5d36c769c004f7ccc7daf78a5ffb8ef18df/Microsoft.Phone.Controls.Toolkit/ProgressBar/PerformanceProgressBar.cs) and [WP7 Generic.xaml](https://github.com/microsoftarchive/WindowsPhoneToolkit/blob/4e4fd5d36c769c004f7ccc7daf78a5ffb8ef18df/Microsoft.Phone.Controls.Toolkit.WP7/Themes/Generic.xaml) at `4e4fd5d36c769c004f7ccc7daf78a5ffb8ef18df`, under MS-PL. The earlier delayed-show description is corrected: the wrapper begins fading immediately. Initial template application does not animate. Reduced motion, cancellation generations and cross-host lifecycle cleanup are recreation policies.

Snapshot isolation applies to mutations and host requests through the supplied temporary model and session. Registered native Rust renderers are trusted code; a callback that independently captures global or external state is not sandboxed. The preview copies displayed badge values and the already-observed native application catalog, but no DeviceStore authority, provider action tokens, persistence callback or live host service callback. Native application enumeration is not triggered by mounting a preview. An explicit reactive-owner image policy permits cached/bundled images without starting host loads; separately evaluated live owners are unaffected. Application-private session state is not implicitly cloned.

## Verification status

Implementation validation is in progress. Final source identity, platform runs, rendered results and publication hashes will be recorded here after their actual gates complete. Historical totals from the preceding release do not qualify this source.

The frozen local eight-package graph passed **277 tests, zero failed or ignored**, with package integration and doc tests included. The [native log](local-native-tests.txt) records the complete run; the [JavaScript log](local-javascript-tests.txt) records **85 passing boundary tests**, zero failures or skips. These are local gates, separate from the rendered and platform qualification still required below. The native command was:

```sh
CARGO_BUILD_JOBS=1 CARGO_PROFILE_TEST_DEBUG=0 \
cargo test -p phone-app-office -p phone-app-people -p phone-app-podcasts \
  -p phone-app-weather -p phone-controls -p phone-sdk -p phone-state -p phone-system \
  --no-fail-fast -- --test-threads=1
```

Archived local text logs use one final newline. The final run did not set `CARGO_INCREMENTAL` or `RUST_BACKTRACE` and did not use `--nocapture`. Package totals, including integrations, were Office 4, People 3, Podcasts 11, Weather 2, controls 58, SDK 63, state 60 and system 76. The committed-slide regression covers paired and unavailable-capture paths, late default/cancel Button insertion, actual keyboard/tab input, normal tile opacity and restoration of newer underlying values. An earlier 276-pass checkpoint preceded that regression and is not added to these totals.

Mounted tests use the framework's assembly-scoped headless session, keeping its dispatcher alive until native compositor/TLS teardown while isolating each dispatched application and disposing its windows and render owners.

Native tests inspect actual controls, selection, layout, focus/input admission, scroll restoration and lifecycle ownership. Explicit native animation clocks exercise deterministic endpoints; the browser fixture separately observes real Chromium touch input, compositor geometry and rendered pixels. Headless preview bitmaps establish allocation dimensions and ownership only, not image-content fidelity.

No original Windows Phone guest was booted or captured in this work. These changes do not certify 100% OS, visual or interaction parity. See [current feature differences](../../parity.md), [the design evidence](../../metro-design.md) and [the original-emulator record](../../original-emulator.md).
