# Metro shell and SDK preflight evidence

This directory preserves the investigation that led to candidate
[`d515348c699c22110def2dded8f7b8aea4b9cd03`](https://github.com/FerroUI/WindowsPhone/commit/d515348c699c22110def2dded8f7b8aea4b9cd03),
tree `ad870fb6a20b630070b6b9e0287dd0eec665f22c`. **At this preflight snapshot,
the candidate's full CI was pending. This is not an acceptance or publication
record.** Earlier passing native jobs and local tests do not turn either failed
browser run into a passing run.

All **128 files** in the retained local evidence inventory, plus the complete
**59-frame failing browser paging trace**, were copied without byte modification:
**129 evidence files / 2,563,995 bytes**, including **24 compressed logs** and
**10 selected original PNGs**. [manifest.json](manifest.json) records every
relative path, byte count and SHA-256, plus decoded size and SHA-256 for gzip
logs. Existing metadata and review records retain their original `target/`
paths; the equivalent archived file has the same relative path beneath this
directory. Full platform binaries, complete screenshot archives and xcresult
data remain identified by their original artifact metadata rather than being
duplicated here.

The byte-preservation claim applies to the retained files being archived here.
GitHub job logs were obtained as connector-decoded text and stored as UTF-8;
the patch writer normalizes CRLF line endings and adds a final newline. Their
recorded hashes identify that archived text and its gzip representation, not
the server's original log-ZIP bytes. Local shell logs preserve their original
file bytes before compression. Downloaded artifact JSON and PNG files were
copied byte for byte and verified against the downloaded ZIP members.

## Candidate identity and outcome

| Source candidate | Observed checkout | Result represented here |
| --- | --- | --- |
| [`413b4bff`](https://github.com/FerroUI/WindowsPhone/commit/413b4bff45efa83c7afc48c51a0001857c80d0e4) | Synthetic merge `fedefc5a00e9d6d9782de241af6e8ca79c78d55c` | First browser run failed after 45 passing checks; native jobs passed. |
| [`019ad836`](https://github.com/FerroUI/WindowsPhone/commit/019ad836070d53edf86fbbc1eb7a42235ed75831) | Synthetic merge `48d41e420e1d159c1294da19580a08c8d35081d9` | Verification corrections exposed a real capture-lifecycle defect: 106 browser checks passed, but the suite failed. |
| [`d515348c`](https://github.com/FerroUI/WindowsPhone/commit/d515348c699c22110def2dded8f7b8aea4b9cd03) | Local committed runtime source | Capture regression changed from failing to passing; 68 phone-system tests and 2 Weather tests passed locally. Full candidate CI remained pending. |

The first two synthetic merges each have baseline
`0f7010ef41b23056ab54b7efeea182a0860ac0d2` and the named candidate as parents.
Their complete trees equal their respective candidate trees:
`49a3af233bcfcefe779cf945c7a237cd0b22494d` and
`d7ff330c91e8437f63de3435901b5c9d626eeca8`. The framework remains pinned to
`ec86b9c0204b8dee243eb300135cb0834f0a84b3`. Identity records are retained in
[first-ci.json](first-ci.json), [retry-ci.json](retry-ci.json) and
[capture-lifecycle-fix.json](capture-lifecycle-fix.json).

## First candidate: fixture failures, 45 passing browser checks

The [first browser results](first-browser/results.json) contain **45 passing
checks**, **118 captures**, `passed: false` and `errors: []`. Empty browser-error
collection does not override failed assertions or timeouts. The final failure
was a timeout targeting `HardwareStart`; the earlier refinement failures are
preserved in the same JSON. The corresponding
[build run](https://github.com/FerroUI/WindowsPhone/actions/runs/45951776435)
failed its rendered-view step and skipped deployment.

Two verification defects obscured later application checks. An older locator
required an automation name on the visible native Pivot header, while the native
`PivotItem` owns the semantic name and the separate `PivotHeaderItem` carries
displayed text. The new private-lock notification fixture also restored the
document without unlocking the shell. Document import deliberately preserves
lock state, so subsequent fixture input was blocked. The saved
[failure view](first-browser/failure-view.json) and
[Metro controls failure](first-browser/metro-controls-failure.json), together
with [first-ci.json](first-ci.json), retain that diagnosis and the observed
failure state.

The follow-up targeted the actual rendered header, closed Action Center and
used the mounted Unlock command before restoration, and removed the transient
completion message left by the progress fixture. Cleanup now requires the lock
layer to be absent. These were verification/documentation changes; their
compiled runtime inputs were unchanged. The exact paths and **80 passing
JavaScript tests** are recorded in
[verification-fixes.json](verification-fixes.json) and
[javascript-retry.log.gz](javascript-retry.log.gz). This correction did not
weaken or bypass the browser gate.

The initial local package checks are separately preserved in
[results.json](results.json): 145 application/SDK/system assertions, 42 controls
assertions and 79 JavaScript tests passed in their identified commands. The CI
Rust record contains **468 passed, 0 failed, 1 ignored** in
[first-browser-rust.json](first-browser-rust.json). These overlapping suites
are not added together into a unique-test total.

## Retry: 106 passing checks exposed a runtime capture defect

The [retry browser results](retry-browser/raw/results.json) contain **106
passing checks**, **196 captures**, `passed: false`, `errors: []` and one failed
refinement module: `verifyNavigationMotion`. Its assertion required both
moving shell surfaces and found no outgoing image after a same-route document
import. This was an application lifecycle failure, not another header locator
or cleanup timeout. The complete failing observation is retained inside the
raw results and the [browser job log](retry-browser-log.log.gz). The unmodified
[complete 59-frame paging trace](retry-browser/raw/metro-shell-paging-apps-frames.json)
is retained separately so every sampled incoming/outgoing surface remains
available after the remote artifact expires. It supplements the earlier visual
review's unchanged 22-file selected subset.

The recorded retry checks for Settings/Office hubs, pending-operation progress
and notification gestures passed. The following Alarms capture has no
`LockScreen` or `ActionCenter`, confirming that the previous private-lock
fixture no longer contaminates the next scenario. The
[visual review](retry-browser/review.json) ties these observations to the
original selected screenshots and inspections. It also preserves two visible
polish gaps at this revision: Weather's loading/footer content lacked the
20-pixel content inset, and the Podcast activity screenshot retained a
“subscribe” hover tooltip. These are before-fix images; they must not be
presented as a visual verification of `d515348c`.

[retry-summary.json](retry-summary.json) records **468 passed, 0 failed,
1 ignored** across the CI Rust suites and **80 passed, 0 failed** JavaScript
tests. [retry-ci.json](retry-ci.json) preserves the successful native workflow
results and the failed
[browser build run](https://github.com/FerroUI/WindowsPhone/actions/runs/45955246510).
[compiled-verification-comparison.json](compiled-verification-comparison.json)
records 41 byte-identical files out of 43 compared browser package files; the
other two differences are revision stamps. Runtime equivalence explains why
verification fixes could expose a pre-existing lifecycle defect. It does not
replace execution of the retry or the subsequent runtime candidate.

## Capture repair: local failing-to-passing regression

Restoring a document while remaining on Start reset navigation's loaded-page
visit, but the native `PhonePage` key depended only on the route. The retained
page therefore did not emit a new Loaded event to register the new visit's
capture source. The next shell swipe changed routes without an eligible
outgoing capture owner.

Candidate `d515348c` includes the document epoch in the page key. Document
replacement mounts a fresh page and capture owner; ordinary revisions retain
the current native page. The new mounted regression drives the production
snapshot/control/input path through restoration and a real shell swipe, checks
the bounded outgoing bitmap and paired motion, and verifies that locking
releases the image and transforms. Its exact name is
`page_motion::capture_tests::restored_start_page_registers_a_fresh_capture_source_for_the_next_shell_swipe`.

| Local verification | Result | Preserved log |
| --- | --- | --- |
| New capture regression before the repair | 0 passed, 1 failed, 67 filtered out | [Before fix](shell-capture-before-fix.log.gz) |
| Same regression after the repair | 1 passed, 0 failed, 67 filtered out | [After fix](shell-capture-after-fix.log.gz) |
| Complete `phone-system` suite | 68 passed, 0 failed, 0 ignored | [System tests](shell-capture-system-tests.log.gz) |
| `phone-app-weather` suite | 2 passed, 0 failed, 0 ignored | [Weather tests](metro-weather-footer-tests.log.gz) |

[capture-lifecycle-fix.json](capture-lifecycle-fix.json) records the exact source,
tree, commands and log digests. **The headless bitmap backend is a size-only
stub.** This regression establishes capture ownership, loaded-state eligibility,
bitmap dimensions, input routing and cleanup; it does not prove rendered bitmap
pixels or browser compositor output. The unchanged browser assertion remains
necessary on the new candidate.

The same candidate aligns Weather's loading/status/location footer with its
existing content inset, makes the busy label 18 px while retaining the 12 px
attribution, and moves the pointer before the Podcast activity capture. The
local Weather tests are not a replacement for new screenshots of those changes.

## Local browser startup produced zero application checks

Two local startup attempts are retained under
[local-browser-startup/](local-browser-startup/README.json). The installed
Chromium launcher pointed to an absent binary. A downloaded Chrome for Testing
`158.0.8127.88` then aborted because the workspace runtime denied its process
singleton `socket()` call. Both attempts executed **0 checks and produced
0 captures**. These are environment startup diagnostics, not failed application
assertions and not local browser passes. The existing GitHub Actions workflow
provided the rendered evidence instead; both original startup logs and their
metadata are preserved unchanged.

## Native evidence and the corrected iOS navigation assessment

The [first Android review](first-android-notification-review.json) and its
[original result](first-android/result.json) preserve API 36 emulator evidence
for native notification admission, visible fixture title/body, listener
revocation, removal of native rows and private text, preserved local groups,
and Back restoration. That mounted Android scenario did **not** perform
horizontal group dismissal, two-finger clear or clear-all activation. Its
fixture was already visible at scroll offset zero. Those gestures require
their separately identified Rust/browser evidence; browser document fixtures
do not synthesize native-provider authority.

The [first iOS review](first-ios-visual-review.json) preserves seven passing
simulator device checks, six URL activations, actual
Search → Back → Search → Start navigation taps, fresh route observations and
the same native process identity. The
[retry iOS provenance](retry-ios/provenance.json) records unchanged original
JSON from the later passing simulator artifact; it does not claim an additional
full screenshot review. Physical brightness and carrier capabilities remain
subject to their recorded device/entitlement limits.

**The older claim that iOS screenshots had blank Back/Start/Search icons was
already corrected.** The existing
[navigation correction](../../feature-completion/ios-navigation-correction.md)
supersedes that interpretation. In the first candidate's eight inspected
original frames, the 828 × 108-pixel navigation row has the same raw RGBA hash
`1d770f09717890e24d6e687920de59d2f5ced8049ad25421d5f028e4c542206d`,
uniform alpha 255 and 535 / 1,860 / 464 bright pixels in the three button
regions. The gray strip below it is the recorded 34-point UIKit safe area.
No rendering fix is justified by the retracted blank-icon claim.

## Evidence boundaries and preservation

- A native build or Rust test pass qualifies its recorded source and command.
  It does not establish every rendered state or acceptance of another source
  revision. The same tests recur across local and CI runs; totals are not summed
  across runs or platforms.
- Static captures show the inspected frames. Progress snapshots and screenshots
  are not atomic animation samples; source/timeline tests and browser motion
  assertions have distinct roles. Suite duration is execution time, not an
  application performance benchmark.
- The progress trajectory uses public WP7 Toolkit evidence, not the private
  WP8.1 implementation. Shell timing policy, font/raster matching and the
  remaining original-device parity limits are described in the parent dossier.
  No record here establishes a 100 percent Windows Phone clone.
- Failed results, failure observations, native platform metadata and retained
  logs remain intact. Selected browser PNGs are unmodified artifact originals.
  Reviews are derived observations and remain distinct from run output; GitHub
  job-log text has the decoding/newline qualification stated above.
- The source `target/metro-local-evidence` inventory was mirrored completely.
  [first-ci-logs.json](first-ci-logs.json),
  [retry-ci-logs.json](retry-ci-logs.json), local result metadata and the new
  [manifest](manifest.json) retain compressed and decoded log hashes. No log was
  recompressed during this archival copy.

Completed CI and any eventual publication for the latest runtime candidate
belong in a separately dated acceptance record. They are not inferred from
this preflight history.
