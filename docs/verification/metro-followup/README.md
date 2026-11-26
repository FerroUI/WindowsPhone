# Metro controls and iOS startup follow-up

This follow-up repairs the two browser failures and the iOS startup stall
recorded on `a4a7b851048a151db66f9f4404b0c5ed8672c812`, and completes the native
picker interaction and application-bar work from the handoff.

Review: [PR #4](https://github.com/FerroUI/WindowsPhone/pull/4).
The [implementation and harness diff](https://github.com/FerroUI/WindowsPhone/compare/a4a7b851048a151db66f9f4404b0c5ed8672c812...dc9e2a94d206944023169a789539c02307dd201a)
contains the 41 changed source, test, workflow and dependency entries without this dossier.
The retained historical evidence is unchanged; fresh results are recorded
separately. These verification runs do not publish a Pages deployment. The later
approved main release, including fresh main CI and direct public asset verification,
is recorded in [the `81fabb3` publication report](published/81fabb3/README.md).

## Source boundary

| Item | Revision |
| --- | --- |
| Previous main | `a4a7b851048a151db66f9f4404b0c5ed8672c812` |
| Runtime implementation | `c2f9645e0d7ddc2e60278f3f116fe08e77694ce5` |
| Touch harness correction | `5c572ce3534659f753eb6e02f6be16a1130bda8f` |
| Shared input acquisition | `2dc6f31db9039600e7a6e7c5337deba17d97b88c` |
| Complete input-path correction | `4f10a3d6c1ce4adc53aaf8621a9f33244bb1f491` |
| Picker/shell contact ownership | `f6c3d20d865018ccb960b7720b1f6c4b873c6519` |
| Native reopen and XCTest correction | `c96a4f0043bd80137997fcd34dbb4b407414af00` |
| Hidden-part test lookup | `e430b2141eae143cca1440f6b0ade31646659332` |
| Initial bounded native scrolling | `8749c8a60cb7eb854f641d772eb662c2afaf8064` |
| Realized-window and boundary-offset follow-up; current verification source | `dc9e2a94d206944023169a789539c02307dd201a` |
| CI checkout for `dc9e2a9` | `f8bed7d59a6b16b6df2caaea01d88b615c39ae0d` |
| Tree of the verification head and checkout, identical | `85db8c20df19ca40e08ebef89cb086acd72eac60` |
| Previous FerroUI pin | `db2e8859cd4b67bac870839ea70503b8babf541f` |
| Initial native reset dependency | `cd9aeaef1fb6a80f513d577d345fac6483077966` |
| Initial bounded-scroll dependency | `75bdfc3c39a5bd73b5e27651441cd29bce86eebb` |
| FerroUI dependency under verification | `ec86b9c0204b8dee243eb300135cb0834f0a84b3` |

The CI checkout is the temporary merge commit, not the PR head itself. Their
Git trees are equal, so the tested source files match the successor head.
The retained captures identify their exact source commits. A later commit
confined to this dossier does not change the inventoried runtime or harness
files; its documentation must not be mistaken for a new capture of the app.
The final evidence-only commit uses `[skip ci]` after all selected code results
are frozen. Its parent is the tested `dc9e2a9` code commit, and its complete diff
must remain under `docs/verification/metro-followup/`. The PR body identifies that
documentation head separately. A subsequent approved merge/publication must
use an explicit clean message, without inheriting the documentation skip marker;
`[verify-ui] [verify-native] [verify-mobile] [verify-android-ui]` requests the
rendered browser, full desktop builds, ARM/iOS and mounted Android UI checks on
main. The approved `81fabb3` merge used all four markers.

## What failed and what changed

### Native picker columns

The retained browser snapshot gives the hour and minute selectors the same
rectangle: `x=516, y=143, width=124, height=624`. The minute selector covered the
hour selector; the hour center failed the native hit test.

The template now initializes each field host to column `index * 2` and each
separator to `index * 2 + 1`. The native time presenter assumes the fixed hour
and minute columns already exist; it only moves the optional seconds and
period fields. Date presenters remain free to reorder their columns for the
active culture.

Mounted Rust tests reuse the popup across 320- and 400-unit roots, 12- and
24-hour clocks, and both seconds configurations. They check separate columns
and the native hit target at every selected cell. Browser assertions independently
check positive bounds, ordering, non-overlap, expected fields, and reachability.

### Rapid taps and native drag ownership

The native recognizer emits `DoubleTapped` on the second press and suppresses
the corresponding `Tapped` release. The previous selector behavior therefore
lost rapid second taps and could select a recycled container prematurely.

The picker now owns a bounded tap transaction. Press focuses and expands the
wheel; a valid release stages the value held by the original pressed container.
Movement past the platform tap tolerance, holding, capture loss, changed scroll
offset, viewport changes, unload, recycled tags, or a hidden/unreachable cell
cancel that transaction. The adapter consumes the item-level tap gestures before
the native panel can apply the same selection again.

All wheels share contact ownership. Two or three overlapping contacts suppress
selection until every contact has ended, including when another contact lands
while a native drag is held. Both ordinary capture and gesture-recognizer capture
are tracked. Weak references avoid retaining native pointers or recycled items.

The complete shell adds a second ownership boundary. Overlay popup presses
follow the logical owner route, but cancellation reports direct capture loss
through visual ancestors. The shell can therefore observe a picker press
without receiving the cancellation through its `PhoneScreen` handler. A stale
shell contact makes the next press look like overlapping shell input; the shell
captures it and disables native gesture recognition.

The fourth browser attempt isolates this failure: Chromium delivers a trusted
canvas press followed by eight moves across 123.75 pixels at the minute wheel,
yet all eight native frame samples retain `00` at the same selected-cell position.
The [touch trace](attempts/4f10a3d/picker-touch.json) ends with failure cleanup,
not a successful drag release. The original standalone mounted picker test
does not attach the shell controller and cannot detect this cross-tree lifetime
problem. The integration correction assigns the whole picker surface the
existing `phone-native-input` ownership marker so that its native controls own
the entire contact sequence.

The new integration regression mounts the production shell with a native time
picker. It requires both touch presses to traverse the shell's routed handler,
cancels the first contact, then records eight native wheel offsets before
releasing the next contact. It checks for no stale shell contacts, no shell
capture on press, native recognizer capture during scrolling, a changed staged
minute, and an unchanged committed value and route. Native inertia stays
enabled. The regression asserts ownership and behavior rather than the presence
of a particular class string.

The native `ScrollViewer` still owns capture, scrolling, inertia, and snapping.
The native presenter still owns value validation, confirmation, and cancellation.
Routing tests check rapid multi-click counts, cancellation, cross-wheel and
same-wheel multi-touch, held drags, recovery, and unchanged owner value before
confirmation. Browser tests use real pointer/touch input and retain normal
scroll physics.

The fifth attempt verifies that ownership correction. Its
[trusted touch trace](attempts/f6c3d20/picker-touch.json) contains a fresh press,
all eight movement samples, and the matching release. The selected minute moves
from `00` to `01`, the cell moves during the contact and snaps back to center,
and the owner remains unchanged. The production-shell Rust regression passes
in the same run. A later Back/reopen assertion exposes a separate retained-state
failure, preserved in the [native snapshot](attempts/f6c3d20/picker-reopen-failure.json)
and [capture](attempts/f6c3d20/picker-reopen-failure.png).

### A fresh native selection transaction on reopen

In that failed reopening, the owner still holds `07:00`, while its retained
presenter displays the canceled `09:01` selection. Native flyout opening assigns
the owner's committed value to the presenter's styled property. Because that
property already equals `07:00`, the equality check suppresses the callback
that normally initializes the wheels. DatePickerPresenter has the same issue
and also retains a private staged `sync_date`.

The dependency correction adds an internal `reset_selection()` operation to
each native presenter. Every native flyout opening invokes it after assigning
the owner value and before opening the popup. Date reset restores the complete
committed date, including its time and offset, before calling native selector
initialization. Time reset calls the existing native initialization directly.
The same presenter and panel instances remain in use. Native date ranges,
hidden fields, formatting, minute/second increments and 12/24-hour conversion
remain in their existing initialization paths.

The dependency is reviewable in
[FerroUI PR #1456](https://github.com/wieslawsoltes/FerroUI/pull/1456).
Its three commits descend directly from the previous WindowsPhone framework pin,
so the consumer takes only the ten-file picker and scrolling correction and
its standalone tests, without later unrelated framework main changes.
The [dependency source record](framework-dependency.json) identifies its exact
commit, parent, tree and changed-file hashes.

This is an intentional correction of the prior reopen behavior. The framework's
existing date dismissal test explicitly recorded retained canceled values;
the new expectation starts a fresh transaction from the owner and verifies
acceptance without another edit. Consumer regressions independently exercise
the mounted phone presentation. A manual phone-layer wheel reset was rejected
because it could not faithfully restore the native date presenter's private
staged state and formatting context.

### Bounded wheel extent and coherent scroll metrics

The passing DatePicker matrix and failed noon TimePicker case in `e430b21`
isolate a second native defect. The time presenter initializes noon as PM
correctly. A non-looping period panel then reports two item pitches of extent
while its physical viewport can be much taller. The scroll host computes a
maximum offset of `max(extent - viewport, 0)` and feeds zero back into the
native panel, changing its staged period to AM. Short year ranges and their
last values share this bounded-wheel geometry.

The panel now reports `(item_count - 1) * item_pitch + viewport_height` as its
bounded public extent. The legal offset range is therefore exactly zero through
the last item's center, even when the viewport is smaller than one item.
The physical viewport remains unchanged, preserving the native recognizer's
one-to-one pixel conversion. Its private item extent remains unchanged for
keyboard endpoints. A bounds-size notification publishes the real viewport
after arrangement and on later resizes.

The presenter also needs to publish a complete logical scroll state. Each
viewport or extent setter synchronously coerces its offset and propagates
through the owning `ScrollViewer`, whose two-way binding can coerce it again.
The correction snapshots viewport, extent and offset, suppresses feedback
through all three publications, and rechecks that the child state and identity
are still current. Reentrant viewport or offset observers can publish newer
state; the newer snapshot wins. After publication, a final clamp is forwarded
once only if the presenter's effective offset actually changed and the current
child still differs. A child can own transient negative or positive boundary
progress while the parent's effective offset remains at its limit. Metric
publication alone must not erase that progress. Identical values and callbacks
from replaced children do not restage the native selection.

Mounted framework regressions require PM and the last year to survive initial
layout, viewport growth/shrinkage, clipped viewports and item-pitch changes
without selection notifications. They inspect the actual selected realized
cell, its center, native keyboard endpoints and scroll-gesture endpoints.
Those unit tests synthesize recognizer events; the consumer's existing real
input and trusted browser touch checks remain independent requirements.
A generic mounted scroll test observes actual child offset writes, requiring
zero transient writes, one genuine final clamp, and preservation of newer state
during both viewport and offset observer reentry.

### Resize rebinding and logical-child offset ownership

The first bounded-scroll pin passes the complete consumer suite, including noon
PM and native reopen. The broader framework Linux suite nevertheless finds two
real runtime defects. Its [failure record](attempts/8749c8a/framework-runtime-failure.json)
and [full compressed log](attempts/8749c8a/framework-linux.log.gz) retain both
failures independently of the unrelated conventions check described below.

For the bounded year range 2020–2024, increasing viewport height from 200 to
480 changes the realized selection window and grows the child pool. The retained
containers still carry their previous values. Year 2024 stays in slot 3 with a
cell rectangle at `y=180, height=40`, centered at 200 instead of the new viewport
center 240. Measurement now compares the previous above/below window counts and
realized child count, then refreshes item bindings when either changes. The
selected year moves to the correct slot and the existing center/notification
assertions remain unchanged. Initial scroll invalidation still occurs only at
initialization.

The existing carousel backward-wrap test exposes the second defect. A 120-pixel
backward swipe in a 320-pixel viewport gives the logical child an offset of
`-0.375`; the presenter's effective offset remains coerced to zero. The initial
coherent-publication correction wrote zero back unconditionally, erasing the
child's release progress and selecting index 0 instead of the last index 2.
The follow-up records the effective offset before publication and forwards the
final clamp only when that effective value changed. It needs no carousel-specific
exception.

The new generic `child_owned_overscroll_survives_unchanged_scroll_metrics` test
covers negative progress, positive progress beyond the maximum, metric-only
changes, an explicit owner offset change, and a genuine final shrink/clamp. The
previous generic reentrancy test, bounded-wheel centering test and existing
carousel assertion are unchanged. Local consumer and framework test type checks
both pass on the published follow-up bytes; they do not execute these bodies.
The [follow-up validation record](local/bounded-scroll-followup.json) identifies
the source hashes and commands. The final Linux run passes both previously
failing cases and all five other targeted regressions: native date and time
dismiss/reopen, bounded keyboard/gesture endpoints, coherent metric publication
with reentry, and child-owned boundary offsets. The
[framework record](verified/dc9e2a9/framework-ci.json) retains the seven exact
pass lines and suite totals; the [complete log](verified/dc9e2a9/framework-linux.log.gz)
preserves their full execution context.

### Application-bar command slots

The primary `UniformGrid` now stretches inside a centered group bounded by
`command_count * 76`. Captions can appear without changing the commands' slots
or identities. Mounted tests cover one through four commands at widths 320,
375, 400, and 480, comparing collapsed and expanded geometry. Browser tests
also assert centered, equal-width slots and stable positions after expansion.

### iOS first-frame stall

The previous simulator console ended at
`snapshot.permissions.location.begin`, immediately before the synchronous
CoreLocation class authorization query in native-host construction. The owned
process was alive, but no mounted first frame appeared. That identifies the
blocking boundary; the old diagnostic attempt did not prove a deeper locationd
IPC cause.

Snapshots now read a cached authorization observation. A lifecycle-owned
`CLLocationManagerDelegate` is constructed on a deferred UIKit-queue turn and
reports the initial and subsequent authorization states without requesting
permission or starting location updates. Generation checks discard stale queued
construction; suspension detaches the delegate, releases the manager, and clears
the cache. Callbacks check the current observer and manager identities.

Before the first observation, `PermissionState::Unknown` is represented as
`checking` in the SDK. Applying an unknown snapshot atomically clears stale
location data. Explicit user-requested permission and location operations are
also driven from the authorization callback and retain their existing timeout
and cancellation ownership.

The fresh-simulator gate waits for a real observed state and requires `Prompt`.
It rejects permanently unknown, unsupported, and pre-granted fixtures, so a
placeholder state cannot make the startup check pass.

The `5c572ce` simulator run captures this transition directly. Its
[first mounted Start frame](attempts/5c572ce/ios-start-before-location.json)
is ready at sequence 2 while location is still `unknown`. The
[provider report](attempts/5c572ce/ios-device-tests.json) then observes `prompt`,
and the [next warm capture](attempts/5c572ce/ios-wallet-after-location.json)
retains that resolved state. The process and launch identities remain the same.
The first frame is therefore independent of completion of authorization
observation; the permission remains truthfully pending until its callback.

The fifth attempt reaches a ready Start frame at sequence 1 and subsequently
observes `Prompt`, with all six provider checks passing. It then fails at a
different boundary: the XCTest URL driver exceeds its 120-second bootstrap
deadline. Retained diagnostics show the XCTest method entering approximately
151 seconds after launch, during failure capture. No warm URL request was
issued, so this attempt does not count any warm activation as passed. Its
[complete failure record](attempts/f6c3d20/ios-ci.json) is retained.

Only the driver's bootstrap allowance increases to 240 seconds. The existing
provider, route and in-test deadlines, native process/launch identity checks,
and six ordered URL assertions are unchanged. Failure cleanup now requests
termination of the owned `xcodebuild` launch child before sampling, waits at
most ten seconds, escalates if needed, reaps it and clears its PID. Success
retains the finished-marker handshake. Three real-process tests exercise the
production cleanup function, including a child that ignores termination and
preservation of the original failure status. The
[local validation record](local/ios-bootstrap-cleanup.json) includes exact
unchanged-assertion hashes and distinguishes launch-child cleanup from
independently unverified simulator process termination.

The final `dc9e2a9` simulator run again captures the startup separation directly:
the first mounted Start observation is sequence 1 with authorization `Unknown`;
all six later route observations report `Prompt`. The seven observations share
native PID 24677, launch identity `E2036900-4059-4594-A63F-FC87D449EA97` and scene
identity `E6CC8F52-78CE-45D0-B5FB-8E3299BA5C06`. Their sequences are
`1 → 195 → 231 → 240 → 245 → 249 → 254`. The native provider check also observes
`Prompt`. All six exact warm URI assertions pass.

The [first screenshot](verified/dc9e2a9/ios-start.png) contains a simulator
Apple Intelligence notification over the top of the live Start tiles. The
[restored Start screenshot](verified/dc9e2a9/ios-home-restored.png) has no
notification overlay. **Correction, 26 November 2026:** the earlier description
of blank navigation icons was a mistaken image interpretation. All three glyphs
are visible. Independent decoding of 16 retained captures across `81fabb3`,
`dc9e2a9`, `8749c8a` and `e430b21` found the same opaque navigation-row pixels.
The [correction and its limits](../feature-completion/ios-navigation-correction.md)
supersede the visual assessment in the earlier capture-comparison and publication
records; their original bytes and provenance remain preserved. This establishes
icon visibility in those frames. Actual button interaction is covered by the
new qualification work and must be attributed to its own revision.
The lifecycle gate is not a complete iOS chrome or original-device parity test.

### Pixel-rounded game layout

The retained Minesweeper board was correctly arranged at 360 units with eight
columns and 2-unit gaps. Its tracks are two 44-unit columns followed by six
43-unit columns; the third column starts at 92. The old assertion expected the
fractional uniform-offset value 90.5.

The replacement assertions check distinct cells, finite positive bounds,
pixel-aligned edges, tracks differing by at most one physical pixel, exact gaps,
shared row and column edges, complete board coverage, and reachable empty cells.
Positive and negative geometry fixtures exercise those contracts. The retained
board passes the corrected grid contract; the retained overlapping picker still
fails the strengthened picker contract.

### Browser input setup and settled coordinates

The first PR run passed the original picker-overlap and game-grid checks, all
native contracts, and all mobile/desktop build gates. Its rendered suite still
failed two checks. Those failed artifacts are retained under
[`attempts/c2f9645`](attempts/c2f9645/manifest.json).

The Messaging failure snapshot shows the second fixture conversation instead
of the search form. The search handler cannot open a conversation. The helper
had waited for route publication but not the page's turnstile entrance. During
motion its coordinate calculation combines a transformed origin with logical
dimensions, providing a supported explanation for the wrong-row click.
The corrected helper waits for both page projections and the outgoing image
to detach, then reasserts the expected route before clicking. Search, forwarding,
contact selection, individual deletion, and bulk deletion remain real UI input.

The picker test omitted the explicit Chromium touch-emulation setup used by
the passing touch suites. The correction enables it with guaranteed cleanup
and independently records trusted pointer delivery. Cancellation must follow
a matching press; a fresh drag must deliver movement and release for the same
contact. Eight native geometry/value samples must show movement while that
contact is still down. The original changed-value, centered-snap, staged-data,
Back/reopen, and explicit-confirmation assertions remain required. The trace
is logged on both success and failure; no additional Rust change was made on
the basis of an undiagnosed browser timeout.

The first four PR attempts contain byte-identical WASM:
`942db1c6250956ed980c6ee92f8d98654088f165e79294d37cb88a527764fafc`.
Their harness corrections therefore exercise the same compiled runtime. Their
archive identities change because the generated index and service worker record
the new build identity. The later picker ownership correction changes runtime
source and requires a fresh compiled verification.

The `5c572ce` attempt stopped earlier at OneNote's new-note workflow, before reaching
the picker module. Its [failure record](attempts/5c572ce/manifest.json) shows the
unchanged OneNote hub instead of the editor. The same coordinate problem also
existed in the shared click helpers, so normal workflow input now acquires
coordinates centrally: require settled native page/picker/outgoing surfaces,
cross a rendered-frame boundary, and reacquire the same target with stable
bounds. Raw snapshots and explicit `rawControl` observations remain available
for deliberate interactions during animation.

The next run completed OneNote's entire create/edit/reload/pin/delete workflow,
then exposed the same bypass in Start-to-AllApps navigation. Its retained
native frame still has a turnstile attached at only `+0.542401` degrees.
Requiring a small angle would therefore still accept a moving page. The final
correction makes stable acquisition the default for ordinary `control()` calls,
waits for completed entrances in navigation and static gallery captures, and
reacquires the calendar, alarm, picker, tile-menu, and alphabet-header fast
paths. The measured frame is included in the regression fixture.

AllApps still receives one native press and release, with its pressed frame
captured before release. Disabled-command probes, route cleanup while a
contact is held, midpoint observations, recorded gesture streams, and generic
capture timing retain their original semantics.

With those normal-input paths corrected, all five System refinement checks,
including Messaging search, forwarding, recipient selection, and deletion,
pass in `4f10a3d`. Its 87 checks shared with the first attempt take 326,983 ms
instead of 265,630 ms, and the newly passing Messaging workflow adds 6,609 ms.
The old total six-minute deadline expires at the first DPR2 density check.
The suite therefore gets a bounded eight-minute total budget and a nine-minute
workflow step, allowing failure capture and cleanup. Individual control waits
and every behavioral assertion retain their previous limits.

The successful cold `8749c8a` run spends 36m22s before entering rendered
verification, including a 22m12s WASM build. Adding the existing nine-minute
step allowance already exceeds the former 45-minute outer job before artifact
upload and cleanup. The outer browser job therefore increases to 60 minutes.
The inner eight-minute suite and nine-minute step remain unchanged. The
[measured budget](local/browser-cold-job-budget.json) and
[completed job steps](local/browser-cold-job-steps.json) retain the timing and
semantic workflow comparison; only the outer timeout scalar changes here.

### Accessing hidden native parts in the regression tests

The `c96a4f0` consumer run stops before the browser build: the two new mounted
reopen regressions use the existing visual-descendant helper to find hidden
date fields and the disabled seconds column. Those selectors exist in the
native template namescope, but a hidden `ScrollViewer` has not created its
visual content presenter. All 33 existing control tests pass; the two new
tests fail at lookup and do not complete their assertions. The
[failed gate record](attempts/c96a4f0/browser-ci.json) identifies exactly which
later checks were not reached.

The test-only correction observes the presenter's public, direct
`TemplateApplied` event before first opening and retains its actual namescope.
It updates the scope on later template applications and removes its handler
on drop. Each reopen resolves registered selectors from the current scope;
separate assertions still require the actual popup presenter and panel objects
to remain identical. Hidden fields stay hidden, the existing visible/hit-test
helper is unchanged, and every range, format, state and acceptance assertion
remains required.

The `e430b21` run validates that lookup correction: the full DatePicker matrix
passes. Its TimePicker matrix then exposes a separate layout defect before
any cancellation occurs. Opening the noon fixture expects `[12, 5, 30, 1]`
(12:05:30 PM), but the native period selector reports `[12, 5, 30, 0]` (AM).
The [gate record](attempts/e430b21/browser-ci.json) preserves this exact failure
and distinguishes it from the earlier missing-part test failure. No WASM build
or rendered browser checks execute in that attempt.

## Verification results

All WindowsPhone gates pass on code `dc9e2a9` with framework pin `ec86b9c`.
The exact app checkout tree equals the code head tree. All five framework
runtime jobs also pass, including the Linux workspace and detachable-component
checks. The overall framework result remains failed because of the separate,
pre-existing C++ conventions mismatch retained below.

| Gate | Result and coverage | Evidence |
| --- | --- | --- |
| App Rust contracts and JavaScript | 402 Rust passes, one ignored; 77 JavaScript passes | [Browser/build job](https://github.com/FerroUI/WindowsPhone/actions/runs/45873369913/job/140094179071) |
| Rendered browser | 93 checks, 163 captures, 358,511 ms; trusted touch cancellation/drag, Back/reopen, explicit acceptance/save, signed swivel, all five System workflows, DPR2/DPR1 and offline pass | [Results](verified/dc9e2a9/browser-results.json), [CI and artifact identities](verified/dc9e2a9/browser-ci.json) |
| Linux desktop | Portable/native contracts and full build/link pass; cold cache saved | [Job](https://github.com/FerroUI/WindowsPhone/actions/runs/45873369913/job/140094179406) |
| macOS desktop | Portable/native contracts and full build/link pass; cold cache saved | [Job](https://github.com/FerroUI/WindowsPhone/actions/runs/45873369913/job/140094179758) |
| Windows desktop | Portable/native contracts, PowerShell parsing and full build/link pass; cold cache saved | [Job](https://github.com/FerroUI/WindowsPhone/actions/runs/45873369913/job/140094179660) |
| Android ARM | Both package variants and embedding-library checks pass on attempt 1 | [Run](https://github.com/FerroUI/WindowsPhone/actions/runs/45873369940), [record](verified/dc9e2a9/android-ci.json) |
| Android mounted UI | Attempt 2 passes after the retained packaging-only HTTP 429 failure; 20 captures, real active call, notification access/revocation and all four restoration pairs | [Successful job](https://github.com/FerroUI/WindowsPhone/actions/runs/45873369928/job/140100751272) |
| iOS | Both UIKit bundles, portable/bridge checks, seven same-process observations, six warm URI activations and the observed `Unknown` → `Prompt` transition pass | [Run](https://github.com/FerroUI/WindowsPhone/actions/runs/45873369912), [record](verified/dc9e2a9/ios-ci.json) |
| Framework Linux | Main workspace: 36,093 passes, zero failures, 130 ignored across 348 harness invocations, including 6,060 Controls passes; resource-group tests: 60 passes; component tests: 1,119 passes; both component Clippy checks pass | [Job](https://github.com/wieslawsoltes/FerroUI/actions/runs/45873299369/job/140094080754) |
| Framework macOS, Windows, X11 and browser | All four jobs pass; macOS validates its native suite, with its whole-workspace step skipped by design | [Run](https://github.com/wieslawsoltes/FerroUI/actions/runs/45873299369) |
| Framework API gate | Preview-policy applicability skip; no signature comparison | [Job](https://github.com/wieslawsoltes/FerroUI/actions/runs/45873299369/job/140094082456) |
| Framework conventions | Fails on two stale generated C++ headers, independently reproduced on the unchanged merge base | [Attribution](local/framework-cpp-baseline.json) |

The rendered swivel assertions pass, and the restored-editor JSON/PNG is retained.
Their sampled angle arrays are asserted in memory and are not persisted in the
artifact. Desktop rows establish build/link and native adapter contracts, not
WindowsPhone desktop GUI captures. PR Pages deployment is skipped.

### Framework merge baseline

The current framework standalone run checks out merge
`50786219e3cce60940743823361866a2834325b0`, whose actual base parent is
`21720a379c059a6a73805bf583fdd0589c933969`. Its full tree differs from the
consumer's focused dependency pin. The [source comparison](framework-pr-source-comparison.json)
verifies that all ten edited files are byte-identical to that pin.

The current [standalone run](https://github.com/wieslawsoltes/FerroUI/actions/runs/45873299369)
repeats the failure on two stale generated C++ headers first retained in the
[previous conventions job](https://github.com/wieslawsoltes/FerroUI/actions/runs/45866630349/job/140071228208).
An isolated checkout of the unchanged
base fails the identical check. The generator, selection helpers, API model,
sample manifest and stored headers are byte-identical between base and merge;
the API-model check itself passes in CI. Base-only regeneration updates the
headers' API fingerprint, IDs and newer control declarations, after which that
specific check passes. The [attribution record](local/framework-cpp-baseline.json)
retains input hashes, exact source boundaries and command results.

This is a pre-existing mismatch on the newer framework merge baseline.
No generated binding changes enter the focused dependency pin. Later convention
steps were not reached by that job, and its failed status is retained separately
from the native runtime gates and WindowsPhone's own exact-pin verification.
The API job is a preview-policy applicability skip and compares no signatures.
macOS's native job skips its separate whole-workspace step by workflow design;
the full picker and generic-scroll bodies execute in the Linux workspace job.

### Retained Android radio failure and bounded rerun

The first mounted Android run on `8749c8a` reaches the incoming-call scenario
but never observes an active call. Its focused native call activity, screenshot
and UIAutomator tree agree with the injected Answer coordinates. Telecom
receives the request and records `SET_ANSWERED`; the radio interface acknowledges
`ANSWER` at `17:31:26.105 UTC`, then returns an empty `GET_CURRENT_CALLS` result
at `.113`. Android classifies the call as `INCOMING_MISSED` at `.154`.
The original assertion correctly fails after its unchanged 90-second bound.

The [first-attempt record](attempts/8749c8a-android-ui-1/manifest.json) retains the
native UI tree, observations, screenshots, modem responses and full compressed
radio/logcat history with both stored and decompressed hashes. It also records
four original/restored state equalities covering HOME, DIALER, SMS and
notification-listener access. This locates the observed failure below delivered
app input and the acknowledged answer request; the deeper emulator/radio cause
is not established.

One failed-job rerun uses the identical source and assertions in a fresh isolated
AVD. It passes, recording the same call identity from ringing through active and
a normal remote disconnect after deliberate emulator hangup. All four original
role/listener states are restored. Its [result](attempts/8749c8a/android-ci.json)
is recorded separately from the failed attempt. No app change, fabricated call
state, extra test retry loop or relaxed assertion follows from the failure.

The current `dc9e2a9` Android UI job encounters a different infrastructure failure:
Maven Central returns HTTP 429 during APK dependency downloads, before emulator
startup. The [packaging failure record](attempts/dc9e2a9-android-ui-1/manifest.json)
retains its full log and source-only artifact. The portable/native checks passed;
no Android UI or capture is counted as executed. Gradle exposes no Retry-After
header. One unchanged-source failed-job rerun is requested more than 140 seconds
after the terminal download failure. That rerun passes the complete mounted
emulator gate: 20 captures, the same call identity from ringing to active and
normal remote disconnect, fully visible notification labels, clearing on access
revocation, and byte-equal restoration of all four role/listener states. Its
[final record](verified/dc9e2a9/android-ci.json) identifies both attempts and
their separate artifacts. Android ARM also passes on its first attempt.

## CI and publication boundary

Same-repository pull requests now run the existing browser, native, Android, and
iOS gates. PR concurrency is isolated from main. Build jobs use read-only
repository/action permissions; the Pages artifact and deployment are excluded
from PR events. Pages and OIDC write permissions exist only on the deploy job.

The Windows native job now has a 60-minute limit. The
[previous 40-minute job](https://github.com/FerroUI/WindowsPhone/actions/runs/45494885471/job/138802617905)
was cancelled during Win32/Skia compilation; the full build and link gate
remains required.

This pre-merge verification left main and the published site unchanged. PR #4
was subsequently approved and merged; the fresh main checks and live publication
are recorded in [the `81fabb3` release report](published/81fabb3/README.md).

## Evidence and scope

- [Historical manifest](history.json) records the original browser and iOS run,
  artifact IDs, ZIP SHA-256 values, and retained file hashes.
- [Changed source hashes](source-files.json) identify every changed file in the
  tested implementation and verification harness.
- [Local check record](local-verification.json) separates successful type and
  harness checks from runtime tests that execute in CI.
- [First PR attempt](attempts/c2f9645/manifest.json) retains its two new browser
  failures and successful iOS, Android, and desktop evidence separately.
- [Second PR attempt](attempts/5c572ce/manifest.json) retains the early OneNote
  failure and the observed iOS `unknown`-to-`prompt` startup transition.
- [Third PR attempt](attempts/2dc6f31/manifest.json) retains the small-angle Start
  entrance failure and successful native and mobile results.
- [Fourth PR attempt](attempts/4f10a3d/manifest.json) retains the trusted
  cancel/drag trace, the total-suite deadline, passing Messaging workflows,
  and fresh native/mobile evidence.
- [Fifth PR attempt](attempts/f6c3d20/manifest.json) proves fresh native drag
  ownership, preserves the separate Back/reopen and XCTest bootstrap failures,
  and records the passing desktop, Android, DPR2, DPR1 and offline gates.
- [Sixth PR attempt](attempts/c96a4f0/manifest.json) retains the two hidden-part
  test lookup failures separately from the successful iOS and Android gates.
- [Seventh PR attempt](attempts/e430b21/manifest.json) retains the passing
  DatePicker matrix and the initial noon/period failure before browser execution.
- [Eighth PR attempt](attempts/8749c8a/manifest.json) retains all green consumer
  gates, the sole unchanged-source Android rerun, and the broader framework's
  two runtime failures that required the final resize/offset follow-up.
- [First Android UI attempt on `8749c8a`](attempts/8749c8a-android-ui-1/manifest.json)
  retains the acknowledged answer followed by an empty radio call list, its
  failed active-call assertion and verified cleanup separately from the rerun.
- [First Android UI attempt on `dc9e2a9`](attempts/dc9e2a9-android-ui-1/manifest.json)
  retains Maven Central throttling during packaging, before any emulator check.
- [Final verification](verified/dc9e2a9/manifest.json) identifies the successful
  consumer source, all platform results, selected raw captures, the complete
  framework Linux log and the separately attributed conventions failure.
- [Original overlapping picker](history/a4a7b85/shell-parity-failure.png) and
  [its native snapshot](history/a4a7b85/shell-parity-failure.json).
- [Original game board](history/a4a7b85/sdk-games-failure.png) and
  [its native snapshot](history/a4a7b85/sdk-games-failure.json).
- [Original iOS startup console](history/a4a7b85/ios-console.log) and
  [owned-process record](history/a4a7b85/ios-owned-process.json).

The evidence establishes the listed layout, interaction, startup, packaging,
and build gates. It does not certify complete Windows Phone device pixel or
motion parity. A prerequisite probe for the original emulator is not an
original-device UI comparison. Desktop build/link success does not imply a
desktop GUI capture. Simulator and emulator checks retain their own platform
and entitlement boundaries.
