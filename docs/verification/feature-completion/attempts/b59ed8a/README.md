# Executable contract candidate: `b59ed8a`

Candidate `b59ed8a041e2b53217de5eb93b7eb580beada3be` was tested through merge
`1afd9e85f3c251c260157ff4d21c60c903615849`, with identical tree
`e5e052c56545ca8babd408ef844b9218747409bd` and framework pin `ec86b9c`.

The [browser record](browser-ci.json) and [complete log](browser-build.log.gz)
retain the actual executable results: state/domain/host passed 161 tests with
one ignored case; the executed SDK-gate packages passed 172 tests and failed one.
Alarm service passed all 18 tests, Calendar all 10, and controls all 40, including
the actual 3,000-row native grouping/virtualization fixture. Both previous
Calendar fixture corrections therefore passed. Runtime passed 28 tests and
failed `location_and_missed_call_channels_complete_independently`.

That fixture requested location while its demo settings disabled location. The
production admission guard correctly rejected the request before its provider
channel could exist. The successor explicitly enables the fixture's setting
and supplies a granted location permission, preserving the original ordering
and both completion assertions. The existing disabled-setting policy test and
production guards remain unchanged.

SDK/provider and system/preview packages were not reached by this failed run;
JavaScript, WASM, rendered verification and deployment were skipped. The next
SDK command adds `--no-fail-fast` and step-scoped backtraces so every package can
report during one failed gate. All 25 packages, assertions and failure exit
behavior remain intact. The [successor check](successor-location-check.json)
records parsed workflow equivalence and successful runtime test-body metadata
compilation in 10.33 seconds, with its [complete log](successor-location-check.log.gz).

The [Android ARM record](android-arm-ci.json) records first-attempt success for
portable checks, both APKs, embedding, manifest/JNI/alignment checks and upload.
Other native jobs were still running when this browser-failure snapshot was
recorded. Their unfinished work is not counted as passing here; a successor may
supersede it. Product implementation sources are unchanged from the fully
qualified native candidate `ee85700`; this candidate changed a test fixture and
retained evidence. Source equality does not replace current execution results.
