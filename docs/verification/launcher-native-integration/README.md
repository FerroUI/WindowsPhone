# Launcher and native integration verification

This directory preserves evidence for the launcher and device-service expansion, using FerroUI `db2e8859cd4b67bac870839ea70503b8babf541f`. Browser source [`77f0a0f91a168e0f4c152cc59d762625a72f1382`](https://github.com/FerroUI/WindowsPhone/commit/77f0a0f91a168e0f4c152cc59d762625a72f1382) passed its build and deployment jobs in [run 45282852497](https://github.com/FerroUI/WindowsPhone/actions/runs/45282852497). Each native record retains its own tested revision and qualification scope.

## Records

| File | Evidence and scope |
| --- | --- |
| [browser.json](browser.json) | Browser CI outcomes, exact Rust/JavaScript counts, build duration, artifact IDs and hashes, selected capture hashes, and visual-review findings. Temporary workspace paths are normalized for this directory; the source-record hash is retained. |
| [desktop.json](desktop.json) | Successful Windows/macOS/Linux host, plugin, shared-native and packaging tests, isolated MPRIS protocol verification, and actual complete executable builds/links. |
| [android-package.json](android-package.json) | Actual ARM64 App/Launcher assembly, embedding compilation, contract/provenance checks, merged manifests, JNI entry and ELF/APK 16 KiB alignment at `77f0a0f`. |
| [android-ui.json](android-ui.json) | Successful actual App/HOME runtime, role restoration, native incoming SMS, Answer input, ongoing foreground call ownership and remote-end teardown at `77f0a0f`, including cold Start unread and native notification admission/rendering/revocation; original capture and APK hashes. |
| [ios.json](ios.json) | Successful standard/optional UIKit builds, physical-iPhone Swift SDK check, five real provider checks, and six same-process system-delivered activations at `9a628e3`; original captures, identities and startup history. |
| [results.json](results.json) | Unmodified full rendered-suite result: 71 passed checks, 132 captures, `errors: []`, and 215,695 ms elapsed. |
| [browser-artifact-manifest.json](browser-artifact-manifest.json) | Unmodified complete artifact manifest with hashes for all 268 extracted browser verification files. |
| [observation-validation.json](observation-validation.json) | Focused 25 runtime and 3 state regressions at `99ffbfc`; their counts overlap browser CI and are not added to it. |
| [activation-validation.json](activation-validation.json) | Real reactive launch regressions: 22 shell tests and 2 SDK activation tests, including preserved failures before the correction. Counts overlap subsequent CI. |
| [source-boundaries.json](source-boundaries.json) | Target-specific source and workflow fingerprints binding each complete passing platform record to the implementation revision. |
| [record_launcher_source_boundaries.py](record_launcher_source_boundaries.py) | Reproducible qualification and source-comparison helper; requires Python 3.10+ and PyYAML. |
| [test_source_boundaries.py](test_source_boundaries.py) | Focused regressions rejecting substituted targets, incomplete platform gates, omitted source inputs and stale or corrupted saved proofs. |
| [local-validation.json](local-validation.json) | Unmodified local source/protocol validation record prepared before native target and rendered CI. Its overlapping executions remain separate and must not be summed. |
| [publication-http.json](publication-http.json) | Unmodified direct public HTTP observations of the deployed page and service worker on 23 November 2026 at 23:32 UTC. |
| [corrections.json](corrections.json) | Causal fixes found by real platform checks, their exact source commits and failed attempts, plus separately attributed SDK-download infrastructure retries. |
| [manifest.json](manifest.json) | SHA-256 and byte length for each included file, with original-source attribution for copied records and captures. |

The browser job passed **331 Rust tests** (156 state/domain/host and 175 SDK/runtime/app/system), with zero failures and one intentional private-D-Bus test skip. It passed **55 JavaScript boundary tests** and compiled the WASM browser profile in **22m50s**. The ignored MPRIS test passed separately in its dedicated D-Bus session in Linux CI. The **215.695-second** rendered-suite duration measures verification execution, not application startup or rendering performance. [Exact results and qualifications](browser.json).

## Observation and source history

[observation-validation.json](observation-validation.json) records the focused foreground initialization, read ownership, authority revocation and completion/event ordering checks. Its 25 runtime and 3 state tests are also represented in broader CI and are not added to those counts.

[corrections.json](corrections.json) distinguishes application fixes, verification-driver corrections, diagnostic follow-ups and infrastructure failures. The [history directory](history/) retains the original earlier browser/desktop and Android successes, partial iOS provider evidence, and the exact later runtime attempts that stopped before completing their required phases. Those historical records retain their original artifact filenames; they are not substituted for current target results.

## Source and evidence boundaries

[source-boundaries.json](source-boundaries.json) records the tested revision and complete qualification record for each of five scopes: browser, all three desktops, Android ARM64 packages, Android x86_64 runtime, and iOS. Shared app, SDK, runtime, system, asset, dependency and pinned-framework inputs belong to every applicable scope. Platform-specific native files and build/verification scripts are included where consumed; iOS also includes the browser icon that its bundle builder reads.

The helper validates each target's actual required gates before comparing source. A browser record cannot qualify desktop code; a desktop record must contain all three real platform jobs. Android runtime evidence must include the cold Start badge, notification admission/rendering/revocation/restoration and call ownership. iOS must include both builds, all five provider checks and all six ordered warm activations. Artifact run and head identities must agree with the tested source.

Workflow comparisons retain runners, tool versions, matrices, commands, step conditions, timeouts, environment, permissions, defaults and required job dependencies. Event admission, top-level concurrency, the selected jobs' top-level admission conditions and unrelated jobs do not change the selected execution contract. Scalar tags, explicit tags, styles and values remain significant. An unreviewed local action or reusable-workflow dependency fails closed.

The saved proof hashes its generator, evidence records and scoped inputs. `--check-only` reads and reconstructs that saved proof without rewriting it; missing, corrupt or stale proofs fail. A later documentation-only commit is accepted only if every target fingerprint remains equal. This establishes unchanged tested source, not identical build timestamps, artifact bytes or an additional platform execution. The helper's **39 focused regression tests** passed; these tooling tests remain separate from application CI counts.

## Native desktop gates

| Target | Host / desktop-plugin / shared-native tests | Packaging and protocol checks | Complete executable |
| --- | --- | --- | --- |
| Linux | 33 / 9 / 17 passed | 6 packaging tests; 1 separate private-D-Bus MPRIS test | Built and linked |
| Windows | 30 / 6 / 16 passed | 4 packaging tests, 2 POSIX-only skips; PowerShell helper parsed | Built and linked |
| macOS | 29 / 5 / 17 passed | 6 packaging tests | Built and linked |

All three actual jobs passed at [`77f0a0f`](https://github.com/FerroUI/WindowsPhone/commit/77f0a0f91a168e0f4c152cc59d762625a72f1382) in [run 45282852497](https://github.com/FerroUI/WindowsPhone/actions/runs/45282852497): [Linux](https://github.com/FerroUI/WindowsPhone/actions/runs/45282852497/job/138071505320), [Windows](https://github.com/FerroUI/WindowsPhone/actions/runs/45282852497/job/138071505049), and [macOS](https://github.com/FerroUI/WindowsPhone/actions/runs/45282852497/job/138071505178). Complete build/link steps took 7m44s, 3m41s and 4m16s respectively. All use actual target media providers; Linux installs its media development libraries. The isolated GIO fixture is run by its parent with temporary XDG data. Native compilation and protocol coverage does not establish hardware, permission dialogs, rendered desktop interaction or installation as the user's shell. [Desktop record](desktop.json) retains exact jobs, commands, durations and log hashes.

## Android ARM64 packages

Revision [`77f0a0f`](https://github.com/FerroUI/WindowsPhone/commit/77f0a0f91a168e0f4c152cc59d762625a72f1382) passed [run 45282852656](https://github.com/FerroUI/WindowsPhone/actions/runs/45282852656): **111 JVM assertions**, **24 AOSP source hashes**, **8 package-validator tests** and **7 portable Android Rust tests**, actual App/Launcher APK assembly and the embedding library check with default features disabled. Both APKs passed protected component and merged-manifest validation, exported `JNI_OnLoad`, and **16 KiB ELF and APK ZIP alignment**. Actual decoded manifests require `BIND_INCALL_SERVICE`, the exact `phoneCall` foreground-service type and its permission; they do not add `MANAGE_OWN_CALLS`.

The [ARM64 debug APK artifact](https://github.com/FerroUI/WindowsPhone/actions/runs/45282852656/artifacts/14870429345) was downloaded and SHA-256 verified as `1745b0e6d0a4574bf33169d182b2cb289b0c4780fec099ddfcb6c4bdbe6e96b0`. Individual APK/native hashes are retained in [android-package.json](android-package.json); unmodified decoded manifests are [App](manifests/android-app.xml) and [Launcher](manifests/android-launcher.xml). Packaging and embedding checks remain distinct from the emulator runtime gate and physical-device qualification.

## Android launcher and communications runtime

The same revision passed [API 36 x86_64 run 45282852605](https://github.com/FerroUI/WindowsPhone/actions/runs/45282852605). Both APK flavors passed actual mounted geometry, Start/Wallet activation and physical Back. The private emulator then selected the Launcher package as its real HOME holder, independently observed active launcher/device roles, resolved implicit HOME, returned from Wallet using the HOME key, and stayed at Start after Back. Twelve mounted checkpoints were replayed through the unchanged geometry/role verifier.

A unique modem-injected SMS passed through the actual Android provider. **Before the first Messaging visit**, Start displayed the canonical Messaging badge **1** and the shared store contained the corresponding unread native thread. Messaging subsequently showed the correct sender and complete nonce body. The cold Start and communications stages use the Launcher package with SMS/Dialer active after the original HOME holder has been restored; their observed mode is correctly `App`. Dedicated earlier checkpoints establish active HOME behavior.

The fixture granted only this package's notification listener and posted a unique Android shell notification. The native provider admitted it to the shared store at sequence **16**. Its exact title/body initially lay below the measured notification viewport. One real native swipe inside that viewport, above the shell's close-gesture region, revealed both complete labels at sequence **40** while Action Center remained open. The [original pre-scroll observation](captures/android-notifications-before-scroll.json), [capture](captures/android-notifications-before-scroll.png) and [native input coordinates](captures/android-notifications-scroll-input.txt) preserve that causal sequence. Revocation then removed the notification-read operation, all native rows and both rendered fixture labels at sequence **44**; independent local notification groups remained. The exact original listener set was restored. This run tests grant, revoke and original-set restoration; it does not add a separate regrant phase.

An incoming emulator call was answered through this app's native Answer control. Ringing and active observations required actual `phoneCall` service type **4**, a posted foreground notification and `CallStyle`. A modem-simulated remote hangup then left an empty native call inventory, service type **0**, and no foreground notification/CallStyle ownership. The Android crash buffer was empty. The gate exercises Answer through native UI automation; termination uses `gsm cancel`, so it does not establish End-call button input. Thirteen bounded MMS codec assertions ran with **zero RF requests**; they do not establish carrier MMS delivery.

Cleanup restored the original HOME, Dialer and SMS holders, the complete listener set, and Android user **0 → 0**. The gate changes roles only inside its newly created private emulator. User-facing chooser consent/denial and physical OEM policies remain separate qualification work.

The [runtime artifact](https://github.com/FerroUI/WindowsPhone/actions/runs/45282852605/artifacts/14869353537) contains **20 unmodified original captures** and independent observations; its verified ZIP SHA-256 is `f7f714694772564b2f37833cb5c72215f80d2aace7c21c887f854b0af737df17`. The [x86_64 APK artifact](https://github.com/FerroUI/WindowsPhone/actions/runs/45282852605/artifacts/14870008593) has SHA-256 `c2d590a7f12ac4caac22ad07362522d5f519e3db193197cc3be0e4212bbeb303`. All twenty originals were reviewed; the eight selected images below also received an independent review during evidence packaging.

| Capture | Observed state | Independent observation |
| --- | --- | --- |
| [Back at active HOME](captures/android-home-back.png) | Start remains visible after physical Back with the Launcher flavor selected as actual HOME. Sample badges belong to local fixture data. | [Mounted UI and role](captures/android-home-back.json) |
| [Cold Start unread](captures/android-cold-start-unread.png) | Native Messaging unread badge **1**, before opening Messaging. | [Mounted badge, native thread and roles](captures/android-cold-start-unread.json) |
| [Before notification scrolling](captures/android-notifications-before-scroll.png) | The native group is below local groups; its title/body are outside the measured viewport. | [Exact fixture ancestry and geometry](captures/android-notifications-before-scroll.json) |
| [Rendered native notification](captures/android-action-center.png) | Complete unique title/body become visible after one native swipe. | [Native admission](captures/android-notifications-admitted.json), [visible labels and bounds](captures/android-action-center.json) |
| [Access revoked](captures/android-notifications-revoked.png) | Local groups remain; native rows and fixture text are removed. | [Revoked operation and empty native store](captures/android-notifications-revoked.json) |
| [Received SMS](captures/android-messaging.png) | The modem fixture's sender and complete message body appear in shared Messaging. | [Mounted UI](captures/android-messaging-ui.json), [provider](captures/android-messaging-provider.json) |
| [Answered call](captures/android-active-call.png) | The native surface shows the fixture number, active state and call controls. | [Calls and foreground ownership](captures/android-active-call.json) |
| [Ended call](captures/android-ended-call.png) | The native surface reports call ended after remote hangup. | [Empty inventory and released ownership](captures/android-ended-call.json) |

All native numbers, bodies and title nonces belong to the disposable fixture. [android-ui.json](android-ui.json) retains the exact assertions, role context, all original capture hashes, authority restoration and qualification limits.

## iOS UIKit and native integration

Revision [`9a628e3`](https://github.com/FerroUI/WindowsPhone/commit/9a628e39af18b727555ad4f9a5184761d2519502) passed [run 45287778966](https://github.com/FerroUI/WindowsPhone/actions/runs/45287778966/job/138088468821). Both the standard UIKit application and optional `ios26-telephony` variant built, linked and bundled successfully. The optional Swift provider passed physical-iPhone SDK type checking; all Objective-C bridges passed simulator SDK checks with the feature disabled and enabled. Portable tests passed in both configurations, together with the mounted-proof and owned-process/crash collector tests. These are simulator bundles and SDK checks, not a signed physical-device deployment.

The [original provider result](captures/ios-device-tests.json) records all **five** required checks: observed simulator capabilities, read-only brightness and actual write refusal, Contacts create/query, update/query and delete/query. Brightness remained **0.5** after a rejected request for **0.67**; no physical backlight write is inferred. The uniquely identified test contact was deleted successfully, with no cleanup errors. No call or message was sent. The unprovisioned simulator correctly reported carrier permissions/roles unavailable.

The public XCTest system URL driver then delivered **six** links: Wallet, the Phone keypad, device People, message composition, device Settings and Home. All were accepted by the native delegate and mounted the expected controls. Seven observations, including initial Start, retained PID **30561**, per-launch UUID **27AD9371-71F8-44FE-98AB-94E2AC8DE490**, bundle `org.ferroui.windowsphone`, a foreground application and the same active UIKit scene. All measured a **414 × 814-point** phone surface inside the **414 × 896-point** client, with **48 top / 34 bottom** safe-area insets. Settings finished at its owned `device services` child route with an accepted activation and empty status; this directly exercises the shared activation correction.

| Native state | Original capture | Independent observation |
| --- | --- | --- |
| Initial Start | [PNG](captures/ios-start.png); tile badges belong to local sample data | [Start proof](captures/ios-start.json) |
| Wallet | Original PNG retained in the artifact | [Accepted Wallet proof](captures/ios-wallet.json) |
| Phone keypad | Original PNG retained in the artifact; the link only fills the keypad | [Accepted dial-path proof](captures/ios-phone.json) |
| Device People | [PNG](captures/ios-people.png); the simulator's sample contacts are visible after test-contact cleanup | [Accepted device-contact proof](captures/ios-people.json) |
| Message composition | [PNG](captures/ios-messages.png); shared compose form, with no send | [Accepted composition proof](captures/ios-messages.json) |
| Device Settings | [PNG](captures/ios-settings.png); owned detail page without the earlier false launch-error banner | [Accepted Settings proof](captures/ios-settings.json) |
| Return Home | Original PNG retained in the artifact | [Accepted Home proof](captures/ios-home-restored.json) |

All seven original captures were reviewed; the four retained PNGs also received an independent review during evidence packaging. The six original request records ([1](captures/ios-url-request-1.json), [2](captures/ios-url-request-2.json), [3](captures/ios-url-request-3.json), [4](captures/ios-url-request-4.json), [5](captures/ios-url-request-5.json), [6](captures/ios-url-request-6.json)), [driver completion](captures/ios-prompt-handler.json), and [static startup phases](captures/ios-startup-console.log) remain unmodified. The [verification artifact](https://github.com/FerroUI/WindowsPhone/actions/runs/45287778966/artifacts/14873204474) ZIP has verified SHA-256 `8e11a4c7cbdf482e5aa7e0370914af79d514d9ba1fa23616a734d232721e970f`; the [simulator bundle artifact](https://github.com/FerroUI/WindowsPhone/actions/runs/45287778966/artifacts/14872314593) has SHA-256 `5b71712b3ee4481a150489f4f58794b23910eb58d0e3f89a020dc3ef03605c18`. [ios.json](ios.json) retains exact gates, source identity, original capture hashes and qualification limits.

Earlier attempts intermittently failed to mount the initial view. Their records remain in history. The focused follow-up adds opt-in early startup phases and exact installation/process/crash diagnostics while preserving provider behavior, SDK, runner, the **90-second** initial mount deadline and all functional assertions. That earlier startup failure did not reproduce in this passing run; its root cause remains unconfirmed. No claim of a production startup fix or physical-device reliability is inferred from this one result.

## Representative captures

These four **unmodified 1280 × 936 PNGs** and their matching native-control inspection JSON come from [UI artifact 14872119728](https://github.com/FerroUI/WindowsPhone/actions/runs/45282852497/artifacts/14872119728), SHA-256 `e3d35196b40748fe467b08dec45ae54b2225f6faf5f45e2cd0798d16e10d0052`. The original artifact contains all 132 captures. Its hash was verified after download; per-file hashes appear in [browser.json](browser.json) and [manifest.json](manifest.json).

| Capture | Observed state | Inspection |
| --- | --- | --- |
| [Map pan and zoom](captures/maps-native-tiles-pan-zoom.png) | Measured tile viewport with coordinates, zoom controls, scale and attribution, separate from the app bar and phone navigation. | [JSON](captures/maps-native-tiles-pan-zoom.json) |
| [Saved-place Start activation](captures/maps-saved-place-start-activation.png) | A Start shortcut opens the saved place's title, address, coordinates and centered marker. | [JSON](captures/maps-saved-place-start-activation.json) |
| [Six live-tile templates](captures/start-live-templates.png) | Text, flip front, list, progress, photo and photo-cycle cards; capped count badges and native progress track. | [JSON](captures/start-live-templates.json) |
| [Expanded live folder](captures/start-folder-expanded-children.png) | Live cover, independent child tiles, retained glyphs/badges and reachable close control. | [JSON](captures/start-folder-expanded-children.json) |

All four selected captures received two independent visual inspections. The retained controls fit their intended surfaces without a new clipping or overlap defect. Maps uses the controlled PNG response from [`scripts/ui-map-fixture.mjs`](../../../scripts/ui-map-fixture.mjs); these images establish rendering, interaction, cancellation and route behavior, not public map-service availability or map-data accuracy. The production OpenStreetMap attribution remains visible. Live-tile content and saved-place details are isolated test fixtures, not real user records. These captures are application output, not original Windows Phone reference images or proof of pixel equality.

## Publication and qualification

[Direct HTTP observations](publication-http.json) recorded status **200** and the `77f0a0f` marker for both `https://ferroui.com/WindowsPhone/` and its `app-sw.js` at 23:32 UTC. Both response bodies match the compiled artifact byte for byte; both Last-Modified values were 23:28:49 UTC. This establishes the served revision at the recorded time. It does not establish an existing browser profile's service-worker upgrade behavior.

The local metadata/API checks and browser fixtures alone do not establish actual native provider compilation. The separate native records retain their own commit, job, device/session and operation boundaries. Neither compilation nor simulator/emulator checks establish physical carrier delivery, MMS APNs, OEM behavior, permission behavior on physical devices, audible media, production signing or complete Windows Phone parity. No pending gate is counted as a pass.
