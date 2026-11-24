# Metro controls, motion and Outlook verification

Target: **Windows Phone 8.1 Update (2014)**. Framework remains pinned to `db2e8859cd4b67bac870839ea70503b8babf541f`. This record separates local regressions, actual rendered observations, native compilation and original-emulator attempts. It does not certify original-device pixel or feature parity.

## Current application source: 41d7e3c

Source [`41d7e3c`](https://github.com/FerroUI/WindowsPhone/commit/41d7e3ceddb2d317edf3f1b8f22a7358d38bb7e6) includes the native dialog-caption correction and bounded iOS startup diagnostics. Its fresh browser build passed and was deployed to [the public app](https://ferroui.com/WindowsPhone/). The prior compiled C artifact is not reused. [Source boundaries](source-boundaries.json) bind each implementation and replay to its actual Git objects; the Android follow-up changes the harness while preserving E's complete application runtime inputs.

| Gate | Exact source | Observed result | Evidence |
| --- | --- | --- | --- |
| Browser build and contracts | `41d7e3c` | Fresh optimized build; 379 Rust passes, zero failures, one ignored ordinary case; 67 JavaScript passes | [Browser report](browser-41d7e3c.json), [build job](https://github.com/FerroUI/WindowsPhone/actions/runs/45387122214/job/138420827726) |
| Rendered browser and deployment | `41d7e3c` | All 80 groups passed; 146 captures; `errors: []`; offline reload/input passed; Pages deployment succeeded | [Original results](browser-41d7e3c/results.json), [visual review](browser-41d7e3c/visual-review.md), [public HTTP identity](publication-http.json) |
| Linux x86_64 | `41d7e3c` | 60 Rust executions including the separate private-D-Bus MPRIS check; six Python passes; complete native executable linked | [Desktop report](desktop-41d7e3c.json) |
| macOS ARM64 | `41d7e3c` | 51 Rust passes; six Python passes; complete native executable linked | [Desktop report](desktop-41d7e3c.json) |
| Windows x86_64 | `41d7e3c` | 52 Rust passes; four Python passes and two POSIX-only skips; Clang/MSVC compatibility and PowerShell parse passed; complete native executable linked | [Desktop report](desktop-41d7e3c.json) |
| Android ARM64 packages | `41d7e3c` | Both App/HOME APKs and embedding passed; manifests, JNI and 16 KiB alignment verified | [E package and failed-runtime record](android-41d7e3c.json), [package run](https://github.com/FerroUI/WindowsPhone/actions/runs/45387122271) |
| Android API 36 runtime follow-up | `b65eb4e`, identical E runtime | 47 Python passes, 111 JVM assertions, seven Rust cases and 24 AOSP hashes; 12 mounted routes, six device phases, five bootstrap phases; actual Answer/active/end and exact authority restoration passed | [F Android report](android-b65eb4e.json), [runtime run](https://github.com/FerroUI/WindowsPhone/actions/runs/45389593350) |
| iOS UIKit and Simulator | `41d7e3c` | Both UIKit variants compiled; physical-target Swift type-check and Simulator Objective-C syntax-check; 15 Python and 14 Rust executions; five provider checks and six exact native URL activations with the same process/launch/scene | [E iOS report](ios-41d7e3c.json), [run](https://github.com/FerroUI/WindowsPhone/actions/runs/45387122381) |

The browser suite took 265.630 seconds in total; that is test duration, not an application performance measurement. Fourteen selected PNG/native-snapshot pairs were individually inspected and retained byte-for-byte. The narrow discard/cancel commands now have visible captions within actual enabled 132 × 45 targets. Saved recipient, subject and multiline body visibly survive restoration; native Pivot, light/dark controls and application-bar expansion also passed. The [review](browser-41d7e3c/visual-review.md) explicitly retains compact date-picker long-month clipping and oversized settings-title clipping as limitations.

Public HTML and service-worker responses both returned HTTP 200 with build marker `41d7e3c` after deployment. This confirms publication identity; the rendered behavior record comes from the exact-source CI build. No additional local or public-site browser interaction pass is implied. Native desktop links do not certify desktop GUI, device permissions or hardware. Android/iOS runs establish the observed private-emulator/Simulator contracts; physical carrier/RF, OEM, signing and original Windows Phone device comparison remain separate.

The iOS pass does not establish a fix for the earlier startup failures. Its first permissions queries all returned, `main_view.end` was recorded at 1.851546 seconds, and all seven original route screenshots were reviewed. That phase time is not first-visible-frame timing. The [startup investigation](ios-startup-investigation.md) preserves the two distinct prior failure signatures and the new per-query/console diagnostics without attributing an unproven cause.

Counts are executions within each stated gate. Overlapping local, browser, native and feature-configuration suites are not summed into a unique total.

### Android first-boot baseline follow-up

The E ARM64 package gate passed, while [E’s API 36 run](android-41d7e3c.json) failed restoring its original HOME holder. Twelve native app/launcher/HOME route checkpoints had passed; notification, SMS and Telecom phases had not run and receive zero credits. The recorded original holder was Google’s temporary SDK setup app. The native log explicitly rejected restoring it because its MAIN/HOME component no longer qualified after first-boot setup. This was not an observed application-role-provider failure.

The isolated follow-up [`b65eb4e`](https://github.com/FerroUI/WindowsPhone/commit/b65eb4eb68d2da11729843a7407ad918cff269e7) changes only the test harness, validator and workflow test hook. Before recording a restoration baseline it observes completed provisioning, the unchanged current user, an authoritative HOME holder, matching enabled MAIN/HOME activity resolution and a second consecutive stable sample. It retains raw query results within a 120-second bound. It neither sets provisioning values nor guesses a launcher package, and exact role restoration remains mandatory. [Source boundaries](source-boundaries.json) prove production runtime inputs are unchanged from E.

[The follow-up passed](android-b65eb4e.json): 12 mounted routes, six device phases, five notification/cold-Start bootstrap phases and the native ringing/Answer/active/end lifecycle. HOME, Dialer, SMS and notification access were restored exactly. Both accepted baseline samples already met the provisioning and eligible-HOME contract; no pending setup sample occurred, so this run does not establish a measured wait through E's setup race. All 20 original screenshots were reviewed. The retained native log establishes Answer and call transitions; the emulator GSM listing returned only `OK` and is not used as call-state proof. Both F APKs contain exactly E's native library bytes. ARM64 packaging remains separately attributed to E.

### Native interaction invariants

- Native ToggleSwitch dragging, keyboard/automation and checked-state callbacks remain native. Structural templates are retained across palette changes, preventing a transient loss of `PART_MovingKnobs`; a separate geometry regression covers the compact switch mask under layout rounding.
- Native Pivot pan/snap owns its viewport. Phone integration disables inherited selection auto-scroll, which previously shifted content after native snapping. A real native Window/compositor regression observes forward pan, forward wrap, reverse wrap, selected headers and composed body alignment.
- App bars retain one expansion/dismissal owner, at most four primary icons, separate ellipsis, expanded captions and bounded overflow. Dismissal occurs before the command callback so Word formatting can open its replacement menu.
- Dialog decisions use actual native text commands with explicit labels. The final browser check requires visible caption text and composed geometry in addition to an accessible name, including the 320-unit phone layout. Cancel, validation, Back and saved/unsaved document ownership remain separate.
- Rendered inspection advances to the latest completed compositor readback, matching native hit testing. Readback absence is not replaced by logical bounds; harness readiness waits for actual mounted content.

### Intermediate source and replay

Source [`1a20466`](https://github.com/FerroUI/WindowsPhone/commit/1a2046672e0e82ffd3fe978fdae8ff4c0b5c8366) passed [all desktop link/test gates](desktop-1a20466.json) and [Android packaging/runtime gates](android-1a20466.json). Its browser build had 376 Rust passes and 59 JavaScript passes; the [third rendered attempt](history/third-attempt.json) passed 75 groups with 143 captures but failed two harness lookups. Header selection had required an automation name from a presentation peer; Device readiness had observed parent text before the actual native commands acquired readbacks.

The source-identical app replay at [`98a013d`](https://github.com/FerroUI/WindowsPhone/commit/98a013dbc6a1d0a96fc0691050bd69b32780ae7d) corrected those lookups. It passed the complete Metro touch/wrap/header/keyboard, light/dark, reduced-motion and Outlook sequences. Its [fourth attempt](history/fourth-attempt.json) still failed overall: 79 successful groups consumed 253.709 seconds, and the complete run exhausted the former 270-second suite budget during offline reload readiness. The retained original failure frame shows settled Start and enabled, hit-testable hardware navigation. The next harness permits 360 seconds overall with a seven-minute CI step, retains every individual timeout/offline assertion and reports global exhaustion distinctly.

Visual inspection of that replay also found the destructive confirmation defect: named, usable automation targets had zero visible captions, and discard inherited a save-shaped icon. The original [PNG](history/98a013d-outlook-unsaved-back-confirmation.png) and [snapshot](history/98a013d-outlook-unsaved-back-confirmation.json) remain preserved. This finding required a runtime correction and fresh application build, not another replay of the old binary.

The C iOS source compiled both UIKit variants and passed portable checks twice, but both mounted-Start attempts failed. [Attempt 1](history/ios-1a20466-attempt1.json) stopped after `snapshot.permissions` before main-view installation. [Attempt 2](history/ios-1a20466-attempt2.json) recorded no Rust-entry phase and a SpringBoard screenshot; its console PID line was written after the original failed-process observation. These are distinct, unresolved signatures with zero route/provider/activation pass credits. The [startup investigation](ios-startup-investigation.md) records the exact timestamps and boundaries. The added diagnostics neither relaunch the app nor extend its route deadlines.

## Earlier checkpoint: c9923b2

Source [`c9923b2`](https://github.com/FerroUI/WindowsPhone/commit/c9923b2730e9fde63d4c822452f69ad46af30959) passed native desktop, Android and iOS gates. Its browser build succeeded but its integrated rendered suite failed, so Pages deployment was skipped. These platform results must not be presented as a successful browser release.

| Gate | Observed result | Evidence |
| --- | --- | --- |
| Browser build and contracts | 374 Rust passes, one deliberately ignored ordinary test; 59 JavaScript passes; browser compilation passed | [Failed run](https://github.com/FerroUI/WindowsPhone/actions/runs/45370106537), [retained second attempt](history/second-attempt.json) |
| Rendered browser | 31 groups passed and 76 captures recorded before the native template panic; integrated result failed | [Unmodified results](history/c9923b2-results.json) |
| Linux x86_64 | 60 Rust test executions including the separate private-D-Bus MPRIS check; six Python packaging passes; complete native link passed | [Desktop record](desktop.json) |
| macOS ARM64 | 51 Rust passes; six Python packaging passes; complete native link passed | [Desktop record](desktop.json) |
| Windows x86_64 | 52 Rust passes; four Python packaging passes and two POSIX-only skips; Clang/MSVC compatibility and PowerShell parse passed; complete native link passed | [Desktop record](desktop.json) |
| Android ARM64 packaging | Both App/HOME variants and embedding; 111 JVM assertions, 24 AOSP source-hash checks, eight Python package checks, seven Rust cases; protected manifests/JNI/16 KiB ELF and ZIP alignment | [Android record](android.json), [package run](https://github.com/FerroUI/WindowsPhone/actions/runs/45370106379) |
| Android API 36 runtime | Native HOME role/reentry/Back, SMS badge, notification admission/rendering/revocation and incoming call Answer/active/end teardown passed | [Android record](android.json), [runtime run](https://github.com/FerroUI/WindowsPhone/actions/runs/45370106370) |
| iOS UIKit and Simulator | Both standard and optional carrier bundles compiled; physical-iPhone Swift type-check and Simulator Objective-C syntax-check passed; five provider checks and six native URL activations passed | [iOS record](ios.json), [run](https://github.com/FerroUI/WindowsPhone/actions/runs/45370106416) |

Counts are executions within their stated gates, not a sum of globally unique tests. Overlapping local and CI suites are not added together. Native desktop compilation/linking does not establish desktop GUI behavior, physical hardware, permissions or distribution signing. The native symlink-refusal test is `cfg(unix)` and runs on Linux/macOS, accounting for their extra shared-service case. Linux's GIO child case is invoked through its parent; the logs do not distinguish every optional-runtime early-return branch.

### What the failed browser run established

Office formatting, application-bar geometry/dismissal, incoming turnstiles/reduced-motion checks and native switch drag/persistence passed. Four later failures were investigated independently:

- The native Games Pivot selected its second pane while its body remained one viewport to the right. The [original PNG](history/c9923b2-metro-controls-failure.png) and [snapshot](history/c9923b2-metro-controls-failure.json) preserve the actual geometry.
- Replacing accessibility palette styles removed the native ToggleSwitch drag template during a styling pass, producing `Could not find control 'PART_MovingKnobs'`. This was reproduced in a mounted native regression before correction.
- Outlook's immediate post-navigation snapshot preceded its new composition readback. The [original retained frame](history/c9923b2-outlook-drafts-failure.png) and [snapshot](history/c9923b2-outlook-drafts-failure.json) contain the exact unsaved multiline text. The corrected assertion waits for that real TextBox and value.
- Start wallpaper similarly had no readbacks in an immediate sample but all 17 real image planes in the subsequent failure capture. Its assertion now waits for every fixture plane and retains the common-origin, coverage, opacity, scrolling and persistence checks.

No test substitutes logical layout coordinates for unavailable composition readbacks. Failed records remain failed after their diagnoses are understood.

## Earlier native runtime observations (c9923b2)

### Android

The retained runtime report binds 12 mounted-geometry checkpoints, six native device phases and five bootstrap phases to the exact source. Incoming-call observations progress from ringing sequence 49 → 50, through active sequence 57 → 59 with the same call identity, to ended sequence 61 → 62 with an empty call list. The original native log records the actual accept request, ANSWERED→ACTIVE transition and REMOTE/NORMAL disconnection. Foreground-service type changes 4 → 0 and the CallStyle notification is released. The emulator GSM list returned only `OK`; it is not used as proof of active-call state.

The current user and HOME/DIALER/SMS/notification-listener authority are restored to their original values. Twenty original PNGs were produced and nine representative images were inspected. The selected files and native call histories/log are retained below. This is emulator evidence, not RF/carrier, physical-device or universal OEM qualification. The earlier attempt's call failure remains unexplained because its overwritten log did not retain the accept interval; the later passing run does not invent a production telephony fix.

### iOS

All seven original route captures were inspected. Five actual device-provider checks cover capabilities, read-only brightness and Contacts create/update/delete, with no owned contact left after cleanup. Six exact URI activations were accepted natively and observed with the same PID, launch UUID and active scene; Home was restored. The 134.315-second XCTest duration measures the test, not startup or animation performance. `prompt-handler.accepted=0` counts accepted prompts, not URL activations; native activation outcomes are recorded separately.

Brightness remained 0.5 when a 0.67 write was refused. No call or message was sent. A Simulator pass does not establish physical brightness/torch behavior, carrier entitlements/default roles, NFC/payment or distribution signing. Route captures wait for mounted content and are not frame-timing measurements.

## Original captures and integrity

The current [browser manifest](browser-41d7e3c/manifest.json) retains 14 reviewed PNG/native-snapshot pairs and the original results. The [iOS E manifest](ios-manifest-41d7e3c.json) retains all seven route captures and their native process/activation evidence; the [Android F manifest](capture-manifest-b65eb4e.json) retains 123 original entries, including readiness samples, nine reviewed representative PNGs and the full call log. The [Android E failure manifest](capture-manifest-41d7e3c.json) preserves its distinct first-boot restoration failure. Each manifest records artifact entries, byte counts and SHA-256 values.

[The earlier B capture manifest](capture-manifest.json), [C manifest](capture-manifest-1a20466.json) and failed browser/iOS attempt reports retain their own source boundaries. Images, snapshots and native logs were copied without editing. Reports link to the originating CI runs and identify artifact ZIP digests. Relative repository paths replace temporary workspace paths only through explicit retention maps; original observations remain unmodified. Decoded job logs are identified separately from archive entries. The nested `.gitattributes` disables text normalization for this evidence directory so original CRLF bytes remain intact.

Selected Android images: [active call](captures/android-device-active-call.png), [ended call](captures/android-device-ended-call.png), [Action Center](captures/android-device-action-center.png), [Messaging](captures/android-device-messaging.png).

Selected iOS images: see the exact entries in the [capture manifest](capture-manifest.json), including Start, Settings, Messages and Wallet. Seven native observation JSONs and six URI requests are retained alongside the images.

## Local regressions and earlier attempts

[Local evidence](local.json) records the initial 372-pass native run and 55-pass JavaScript run, the later application-bar/switch-mask focused checks, and the mounted palette replacement regression. Counts overlap and retain their distinct source/phase boundaries. Local Chromium was blocked before application startup by denied ProcessSingleton socket creation; zero local rendered checks or captures are credited.

The [first browser attempt](history/first-attempt.json) at `1f578a9` is preserved separately, including its [unmodified results](history/1f578a9-browser-results.json). Its Windows native job was canceled when superseded, not reported as a compiler failure. The diagnostic Android-only follow-up was also canceled; the subsequent `c9923b2` runtime result has its own evidence.

The [C iOS startup investigation](ios-startup-investigation.md) retains both initial-mount failures and their different phase evidence. Neither attempt credits a route, device or activation pass; subsequent source outcomes are recorded independently.

## Original Windows Phone reference

The [emulator guide](../../original-emulator.md) and [attempt index](original-emulator.json) record actual Linux and Windows inventory, authentic Microsoft package provenance, installation attempts and boot status. The official offline ISO supplied and installed the original phone VHD, but its configurator suspended installation with a restart requirement and no XDE executable; no guest boot was observed. Host Hyper-V availability, installed files and a successful setup are distinct from an observed guest boot. No original device comparison is inferred from this project's Android or iOS simulator runs.

The [Metro research matrix](../../metro-design.md) records exact original source constants and distinguishes WP7/WP8 Toolkit material, WP8.1 requirements, selected video excerpts and uninspected leads. The [feature matrix](../../parity.md) retains the remaining Panorama, outgoing/feathered transitions, picker, typography, application-service and OS-runtime gaps.
