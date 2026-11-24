# iOS startup investigation

**Status, 2026-11-24:** the fresh E source `41d7e3c` passed the iOS gate with all seven mounted route captures, five provider checks and six native URL activations. Its new phase markers show all four permission reads returning in this run. The two earlier attempts at unchanged C source `1a20466` remain failed initial-Start checks with different startup signatures and zero runtime credits. E's success does not identify their cause or establish a startup fix.

## Successful E observation

The fresh-source [E job 138420827831](https://github.com/FerroUI/WindowsPhone/actions/runs/45387122381/job/138420827831) tested [`41d7e3ceddb2d317edf3f1b8f22a7358d38bb7e6`](https://github.com/FerroUI/WindowsPhone/tree/41d7e3ceddb2d317edf3f1b8f22a7358d38bb7e6) on iPhone 11 Simulator, iOS 26.5, using Xcode 26.6. The [evidence report](ios-41d7e3c.json) records **15 Python passes**, **14 Rust executions** of seven cases in two feature configurations, successful physical-target Swift type checking, both simulator Objective-C feature checks, both UIKit bundle builds and the successful native runtime gate. The [manifest](ios-manifest-41d7e3c.json) verifies all 32 retained original artifact files against archive SHA-256 `d2307998aa25796fd93a356132c755fa74bfb5bbb31455b38bfac1c5855ad749`.

All seven original PNGs were visually inspected. They show Start, Wallet, the prefilled dialer, device Contacts, the empty message composer, Device Services and restored Start. The [provider record](captures-41d7e3c/ios-device-tests.json) passed observed capabilities, read-only brightness and Contacts create/query, update/query and delete/query; cleanup errors were empty. No call or message was sent. Simulator brightness stayed at 0.5 after the request for 0.67 was refused; physical brightness, carrier roles and other device hardware remain unqualified.

The XCTest URL driver passed one test in **37.386 seconds**. Each of the six destination observations records `activation.outcome = accepted` with its exact requested URI. All seven observations retain PID **15442**, the same launch UUID and the same active UIKit scene. The [prompt-handler record](captures-41d7e3c/ios-prompt-handler.json) has `accepted = 0`; that field counts accepted system prompts, not native URL activations. The evidence report correlates each request, timestamp, sequence and native identity. These checks establish continuity across these observations, not a general lifecycle stress result.

The first permission interval in the [original E console](captures-41d7e3c/ios-console.log) contains complete before/after pairs:

| Permission read | Begin, elapsed seconds | End, elapsed seconds | Observed interval |
| --- | --- | --- | --- |
| Contacts status | 0.952131 | 0.952361 | 0.230 ms |
| Camera authorization | 0.952370 | 0.982443 | 30.073 ms |
| Location authorization | 0.982507 | 1.001324 | 18.817 ms |
| Cached notification status | 1.001359 | 1.001365 | 0.006 ms |

`application.main_view.end` followed at elapsed **1.851546 seconds**, then `application.framework_init.end` at 1.851559. These are timestamps from one successful instrumented execution, not latency guarantees or performance benchmarks. They show that the previously undifferentiated calls returned in E; they do not attribute C's missing progress to an API or prove it was repaired.

The [launch-start record](captures-41d7e3c/ios-launch-driver-started.json) observed an empty console with no parsed native PID at **11:56:09.262 UTC**. The [cleanup-begin record](captures-41d7e3c/ios-launch-driver-cleanup-begin.json), at **11:57:52.212 UTC**, observed the later console with native PID 15442. The direct CLI driver's PID was 15379; its liveness remained separate from app identity. The [cleanup-end record](captures-41d7e3c/ios-launch-driver-cleanup-end.json) records that driver no longer running, without inventing an exit code that the existing wait had not obtained. No failure sampling or automatic relaunch occurred. These records preserve observation time explicitly and demonstrate the distinction between an early empty log and later PID output.

## Observed evidence and its limits

The initial C run, [45378306221](https://github.com/FerroUI/WindowsPhone/actions/runs/45378306221), tested source [`1a2046672e0e82ffd3fe978fdae8ff4c0b5c8366`](https://github.com/FerroUI/WindowsPhone/tree/1a2046672e0e82ffd3fe978fdae8ff4c0b5c8366). It failed while waiting for the mounted Start route. The [initial-failure record](history/ios-1a20466-attempt1.json), [original console log](history/ios-1a20466-attempt1/console.log), [process identity](history/ios-1a20466-attempt1/owned-process.json) and [original screenshot](history/ios-1a20466-attempt1/failure.png) retain the evidence. The record links the originating CI job and hashes its decoded log.

The last phase was `snapshot.permissions` at elapsed **1.383355 seconds**. There was no subsequent `snapshot.carrier`, `snapshot.hardware`, `snapshot.end`, `application.host.end`, or `application.main_view.begin`. The earlier `snapshot.contacts` phase at 1.307877 was followed by `snapshot.tel` at 1.344526, proving that the first Contacts status query returned. The unsupported `tel:0` availability-query log also preceded later completed phases.

The original failure PNG shows a white application area beneath the native iOS status bar. The owned-process record identifies PID 29690 and rechecks the exact installed executable. Its bounded stack-sampling command timed out after eight seconds; no stack sample or crash report was retained. The evidence therefore localizes missing progress to the aggregate permissions interval, not an individual API or proven deadlock.

The one authorized unchanged-source retry, [job 138403728001](https://github.com/FerroUI/WindowsPhone/actions/runs/45378306221/job/138403728001), also failed the 90-second initial mount gate. Its [report](history/ios-1a20466-attempt2.json), [original console](history/ios-1a20466-attempt2/console.log), [process diagnostic](history/ios-1a20466-attempt2/owned-process.json), [original screenshot](history/ios-1a20466-attempt2/failure.png) and [raw-file manifest](history/ios-1a20466-attempt2/manifest.json) retain all seven artifact entries without editing.

| Observation | Attempt 1 | Attempt 2 |
| --- | --- | --- |
| Native entry and progress | Rust main and framework initialization reached `snapshot.permissions` | No Rust main entry or native phase marker |
| Failed capture | White application surface with the iOS status bar | SpringBoard with the installed Windows Phone icon |
| Process diagnostic | Exact installed PID 29690; stack sampling exceeded eight seconds | No unique exact executable identified; the old error does not distinguish zero from multiple matches |
| Start wait | 90.459 seconds | 90.302 seconds |
| Route observations, provider checks, accepted activations | 0 / 0 / 0 | 0 / 0 / 0 |

Each attempt passed **13 Python test executions** and **14 Rust executions** across two feature configurations, the physical-target Swift type-check, simulator Objective-C syntax checks and both UIKit bundle builds. These repeated build checks are not runtime credits. Neither attempt produced `start.json`, `device-tests.json`, a URL request, or any of the seven expected route captures. The two original failure PNGs were inspected individually.

Earlier B verification passed with the same iOS provider and harness files. Its first permissions interval completed in 41.492 milliseconds; the earlier `9a628e3` record also completed it. Historical `99ffbfc` showed a white pre-mount surface without a specific blocking frame, while `77f0a0f` showed SpringBoard and no framework phases. These are separate observations with [retained history](../launcher-native-integration/README.md#ios-uikit-and-native-integration). The two failed C attempts do not establish a common cause or a regression in the shared control changes.

## Late console output and the launch boundary

Attempt 2's retained console is exactly the 32-byte line `org.ferroui.windowsphone: 34323` followed by a newline. It contains no escape sequence, and the unchanged exact parser returns PID 34323. However, its artifact entry was last modified at **11:31:06**, while the process diagnostic was captured at **11:27:00.891** and the failure PNG has an **11:27:02** ZIP timestamp. The diagnostic and final log therefore describe different observation times. A parser defect was not reproduced, and the bundle/PID matching rule must not be loosened to explain the earlier null value. Original timestamps are retained in the [manifest](history/ios-1a20466-attempt2/manifest.json).

The shell starts `simctl launch --console-pty` directly and tests its child PID with `kill -0`. This establishes console-driver liveness, not native application liveness. The old failure collector reads the console once; its generic exact-process error also loses the distinction between no candidate and multiple candidates. Those are concrete diagnostic gaps. Neither a late driver PID nor a SpringBoard screenshot proves that the application reached main or identifies an exit cause.

The existing XCTest flow starts only after the mounted Start and provider checks. It requires an already-running application and uses `activate()`, then proves the original PID, launch UUID and exact native activation URI. It cannot observe the preceding failed launch in its current position. Adding an automatic `launch()` or retry there would change the lifecycle being tested and could mask an application-startup failure. No automatic relaunch has been added.

## Exact dependency before `set_main_view`

In C, and unchanged in E apart from the diagnostic markers, the synchronous startup path is:

```text
PhoneApplication::on_framework_initialization_completed
  IosPlatform::new -> native::monitor_device(true) -> dirty = YES
  NativeHost::new -> NativeHost::compose
    PhoneRuntime::attach -> PhoneRuntime::observe
      IosPlatform::drain_events -> native::device_events
        wp_ios_device_poll -> wp_ios_device_snapshot
  host.lifecycle(HostLifecycle::Resumed)
  lifetime.set_main_view(...)
```

Relevant sources are `crates/phone-ios/src/application.rs`, `crates/phone-native/src/lib.rs`, `crates/phone-runtime/src/lib.rs`, `crates/phone-ios/src/{platform,native}.rs`, and `crates/phone-ios/native/device.m` at the linked revision.

`wp_ios_device_poll` performs a complete synchronous snapshot when dirty, rather than only consuming completed observations. `snapshot.permissions` contains a second Contacts status query, camera authorization status, the Core Location class authorization query, and a cached notification-permission getter. This makes host construction depend on those calls returning. Even `Resumed` runs before main-view installation. Existing operation timeouts use the main RunLoop's `NSTimer`; they cannot interrupt a synchronous main-thread stall.

## Apple contracts relevant to this interval

| API | Primary evidence and practical limit |
| --- | --- |
| Contacts authorization status | Apple explicitly documents the exact [status query][contacts] as thread-safe and nonblocking. General warnings about Contacts fetch I/O do not diagnose this query. This contract does not rule out an OS or simulator defect. |
| Camera authorization status | The [API reference][camera] returns current authorization but gives no fixed-latency guarantee. Apple's [AVCam sample][avcam] performs the query in its `CaptureService` actor, whose architecture keeps capture API calls off main. This supports isolated observation, not attribution of this failure. Capture-session startup warnings concern different operations. |
| Core Location authorization | The [class query][location-class] is deprecated from iOS 14. Apple documents initial authorization delivery through the [delegate callback][location-delegate]. A [manager's callbacks][location-manager] use its initializing thread's active RunLoop; an arbitrary worker queue is not a substitute. These documents do not promise bounded initialization latency. |
| Notification authorization | The snapshot getter only returns a cached string. The actual [settings query][notifications] completes asynchronously, potentially on a background thread; existing code marshals publication back to main. The cached getter is not a synchronous settings request. |

## Constraint on any asynchronous redesign

Shared device `PermissionState` has `Unsupported`, `Prompt`, `Granted`, `Limited`, `Denied`, and `Restricted`, but no unknown state. Missing snapshot entries become `Unsupported`; SDK UI renders that as unavailable. `DeviceStore` starts with a default snapshot. General `HostPermission`, by contrast, already has `Unknown`.

Consequently, a pending observation cannot honestly be represented by omission, denial, a fabricated prompt state, or a grant. A future asynchronous design needs explicit observation readiness or an unknown permission state, with admission denied until authoritative observation. The notification cache's initial `"unsupported"` value is a related semantic limitation, not the demonstrated cause of this failure.

Snapshots also control admission, capability UI, and retirement of private data. Any redesign must retain atomic permission/data publication, Limited Contacts semantics, revocation handling, lifecycle generations, bounded queues, and rejection of late completions after suspension or closure. Moving the whole UIKit snapshot to a worker or merely dispatching it later on main does not satisfy these requirements.

## Bounded diagnostics verified in E

E added these limited diagnostic changes:

- Each of the four existing permission reads has a before/after phase marker. Queries, returned values, calling thread and permission admission remain unchanged; the existing opt-in 512-entry phase limit still applies.
- At launch start, an observed driver exit, mount timeout and cleanup boundaries, at most five `launch-driver-*.json` records preserve the direct driver's PID/liveness, an exit code only when the existing shell wait obtained it, and a timestamped console observation. The driver is never used as the native sampling PID, and no wait is added for a live driver.
- The failure collector records the number of exact installed-executable matches, classifies none/unique/multiple, and retains at most eight matching PIDs. It still requires uniqueness, matching console identity when available, the unchanged bundle path/inode/size, and a second live-path check before sampling.
- Console metadata records its observation time, byte count, modification time and bounded-read digest. Reads exceeding one MiB, changing during the read, ending with an incomplete line, or containing ambiguous identities are refused for process identification. The raw log remains unmodified.

Local validation passed eight diagnostic/ownership tests, the seven existing mounted/provider-proof tests, and Bash syntax checking. These tests cover the observed late-output distinction and refuse zero, multiple or mismatched process identities without sampling them. The E CI gate repeated all 15 Python checks and passed Objective-C SDK compilation in both feature configurations, both bundles and the runtime checks recorded above.

Both failed C attempts remain failed. E retained the existing 90-second mounted-route checks, provider checks, exact same-process URL assertions, native lifecycle and permission rules. If startup fails again, its permission pairs, timestamped console observation and exact-process classification can narrow the stopped call or launch boundary before an API-specific change is proposed. Any future asynchronous redesign remains subject to the readiness, ownership and atomic-publication constraints above.

No additional unchanged-source retry, deadline extension, verification bypass, automatic relaunch or permission fabrication is proposed by this investigation.

[contacts]: https://developer.apple.com/documentation/contacts/cncontactstore/authorizationstatus(for:).md
[camera]: https://developer.apple.com/documentation/avfoundation/avcapturedevice/authorizationstatus(for:).md
[avcam]: https://developer.apple.com/documentation/avfoundation/avcam-building-a-camera-app.md
[location-class]: https://developer.apple.com/documentation/corelocation/cllocationmanager/authorizationstatus().md
[location-delegate]: https://developer.apple.com/documentation/corelocation/cllocationmanagerdelegate/locationmanagerdidchangeauthorization(_:).md
[location-manager]: https://developer.apple.com/documentation/corelocation/cllocationmanager.md
[notifications]: https://developer.apple.com/documentation/usernotifications/unusernotificationcenter/getnotificationsettings(completionhandler:).md
