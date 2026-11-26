# Appointment activation candidate: `ee85700`

Candidate `ee85700f905a5dc5644bfe58f8c0dc5edc44fd1b` was tested through PR merge
`099fe673072c303205168b220e7183151a486840`. Both have tree
`5c4ce3f99e0377e7a43e08f3f41a11e284e52e9e`, with framework pin `ec86b9c`.
The [source snapshot](source.json) and individual records retain full identities.

## Browser failure

The [browser record](browser-ci.json) and [complete log](browser-build.log.gz)
preserve this attempt. The state/domain/host gate passed 161 tests with one
ignored case. Alarm service passed 17 of 18 tests; the failure was
`calendar_open_rechecks_reentrant_unlock_before_publishing_activation`, which
read a signal after its owner had been disposed. The other three newly added
Calendar-opening regressions passed. Calendar's own package, subsequent SDK
packages, JavaScript, WASM compilation, rendered checks and deployment were not
reached.

The failing fixture constructed a replacement `Session` inside its unlock
effect. `Session::new` inherits the current reactive owner, and FerroUI disposes
an effect's previous children before rerunning it. The fixture's deliberate
relock therefore disposed the replacement session's signals despite its retained
`Rc`. The successor prepares that independently owned session before constructing
the callback, then publishes it during the invalidation. All seven dismissal and
unlock cases, privacy assertions and payload assertions remain unchanged.
Production activation guards are unchanged by this fixture correction.

## Completed qualification and retained infrastructure failure

The [desktop record](native-ci.json) records successful full links and all
host/platform/shared-native/packaging gates on Linux, macOS and Windows. Cache
post-jobs succeeded on existing primary keys without writing replacement caches.

The [Android mounted record](android-ui-ci.json) and
[real CallLog receipt](android-call-fixture.json) record another successful
provider and native-runtime execution on this exact merge. Captured rows alone
changed `NEW`, `IS_READ` remained unchanged, the later missed call remained new,
token reuse was rejected, and fixture rows and original authorities were restored.

The [iOS record](ios-ci.json) records another successful native execution of
this exact merge: six system URL activations, four genuine XCTest navigation
taps, eleven full route captures and all seven provider checks. Its
[driver receipt](ios/prompt-handler.json), [provider receipt](ios/device-tests.json)
and four unmodified native button crops are retained here. The earliest retained
Start already observes an available network path and unrequested location
authorization; no earlier Unknown sample or Internet reachability is inferred.
XCTest passed one test in 42.883 seconds; the shell observed its ready handshake
after 67.195 seconds, within the unchanged deadline. The record separates the
successful driver/provider cleanup from unobserved final simulator inventory.

The first ARM packaging attempt passed its portable contracts and native Rust
build, then failed while resolving Gradle dependencies with Maven Central
HTTP 429 responses. The [failure record](android-arm-attempt1.json) and
[complete log](android-arm-attempt1.log.gz) remain distinct from the retry.
No source changes were made for that infrastructure retry.

[Attempt 2](android-arm-attempt2.json) passed the same source's portable gates,
both APKs, embedding library, merged-manifest/JNI/alignment checks and artifact
upload. Its [complete log](android-arm-attempt2.log.gz) is retained separately.
The uploaded archive's API and log digests agree; the archive was not downloaded
locally. These are package/build results, without an ARM-device execution claim.

The [local metadata check](local-check.json) compiled Calendar and alarm-service
test bodies successfully. Its [full compiler log](local-check.log.gz) is retained;
the [initial shell PATH failure](local-path-invocation.log.gz) happened before
any Rust compiler ran. Raw log paths in the record identify the original files;
these compressed copies preserve their exact decoded bytes. Metadata compilation
does not establish test execution.

The [successor fixture check](successor-fixture-check.json) records the exact
corrected test-file hash and successful metadata compilation in 1.33 seconds.
Its [complete log](successor-fixture-check.log.gz) is retained independently of
the failed candidate's execution.
