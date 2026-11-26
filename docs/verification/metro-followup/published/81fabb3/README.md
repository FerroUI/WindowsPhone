# Metro and iOS release: `81fabb3`

[Open the published app](https://ferroui.com/WindowsPhone/) · [Merged PR #4](https://github.com/FerroUI/WindowsPhone/pull/4) · [Build and publication run](https://github.com/FerroUI/WindowsPhone/actions/runs/45882926545)

## Source and publication

The release source is `81fabb3fdfdeaae81792771a5c448c99b3e93fba`, with tree `9bd934a790b56b980b7b3bb1a14dcdf1381478b2` and FerroUI dependency `ec86b9c0204b8dee243eb300135cb0834f0a84b3`. This main tree equals the reviewed PR head `7fbde4e1e8af91d23e939867914e18152fbceaa4`; the changes from the previously tested code `dc9e2a94d206944023169a789539c02307dd201a` are confined to the earlier verification dossier. The main release has its own fresh builds, tests and public asset checks.

The merge explicitly requested `[verify-ui] [verify-native] [verify-mobile] [verify-android-ui]`. Browser, desktop, ARM Android, Android emulator UI and iOS simulator results below all belong to this main commit. Documentation added after publication does not change the deployed runtime revision.

## Main verification

| Gate | Fresh main result | Evidence |
| --- | --- | --- |
| Browser Rust and JavaScript | 402 Rust passes, one ignored; 77 JavaScript passes | [Browser CI](browser-ci.json) |
| Rendered browser | 93/93 checks, 163 captures, 340,125 ms; zero recorded browser errors; picker cancellation, replacement drag, Back/reopen, explicit save, swivel, System, DPR and offline checks pass | [Rendered results](browser-results.json), [trusted pointer trace](picker-touch.json) |
| Linux desktop | 33 host + 9 platform + 17 shared-native Rust passes; separate private-bus pass; six Python passes; complete build/link in 5m47s | [Native CI](native-ci.json) |
| macOS desktop | 29 host + 5 platform + 17 shared-native Rust passes; six Python passes; complete build/link in 7m32s | [Native CI](native-ci.json) |
| Windows desktop | 30 host + 6 platform + 16 shared-native Rust passes; six Python passes; PowerShell helper parsed; complete build/link in 16m22s | [Native CI](native-ci.json) |
| ARM Android | 111 Java assertions, 24 AOSP source checks, eight Python tests, seven Rust tests; application/HOME packaging, embedding, manifest/JNI/alignment gates pass | [ARM CI](android-arm-ci.json) |
| Android emulator UI | Attempt 2 passes; 47 Python tests and seven Rust tests, 21 captures; carrier call lifecycle, notifications, MMS codec and all four restoration pairs verified | [Android CI](android-ci.json) |
| iOS | Both UIKit configurations build; 8 standard + 8 optional-feature Rust passes; 7 verifier + 11 diagnostics/cleanup Python passes; seven snapshots, six warm activations and six provider checks pass | [iOS CI](ios-ci.json) |
| Pages and public HTTP | Deployment succeeds; 39 precache/worker assets and the additional root-document request pass byte identity and required content-type checks | [Deployment](deployment.json), [HTTP observations](publication-http.json) |

The Linux host/platform totals each retain one ignored fixture; the private-bus test runs separately and passes. All three desktop caches were cold and saved successfully. Test counts retain their suite boundaries and are not deduplicated across platforms.

Fresh iOS observations retain one native process, launch and scene across all six warm activations. The first retained Start is sequence 2 with location `Unknown`; later captures report `Prompt`. Initial Start shows the navigation icons, while restored Start and Settings retain the blank-strip appearance. The record distinguishes observed driver cleanup from independently unverified app, runner and simulator termination.

The first Android UI attempt stopped during Gradle dependency resolution with 92 logged HTTP 429 responses from Maven Central, after its native Rust build and portable gates passed. The emulator had not started. Its [metadata](android-ui-attempt1.json), [complete compressed log](android-ui-attempt1.log.gz) and [source marker](android-ui-attempt1-head.txt) are preserved. After the ARM job finished, one [targeted retry](android-ui-retry.json) ran on unchanged source and passed. No further retry was needed. The successful run has 21 captures, including the additional adaptive notification-scroll position; the earlier PR run's 20-capture count is not reused.

## Public asset identity

Pages job [140141464804](https://github.com/FerroUI/WindowsPhone/actions/runs/45882926545/job/140141464804) completed successfully at **2026-11-26T19:57:20Z**. Public observations began at **2026-11-26T19:58:32.250582+00:00**. The root document, explicit index and service worker all identify **`81fabb3`**. All **40 requests passed**: 38 precached files, the service worker and the public root document.

The [compiled artifact](compiled-artifact.json) has verified ZIP SHA-256 `7ddbb76c1819a78f8b4cd3188356aae82ef40b62efeb3b275373b16482aa8e79`. Its **23,514,339-byte WASM module** and the public response both have SHA-256 `4c613d29097a228e2e603f0fb9229d190590dd21a5ae9271fc16512cfc1feba0`. The [43-file comparison](compiled-comparison.json) with the reviewed artifact finds 41 byte-identical files; the other two change only the single checkout-derived stamp `f8bed7d` → `81fabb3`.

The HTTP check covers every entry in the generated service-worker precache plus the worker itself, and independently requests the public site root against the built index. It checks status, byte length and SHA-256 against the compiled artifact, validates the build marker against the source commit, and requires usable HTML, JavaScript, CSS and WASM content types. The retained root document, index and worker are the original observed response bodies. These HTTP observations establish deployed artifact identity; the rendered interaction results come from the exact-source CI browser run.

For an existing installation, Settings → phone update checks for the new build. Close every old app tab and installed app window, then reopen the app, so the waiting service worker can activate. The worker preserves scope ownership and does not force activation while an old app window remains open.

## Evidence and boundaries

- [Release summary and source boundary](release.json), [exact source identity](source.json).
- [Browser CI and artifact identities](browser-ci.json), [complete rendered results](browser-results.json), [trusted pointer trace](picker-touch.json).
- [Native desktop](native-ci.json), [ARM Android](android-arm-ci.json), [Android emulator and attempt history](android-ci.json), [iOS](ios-ci.json).
- [Deployment record](deployment.json), [original deployment log](deployment.log), [public HTTP record](publication-http.json).
- Original public response bodies: [root document](publication/root-document.html), [index](publication/index.html), [service worker](publication/app-sw.js).
- [HTTP verification program](verify-publication.py), [SHA-256 manifest](manifest.json).

The CI records retain the original extraction/log paths for provenance. Their archived copies and selected raw payloads are listed here and in the manifest; complete capture archives remain identified by their GitHub artifact IDs and ZIP digests.

The [earlier implementation dossier](../../README.md) explains the picker transaction, bounded scrolling, application-bar and iOS lifecycle changes and retains the reviewed source, test fixtures and captures. Its captures remain attributed to their original revisions.

The pinned FerroUI changes are available separately in [framework PR #1456](https://github.com/wieslawsoltes/FerroUI/pull/1456), which remains open. Its runtime gates passed; its aggregate CI result retains an unrelated generated C++ header check failure reproduced on its unchanged base. That upstream result is not presented as an entirely green framework workflow.

Native builds and emulator/simulator observations do not establish physical-device, signed-package or original Windows Phone visual parity. The original Windows Phone emulator setup workflow was skipped for this release. The pre-existing iOS restored-home navigation-strip capture limitation remains documented in the implementation dossier.
