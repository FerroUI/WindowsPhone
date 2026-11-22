# Feature and fidelity status

## Baseline and meaning

The target is **Windows Phone 8.1 Update (2014, often called Update 1)**. Live folders belong to this target; Windows 10 Mobile's later navigation and visual treatment do not. Shared behavior from the 2015 Microsoft user guide is used where it also applies to 8.1 Update. Primary references and detailed original requirements are in [reference.md](reference.md).

**This repository does not claim 100% visual or feature parity.** An application can reproduce the shell and useful workflows, but it does not acquire the original operating system's cellular stack, device-management privileges, application runtime or retired cloud services. The table below describes code that is present, rather than asserting that every behavior has passed testing on every browser and physical device.

| Area | Present implementation | Remaining differences |
| --- | --- | --- |
| Start layout | Small, medium and wide tiles; deterministic packing; optional extra columns; persisted order and sizing; pin/unpin. | Exact device-specific tile metrics, tilt and rearrangement animation need visual comparison and tuning. |
| Live tiles | App data and notification counts feed tile content; photos and branded hub treatments are present. | Not the original background-agent tile system, push channel or complete animation catalog. |
| Background/theme | Accent selection, light/dark theme and image treatment within tile interiors. | Full original wallpaper/parallax behavior and every original theme resource have not been certified. |
| Live folders | Create by combining tiles; expand, rename and remove children; preserve child identity, size and secondary content targets. | Original folder animation and every arrangement rule remain unfinished. |
| Secondary tiles | Pin saved app content and browser favorites; persisted targets reopen their content. | Original third-party secondary-tile APIs and all live-content templates are not reproduced. |
| App list | Alphabetical app rows, search, letter jump, pinning and included-app management. | Exact original search motion, jump-grid metrics and package-manager behavior differ. |
| Back/Start/Search | Navigation journal, detail routes, transient UI dismissal, Start and search. | OS activation contracts, tombstoning and all original back-stack edge cases are not reproduced. |
| Recent apps | Cached native page previews, switch and close; the six most recent captured pages have bounded image sizes. | Browser-native video, camera and iframe content is not captured. No OS multitasking scheduler or suspended third-party processes. |
| Lock screen | Background, time/date, notification counts and unlock interaction. | In-app lock only; no host-device authentication, PIN security or complete original notification customization. |
| Action Center | Four configurable quick-action slots, notifications, clear/dismiss and deep-link actions. | Original OS notification integration remains incomplete. Radio toggles affect this app's represented state only. |
| Pivots/app bars | Large headings, section navigation and bottom icon/text commands with expanded menus. | Not every original Panorama/Pivot transition, spring, tilt or scrolling interaction is reproduced. |
| Touch | Direction locking, native vertical scrolling, shell gestures and tile drag arbitration. | Full multipointer combinations, original inertia curves, double-tap and pinch parity need further work and hardware testing. |
| Keyboard | Native text editing, selection and host software-keyboard integration. | The original Word Flow keyboard, swipe decoder, suggestion model and complete keyboard layouts are not reproduced. |
| Typography | Segoe UI when available; bundled Inter fallback; large lightweight headings and reusable type styles. | Original Segoe WP is not bundled. Font metrics and rasterization can differ substantially, so pixel-perfect text parity is not asserted. |
| Phone | Dial pad, local call history, contact dialing and host `tel:` handoff. | No SIM, incoming cellular calls, voicemail service, carrier controls or call-status confirmation. |
| Messaging | Local threads, contact navigation, drafts and `sms:` composition handoff. | No internal SMS/MMS transport, delivery receipts, carrier sync or background message ingestion. |
| People | Add/edit/delete contacts, detail pages, favorites and contact actions. | No Microsoft account/address-book synchronization or original social-network aggregation. |
| Outlook | Local/sample mail collection, read/flag/delete, detail view and external mail composition. | No mailbox authentication, Exchange/IMAP/Graph synchronization, push inbox, attachments or original account setup. |
| Calendar | Month navigation and local appointment creation/editing/deletion. | Account calendars, meeting invitations, recurrence completeness and all original calendar views remain unfinished. |
| Alarms | Local alarm definitions, enable/disable and weekdays; browser foreground checks. | Browser throttling/suspension prevents an OS-level alarm guarantee. No full original sound catalog. |
| OneNote | Local note list and note editing with persisted content. | No OneNote cloud notebook protocol, full formatting, embedded media or account synchronization. |
| Calculator | Local arithmetic, memory/scientific operations and keyboard interaction. | Exact original scientific landscape UI and every numerical edge case are not certified. |
| Photos | Camera Roll, albums/favorites views, import, selection, zoom controls, share and deletion. | Album management, editing/lenses, cloud photos and gesture-perfect pinch/rotation remain incomplete. Imported photos are resized JPEG application copies. |
| Camera | Permission-based browser viewfinder and actual capture into the Camera Roll. | No full Lumia camera controls, hardware shutter key, lenses, exposure stack or original capture performance. |
| Music/video | Imported local files, collection/now-playing views, actual playback and transport controls. | Codec support depends on the browser. Original subscription catalogs, DRM, playlist/album completeness and Xbox services are absent. |
| Internet Explorer | Bottom address field, URL/search navigation, favorites and an embedded web surface with external-open fallback. | The engine is the host browser, not IE11. Original tabs, history UI, Reading View, downloads and pinned-site contracts remain incomplete. Sites may refuse embedding. |
| Cortana | Local app/contact/note search, app-launch commands, time, contact dialing and web search. | No original cloud assistant, speech recognition, notebook, reminder intelligence or Microsoft account connection. |
| Maps | Place search and current-location handoff to OpenStreetMap. | No embedded original HERE/Bing map engine, offline maps, turn-by-turn guidance or full in-app map interaction. |
| Weather | Current conditions from Open-Meteo after explicit location permission. | Not the original weather service or complete multi-city/forecast experience. No weather is fabricated when the service is unavailable. |
| Files/Office | Local content entry points, backups, recent notes and online-document handoff. | No native Office document engine or general filesystem access. Online editors run in their own service. |
| Store | Browse, enable/disable, open and pin included apps. | No original Store account/catalog, purchases, package downloads, XAP/APPX execution or legacy account login. |
| Games | A playable tic-tac-toe game with deterministic minimax and session scores. | A built-in game is not original Xbox hub/service parity. No Xbox Live login, achievements or original catalog. |
| Settings | Accent, background, layout, quick-action assignments, content text scaling, app brightness, local settings and persistence. | No host OS management; incomplete original settings inventory, regional/accessibility configuration and account management. Brightness affects the app surface, not the physical display. |
| Battery Saver/Data Sense | Local battery-saver preference and measured serialized-data size. | No access to host battery policy, cellular usage or data allowance. Any displayed settings describe app behavior. |
| Backup/PWA | Local phone data, IndexedDB media, JSON export/restore, manifest and offline shell cache. | No Microsoft cloud backup/sync. External services and uncached remote web content still need connectivity. |
| Desktop host | Shared FerroUI UI, local document storage/recovery, URI handoff and document backup export. | Camera, imported media, location, sharing and backup restoration currently require the browser host. |

## Verification still required

The original reference material establishes behavior and visual direction. It does not replace comparison with original-device output. Before promoting any surface to a pixel-parity claim, record the original device/release, logical resolution, font, scaling and a matching application capture.

The highest-value remaining comparisons are:

1. Start tile bounds, gaps, typographic baselines and wallpaper alignment at representative 4- and 6-cell widths.
2. App-list and alphabet-jump measurements, app launch/return transitions, and back-stack behavior after repeated switching.
3. Action Center placement and partial gestures over Start, app content, editor focus and lock view.
4. Pivot scrolling, app-bar expansion and controls with a software keyboard visible.
5. Tile dragging combined with scroll, reordering, folder creation, cancellation and a second pointer.
6. Long lists, imported photographs/media, app suspension, browser storage limits and repeated backup recovery.
7. iOS Safari/Chrome and Android Chrome behavior for camera, sharing, media, browser history and installation.

Focused compile/unit/smoke checks are useful gates. They should not be described as exhaustive device testing, service parity or a completed visual audit.

## Service authenticity

Microsoft's [Windows Phone 8.1 lifecycle notice](https://learn.microsoft.com/en-us/lifecycle/announcements/windows-phone-8-1-end-of-support) and [Store package/update lifecycle notice](https://blogs.windows.com/windowsdeveloper/2018/08/20/important-dates-regarding-apps-with-windows-phone-8-x-and-earlier-and-windows-8-8-1-packages-submitted-to-microsoft-store/) establish the legacy platform's discontinued support and distribution constraints. Recreating a sign-in screen would not restore those services, so the app does not claim that local sample data or included-app management is an authenticated legacy account.

Modern integrations can be added behind the existing host-service boundary. They need their own provider configuration, explicit account flows and truthful delivery/synchronization states; they must not silently masquerade as restored original Windows Phone services.
