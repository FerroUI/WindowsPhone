# Calendar, call activity, native grouping and iOS observations

This implementation continues from `6fbd965` and retains the reviewed FerroUI
pin `ec86b9c`. It adds working application and platform features while keeping
their execution and original-device fidelity boundaries explicit.

## Implemented behavior

| Area | Change |
| --- | --- |
| Calendar tasks | Overflow → tasks; native editor/detail/list/context surfaces; optional due dates and independent reminder times; low/normal/high priorities; notes; completion/reopening; advance/postpone; search, filters and sorting; confirmed single/bulk deletion; typed activation and pinned task tiles. |
| Task storage | Versioned `phone.calendar` AppStorage, up to 512 tasks within the shared 256 KiB namespace budget; captured revision/epoch checks; monotonic identities; atomic persistence; draft retention and explicit discard. |
| SDK reminders | Pure registered providers collected independently of the active app, with 512 reminders per provider and 4,096 total; deterministic ordering, bounded foreground queue, snooze, dismissal and validated activation. Registry/session/document/host ownership is rechecked across synchronous callbacks. |
| Reminder privacy | Calendar and app reminders redact private title, time, location and detail while locked with notification details disabled. Generic reminder controls remain usable; explicit open unlocks the in-app surface. |
| Phone activity | Shared Start/lock badges use the actual Android unacknowledged missed-call total. Explicit acknowledgement uses a one-use 120-second receipt covering at most 500 exact observed records, with at most four live receipts. Later calls remain new. Local history persists acknowledgement separately from missed-call direction. |
| Native app list | Reusable measured sticky/push-off headers and native virtualized jumps; retained row/inset restoration; search/document reset. English A–Z/# grouping remains the current collation boundary. |
| Recent apps | Built-in and SDK extension previews share one six-entry cache, distinct identities, privacy gating, bounded bitmap dimensions and close/epoch cleanup. |
| iOS network | Lifecycle-owned Network.framework path observation. Unknown retires a stale VM value; resume creates a fresh owner; late callbacks cannot restore a retired observation. Path availability does not establish Internet/service reachability. |
| iOS interaction qualification | Four genuine XCTest taps—Search → Back → Search → Start—extend the six original URL activations. Fresh native observations, actual button geometry/hit testing and bounded screenshot checks retain independent input evidence. |

Android call access is filtered by current permission, AppOps and dialer role.
Receipts capture ID/date identities and update only `CallLog.Calls.NEW` for
missed records still marked new. `IS_READ` is not inferred from viewing history.
Provider count queries are independent of the 200-row displayed history page.
Foreground bootstrap, content observation, monotonic query revisions, bounded
retry and authority invalidation prevent old reads from reviving stale counts.
The location channel remains independent of the new call observation channel.

The CallLog regression runs only in an explicitly selected fresh debug x86_64
emulator. It refuses pre-existing new missed calls, inserts identified fixture
rows, captures two of them, adds a later call, verifies **3 → 1**, checks every
retained `IS_READ` flag and rejects token reuse. Cleanup deletes only the
recorded fixture identities and verifies the original row state. It makes no
RF call or UI-interaction claim.

## Local preflight and CI boundary

The [preflight record](preflight.json) preserves source hashes and complete local
logs, including the initial Calendar DSL property failure and its correction.

| Local check | Result |
| --- | --- |
| Domain / host Rust execution | 37 / 35 passed; one host test ignored |
| Portable iOS, standard / carrier configurations | 11 / 11 passed |
| JavaScript unit tests | 77 passed |
| Java production call-receipt policy | 11 assertions passed |
| Android call-fixture verifier/guard regressions | 8 passed |
| Existing Android device / bootstrap verifiers | 3 / 19 passed |
| iOS observation / cleanup verifiers | 11 / 11 passed |
| Production grouped-list geometry | 4 isolated Rust tests passed |
| Integrated Rust tests metadata | Passed for state, controls, SDK, Calendar, alarms, system, runtime, Phone and iOS; final Calendar/alarm refresh also passed |
| Java parsing, Bash/Node syntax and whitespace | Passed; Java parsing does not resolve Android SDK symbols |

The candidate CI must execute the newly compiled test bodies, the rendered
Calendar/Phone workflows, the real Android CallLog fixture, and the new native
iOS navigation/path checks. No fresh browser/native execution or publication is
claimed by this preflight record.

New focused coverage includes a 3,000-row variable-height native grouped list,
extension preview rendering and lifecycle, 24 Calendar/SDK/reminder Rust tests,
seven Calendar browser groups and two Phone browser groups. Those additions are
requirements for the candidate execution, not substitutes for it.

## Corrected historical assessment

The [iOS screenshot correction](ios-navigation-correction.md) supersedes the
earlier blank-navigation interpretation. All three icons are visible in all
16 inspected retained frames. Their PNGs, artifact hashes and frozen original
publication records remain unchanged. No speculative redraw fix was introduced.

## Primary references and limits

- [Microsoft Lumia Windows Phone 8.1 Update user guide](https://www.bhphotovideo.com/lit_files/294642.pdf), printed page 50: Calendar task entry and held-row actions.
- [Microsoft Devices: task priorities and reminders](https://blogs.windows.com/devices/2015/02/05/tick-tock-time-management-tips-lumia/).
- [Android CallLog.Calls](https://developer.android.com/reference/android/provider/CallLog.Calls): distinct acknowledgement and consumption fields.
- [Android default phone application](https://developer.android.com/develop/connectivity/telecom/dialer-app): platform-owned role and call behavior.
- [Apple Network path monitoring](https://developer.apple.com/documentation/network/nwpathmonitor): path observation, distinct from application-service reachability.

Task schedules retain the existing local wall-clock calendar model. There is
no new account synchronization, OS wake-up/background-delivery guarantee, iOS
missed-call acknowledgement provider, or original Windows Phone runtime ABI.
Physical carrier/permission/device behavior and exact original-device fidelity
retain their separate qualification requirements in [the parity record](../../parity.md).
