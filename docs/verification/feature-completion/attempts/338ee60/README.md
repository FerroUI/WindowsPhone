# First feature candidate: `338ee60`

This directory preserves the first execution of PR #5. The candidate is
`338ee603df73190cb71ed78bbd97bb22be9b88a6`; GitHub checked out the synthetic
merge `f2a75acc656bfc47a5a4aeed41cdc033945ce4b5`. Both commits have tree
`165803ec12be0c0527657ad56189b77ece79ba64`. The framework remains pinned to
`ec86b9c0204b8dee243eb300135cb0834f0a84b3`.

## Browser failure and correction

The [browser record](browser-ci.json) and [complete job log](browser-build.log.gz)
retain the failed attempt. The state/domain/host gate passed 161 tests, with one
ignored case. The alarm-service package passed all 14 tests. Calendar announced
10 tests and reported six individual passes before the process aborted during
test-thread teardown. No complete Calendar-package pass, browser compilation,
rendered execution or deployment is claimed for this candidate.

The draft-retention test invoked its final discard callback directly but left
the dialog installed. Its callback retained the same `Rc<Session>` whose dialog
signal owned it, deferring destruction until reactive thread-local teardown and
causing TLS `AccessError`/SIGABRT. Production's shared `DialogAction` clears the
dialog after a successful callback. The successor fixture mirrors that
transaction, asserts no form error and preserves every draft, stale-generation
and explicit-discard assertion. That correction is not part of this candidate's
execution evidence.

## Completed native checks

The [desktop record](native-ci.json) records successful Linux, macOS and Windows
host/platform/shared-native tests, packaging checks and complete application
links. Linux's isolated MPRIS check also passed. These are build and contract
results; desktop GUI or physical-device execution is not inferred.

The [ARM Android record](android-arm-ci.json) records successful Java, pinned
AOSP, Python and Rust checks, both application and HOME-launcher APKs, the
embedding library and package validation.

The [Android mounted record](android-ui-ci.json) records the real emulator
execution. Its [CallLog fixture](android-call-fixture.json) and
[verifier output](android-call-fixture-verification.txt) prove that acknowledging
two captured records changed the total from three to one, preserved the later
call and every `IS_READ` value, rejected token reuse, and removed only the three
owned fixture records during cleanup. Original roles, listener and user state
were restored. The fixture did not make an RF call.

The [iOS record](ios-ci.json) records successful standard/carrier portable and
native bridge checks, both UIKit bundles and simulator verification. Eleven
fresh route observations cover six system URL activations and four actual
XCTest button taps: Search → Back → Search → Start. All seven native provider
checks passed, including a public Network.framework path observation. The
[driver receipt](ios/prompt-handler.json), [provider receipt](ios/device-tests.json),
four unmodified native button crops and individual route observations are
retained in `ios/`. The observed path was available; this does not establish
Internet or application-service reachability. The earliest retained Start
already contained the path and location authorization observations, so no
initial Unknown sample is inferred from this run.

The XCTest body passed one test with zero failures in 89.161 seconds. Its
bootstrap reached the ready handshake within the existing 240-second limit.
Physical cellular behavior, writable device brightness and original-device
visual parity are outside this simulator evidence.

The [source snapshot](source.json) was recorded while jobs were still running;
terminal results and actual checkout proof belong to the individual records.
