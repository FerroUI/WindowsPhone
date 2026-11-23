# Device services, launcher and live tiles

The launcher, Phone, Messaging, People, Maps and Settings use shared Rust contracts. FerroUI owns their retained controls, rendering, layout, navigation input, virtualization and app lifetime. Host providers translate typed operations to the public APIs available on the device. Native Android call presentation is also implemented independently of the mounted Rust view so Telecom can present an incoming or ongoing call while the launcher is not visible.

This document describes implemented source. Verification records identify the exact revisions that passed compilation, packaging and rendered interactions. Emulator and simulator checks do not establish physical SIM, carrier, Bluetooth, notification-delivery or OEM behavior.

## Contract and ownership

`phone-host::platform` defines `DeviceSnapshot`, `DeviceRequest`, `DeviceResponse`, `DeviceControl`, `DevicePermission`, `DeviceRole` and `DeviceOperation`. Host features advertise implemented protocol families; the current snapshot advertises operations that are usable now. An installed plugin therefore does not imply permission to read contacts or send a message.

| Boundary | Enforcement |
| --- | --- |
| Input | Bounded identifiers, text, recipients and attachments; finite normalized hardware levels; known call actions; typed settings pages. |
| Completion | Request/response kind and thread, message, application or client identity must agree. Query channels use the newest request. |
| Permission changes | Nonempty personal results require the latest usable operation. Contacts additionally require granted or limited address-book access. Revoked snapshots clear corresponding transient data; empty cleanup results remain admissible. |
| Lifetime | Runtime and phone-document epochs reject old completions. Provider observations are drained in bounded batches with reentrancy protection and a lifetime check between events. |
| Persistence | `DeviceStore` is outside the backed-up phone document. Contacts, calls, OS notifications, device threads, location and capability observations do not enter ordinary phone backups. |
| Side effects | Requests still undergo fresh native role, permission, identity and foreground checks. Acceptance, composition, submission and delivery are distinct outcomes. |

Native timestamps use Unix milliseconds; call duration uses whole seconds. Telecom accounts and SMS subscriptions have separate identities. A SIM selection is never interpreted as a Telecom account handle.

Attachment selection carries a composer `client_id`. A canceled selection preserves that draft's existing attachments. A result for a different draft is discarded and its leases are released. Direct sends have their own correlation identity; a receipt for an earlier send cannot clear a newly edited draft. Selected attachments are expiring opaque leases rather than caller-supplied filesystem paths. Accepted native transports copy payloads into their own private lifetime before the UI relinquishes the draft.

An MMS download can reveal previously unknown participants and resolve to a different conversation. `MessageDownloaded` is correlated to its original message ID; only the still-selected initiating conversation follows its resolved thread. A later result cannot navigate a different conversation or an open composer.

`DeviceNotificationPolicy` carries the phone's saved message notification preferences to app-owned native announcement providers, including when the Rust UI is absent. A weak runtime subscription updates it on settings changes and document replacement and is disposed on closing. This policy neither grants notification permission nor changes another application's notification channels or required call presentation. The host operating system retains final authority.

## Hardware and Settings

The shared inventory has **25 controls** and **43 settings destinations**. Four chosen primary actions remain visible in Action Center; expansion exposes the additional controls the current provider reports. Settings, the volume panel, Battery Saver and quick actions use the same observations and operations.

Controls cover Wi-Fi, Bluetooth, cellular, flight mode, location, rotation, brightness, automatic brightness, flashlight, ringer/media/alarm volume, ringer mode, quiet hours, battery saver, hotspot, NFC, screen timeout, night light, casting, nearby sharing, VPN, audio output, account synchronization and data saver.

Each control reports one of five access modes:

| Access | UI and dispatch |
| --- | --- |
| Writable | Sends a validated typed value; the next native observation supplies the displayed value. |
| Panel | Opens the appropriate public system panel or settings page. |
| Permission | Exposes the OS consent or special-access flow required for the operation. |
| Read-only | Displays the actual observation without offering a mutation. |
| Unavailable | States the limitation instead of creating a working-looking local toggle. |

Android uses public settings, role and permission APIs. Modern Android restrictions on radio changes are represented as panel access. Brightness/rotation/timeout require actual settings-write authority where applicable; torch, audio streams, power and location use the available device APIs and report failures. The status bar and native volume panel consume observed values. In-app feedback volume, represented browser settings and the preference to use location inside this app remain separate settings.

iOS supplies public brightness and flashlight controls when available, `MPVolumeView` and route-picker panels, foreground location and power observations, and supported Settings entry points. It does not use private Settings URL schemes to imply unavailable radio control. Desktop providers offer supported native settings destinations; they do not advertise an unimplemented system mixer or radio controller.

## Android launcher and communications

The ordinary application and HOME launcher remain distinct APK flavors. Only the HOME flavor declares the launcher category. Becoming the default home uses Android's role selection. HOME activation returns to Start; native predictive/system Back enters the shared navigation journal and dismisses transient UI before leaving an app. At Start, a selected launcher consumes Back; an ordinary application may let the operating system close its activity.

The main All apps list combines registered phone apps and the current-profile native catalog in a virtualized alphabetic list. It supports search, letter jumps, opening, Start pins, native shortcuts, app information and OS-confirmed uninstall when advertised. Native identities are opaque observed components, and actions revalidate them against a fresh catalog. Shortcut enumeration and launch require the appropriate active HOME authority. The independent Device apps application retains grouped discovery, notification previews, pin management and service diagnostics. Native app rows currently use shared iconography; loading every package's own icon and the shared-only list's sticky-header behavior remain separate gaps.

The default Phone role includes `ACTION_DIAL`, a real `InCallService`, incoming and ongoing call UI, notification actions, answer/reject/end, mute, speaker, hold/resume and DTMF. The shared Phone app displays active calls, elapsed time, device call history and available call accounts. External dial activation fills the keypad; it never places a call by itself. Android Telecom retains emergency-call handling and all carrier routing decisions.

The messaging provider implements the default SMS role's receive/compose/respond entry points. SMS has multipart sending and independently ordered sent/delivery results. MMS has bounded attachment selection, MIME parts, subject/group recipients, send/download, provider persistence and explicit retry. A private durable journal and jobs survive the absence of the launcher view. The AOSP MMS codec is vendored with its exact upstream revision, source hashes and license; package workflows verify that provenance. The UI supports device thread search, dated messages, actual delivery states, unread updates, attachment access, download retry, copy/forward and confirmed deletion. No universal Android RCS transport is claimed.

The notification listener exposes observed notifications and generation-checked opaque actions, inline replies and dismissal. It revalidates native `PendingIntent` ownership instead of accepting an executable action from the shared UI. Notification access remains an explicit user-controlled setting. Contacts use the system provider and native editing flow; the shared People app also retains independent local contacts and explicit copy/import workflows.

## iOS standard and optional carrier providers

The standard UIKit build remains useful without carrier entitlements: authorized full/limited Contacts access and editing, native message composition with real accepted/canceled/failed outcomes, document-picked attachment leases, public hardware controls, system settings, notification policy, and MapKit handoff. Composer acceptance is not a delivery receipt. iOS does not advertise replacement of SpringBoard, a complete installed-app catalog, system-wide notification enumeration or Android-style notification replies.

The explicit `ios26-telephony` Cargo feature adds a separate Swift provider:

- LiveCommunicationKit cellular dialing and the call history exposed to an eligible default dialer.
- TelephonyMessagingKit SMS/MMS sending and incoming SMS, MMS and supported RCS text/files.
- A private SQLite message journal, provider-owned attachment files and notification ingestion outside the phone document and its backups.
- Runtime checks of OS/framework support, real signed entitlement values, current default-app eligibility and service availability. A feature flag or an Info.plist value is not treated as entitlement authority.

The checked-in default bundle does not add carrier entitlements or a provisioning profile. Apple requires the carrier messaging entitlement and user-selected default role; ordinary distribution is also restricted to eligible iPhone users and regions. Simulator builds must report these carrier operations unavailable. There is no implemented outbound RCS chat, carrier in-call answer/end/mute/DTMF control, or default-navigation entitlement. These distinctions prevent a supported handoff from masquerading as a privileged service.

Native iOS notification controls cannot independently implement every Windows Phone banner/center/vibration combination. The provider suppresses its own alerts when disabled, honors permitted sound, makes non-banner alerts passive, redacts content under the saved lock-screen preference, and retains iOS notification settings as the final constraint.

## Embedded map and location

Maps renders Web Mercator raster tiles through FerroUI `Canvas` and `Image` controls, with pan, pinch, wheel and keyboard zoom, saved-place markers and visible OpenStreetMap attribution. At most 16 visible placements and two network requests are active for the viewport. No background map region, area download or prefetch is implemented. Scripted render tests install a local generated tile fixture before any app navigation so automated pan/zoom never sweeps the public tile server.

`HostRequest::MapTile` uses the existing network service boundary. Coordinates, zoom and 256-pixel PNG payloads are bounded before image decoding. `phone-sdk::images::TransientImage` gives decoded tiles scoped ownership; dropping the last lease releases their private identity without polluting persisted asset tombstones. Old viewport/document completions cannot publish into a replacement view.

The native cache uses app-private files, HTTP expiry/validators and conditional revalidation. The browser uses normal HTTP caching plus bounded CacheStorage. Both cap tile payloads at 512 KiB and the cache at 2,048 entries/64 MiB, honor `no-store`, and use a seven-day fallback when expiry is unavailable. Fresh entries are not evicted merely to make room. Native requests identify the app; browser requests preserve a valid referrer. The cache supports repeated interactive viewing; it is not an offline-map product. Hosts needing another licensed map service can replace the network service at composition.

One-shot Locate tracks its own request revision, route/document epoch and cancellation state. GPS and viewport camera state are transient; only explicitly saved places enter the document. Place pins route through the registered app's typed activation, including after restore or folder nesting. Directions preserve explicit origin and travel mode and hand off to supported native/external navigation. Internal route calculation and turn-by-turn guidance are not implemented.

Cancellation prevents delivery and drops queued SDK work. An already executing synchronous native HTTP request may finish within its transport timeout; it does not retain a live UI target or mutate a replaced document.

## Start and SDK tile ownership

Start adds per-tile label, badge visibility, live-content preference, background color/photo and explicit resize/reorder/folder commands. Folder extraction and dissolution preserve complete child records, including secondary paths, presentation and notifications.

Reusable Text, Flip, Photo, Cycle, List and Progress templates render bounded tagged notifications. A tile holds at most five cards; a matching tag replaces its card. Scheduled activation and expiry use the shared transient clock and continue to expire data correctly when decorative motion is reduced. Clock ticks never write the phone document.

An app obtains `AppContext::tile_updater()`, enumerates its owned pinned tiles, and publishes or clears notifications. The updater holds weak session/document ownership. It rejects other apps' tiles and becomes invalid after document replacement, app reset/close or host shutdown. It does not create an OS background agent or a push channel. `AppRegistry::bind` registers builtin and extension identities separately, so builtin content pins follow the same validated app-link path without making builtins disableable extensions.

## Verification and remaining qualification

Portable tests cover contract validation, stale response and permission rejection, correlated composer selection/send outcomes, notification policy disposal, tile ownership/expiry, typed builtin pin activation, map bounds/lifetime/cache and image leases. Native checks separately compile/link platform code and verify manifests, package contents, JNI/Objective-C/Swift entry points and alignment. Rendered browser, Android emulator and iOS simulator gates then exercise the mounted UI.

The isolated Android device fixture temporarily selects dialer/SMS roles, restores their prior owners, receives a modem-injected SMS through the actual provider path, and exercises an incoming emulator call through its native Answer control. It does not change HOME or call a real destination. The iOS fixture tests contacts and brightness roundtrips, standard and optional-provider builds, unsupported simulator carrier capabilities, route activation and safe-area geometry. Neither fixture sends a real carrier message.

Physical carrier registration/delivery, MMS APNs and OEM provider differences, Bluetooth/headset routes, real flashlight/brightness behavior, permission revocation across process death, regional entitlement provisioning and store/distribution qualification still need direct target-device evidence. Windows Phone push services, retired Microsoft services, original kernel/system UI, Word Flow and original-device pixel/motion fidelity are outside the behavior established by these implementations.

## Primary API and provider references

- [Android InCallService and default dialer](https://developer.android.com/reference/android/telecom/InCallService)
- [Android roles](https://developer.android.com/reference/android/app/role/RoleManager)
- [Android system settings](https://developer.android.com/reference/android/provider/Settings)
- [Android Telephony provider](https://developer.android.com/reference/android/provider/Telephony)
- [Apple TelephonyMessagingKit](https://developer.apple.com/documentation/telephonymessagingkit)
- [Apple carrier messaging entitlement](https://developer.apple.com/documentation/bundleresources/entitlements/com.apple.developer.carrier-messaging-app)
- [Apple default-app capabilities](https://developer.apple.com/documentation/updates/defaultapps)
- [OpenStreetMap tile usage policy](https://operations.osmfoundation.org/policies/tiles/)
