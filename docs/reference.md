# Windows Phone reference and fidelity specification

## Target release

The visual and interaction target is **Windows Phone 8.1 Update (2014)**, including live folders. The original `system` / `applications` Settings pivots are retained. Update 2 and Windows 10 Mobile introduced later designs, and are not the target. This document describes requirements and reference evidence; it does **not** certify complete visual or feature parity.

## Original behavior

| Surface | Reference behavior | Implementation consequence |
| --- | --- | --- |
| Start | An optional image shows through supported system tiles. | Keep the selected dark/light background visible between tiles; align the image across transparent tile interiors. [1] |
| Live folders | Drag a tile over another, name the folder, then expand it to arrange and resize children. Children keep their live content. | Preserve child identity and data; expand folders inside Start rather than in a generic modal. [2] |
| Pinned content | Folders can include apps, people, websites, maps, notes and other pinned items. | Model destinations separately from tile presentation. [3] |
| Navigation | Start and the alphabetical Apps list are adjacent horizontal views; Apps offers search and letter jumps. Back traverses history; holding Back exposes recent apps. Start preserves the current app. | Keep a navigation journal and per-app state. [4] |
| Tiles | Small, medium and wide; holding enables move, resize and unpin. | Use a stable packing model and persist layout changes. [4] |
| Live tile content | Photos cycle through pictures, People tiles change individual contact images, and wide tiles expose app-specific information. | Use stable keyed content and bounded, staggered updates. Disable animation for reduced motion and Battery Saver; this does not recreate an OS background-agent service. [21] |
| Lock | Drag upward to unlock; a photo background, five quick-status slots and one detailed-status provider are configurable. | Implement reversible drag and provider-driven status. A locked camera originally restricted gallery viewing/deletion until unlock; that behavior remains a fidelity gap. [4][19] |
| Action Center | Pull down from the top; partial opening exposes quick actions. Notifications support app-group dismissal and a two-finger clear gesture. | Keep the drawer separate from navigation history, preserve deep links and distinguish per-app Action Center/banner/sound/vibration preferences. [5][19] |
| Volume | Hardware keys open volume controls; an arrow expands them. Ringer/notifications and media/apps have independent channels, icon-based mute and a vibration toggle. | Retain independent stored levels while muted and map only supported application audio behavior to the host. [18] |
| Messaging | Conversations support contact selection, held-message forwarding/deletion and thread selection. Unfinished messages can be revisited. | Preserve recipient identity and draft state; separate local thread operations from host SMS handoff and delivery. [17][20] |
| Mail | Unread messages use a distinct color; tapping the left side of rows exposes selection checkboxes. Details offer reply/forward/delete. | Share reusable selection behavior while keeping mail mutations atomic. Account synchronization is a separate service boundary. [17] |
| Application bar | Primary actions sit at the bottom. Ellipsis exposes labels and additional text commands. | Provide a reusable Metro app bar that expands upward. [6] |
| Pivot and Panorama | In-app sections are reached horizontally, with strong typographic hierarchy. | Resolve horizontal section navigation separately from vertical scrolling. [6] |
| Browser | IE11 offers unlimited tabs, frequent sites, favorites, history, Reading View and pinned sites. The address-bar button can be assigned to tabs, favorites or refresh. | Persist local tabs and app-initiated navigation journals; expose the address shortcut and external-open path. Explicitly document local bounds and the host's cross-origin navigation boundary. [7][22] |
| Keyboard | Word Flow includes gesture trails, suggestions, contextual correction and contact names. | A host keyboard is a compatibility choice, not full Word Flow parity. [8] |
| Files | Browse phone/SD content, search, create folders, copy/move files, rename/delete and share. Folder properties report storage use. | Use a real library and explicit storage scope; preserve selected-file actions and hierarchy. [15][16] |
| Office | Recent/places views open Word, Excel and PowerPoint documents. Word supports editing, find, formatting and pinning; Excel switches worksheets; PowerPoint offers slides and speaker notes. | Provide native document workflows and actual interchange, while identifying unsupported document structures and cloud services. [17] |

## Visual implementation principles

Microsoft's Metro guidance emphasizes content, typography, negative space and consistent navigation. The original design templates include Segoe WP. [6][9] The app uses a dedicated Metro theme over FerroUI's WinUI control infrastructure. Square surfaces, monochrome icon geometry, large lightweight headings, neighboring pivot labels and bottom application commands should remain consistent across apps.

Exact pixel parity requires comparison at the same logical resolution, font availability and scaling as original hardware. Screenshots, original device captures and measured interaction traces should be used before describing any surface as pixel-identical. The cited material establishes behavior; it does not establish an exhaustive metric table for every model and release.

## Browser and service boundaries

The shell and local data workflows can be reproduced as an application. A browser application is not the Windows Phone operating system. The following distinctions remain explicit:

| Feature | Boundary |
| --- | --- |
| Phone and messaging | Local history, drafts and composition can persist. Host `tel:` / `sms:` handoff does not provide an internal cellular stack or delivery confirmation. |
| Radios and system controls | Wi-Fi, Bluetooth, flight mode and related settings inside the shell do not manage the host operating system's radios. |
| Store | Bundled app enable/disable is distinct from original Store downloads or execution of arbitrary XAP/APPX packages. Microsoft ended Windows Phone support and stopped distributing updates to Windows Phone 8.x. [10][11] |
| Cortana and accounts | Local search and commands must be distinguished from live external assistants, account authentication and cloud synchronization. |
| Camera | Actual capture uses browser media access; HTTPS and permission are required. Denial, missing hardware and an unanswered prompt are handled states. [12] |
| Contacts | Local contact data is independent from the host address book. Browser Contact Picker support is limited and only shares entries selected by the user. [13] |
| Web pages | A site may prohibit embedding using `X-Frame-Options` or CSP; opening externally remains available. [14] |
| Alarms and lock | Foreground alarm checks and an in-app lock do not replace reliable OS background alarms or host-device authentication. |
| Notification preferences | Action Center filtering is active. Saved per-app banner, sound and vibration flags do not currently produce independent host alerts. |
| Battery and storage | Optional host battery observations may be unavailable. Local JSON document sizes are not physical phone capacity, per-app power usage or carrier data usage. |

## Interaction verification

- Scroll Start vertically without activating a horizontal navigation gesture.
- Move between Start and Apps repeatedly and retain scroll positions.
- Pin, hold, resize, reorder and unpin a tile; reload and verify persistence.
- Create, expand, rename and dissolve a folder without losing child destinations.
- Search Apps, clear the query and use a letter jump.
- Navigate inside an app, press Start, reopen it and retain its state.
- Open recent apps with a held Back key; switch or close a card; validate the remaining journal.
- Open Action Center above Start, an app and the lock screen; dismiss it without extra navigation.
- Activate a notification and select the intended destination.
- Scroll a pivot page and use its inputs without gesture interception.
- Press Back while an app-bar menu is open; close the menu before leaving the page.
- Cancel a partial unlock drag and complete a subsequent one.
- Resize with the keyboard open; retain access to controls and navigation.
- Deny camera access and test unavailable persistence without breaking the rest of the shell.
- Verify keyboard focus, readable contrast and reduced-motion behavior.
- Pull Action Center partway for quick actions, expand it, swipe one app's group and clear all with two parallel fingers. A stationary or oppositely moving second finger must not clear all.
- Pan an Excel worksheet horizontally without activating Office hub pivots.
- Open two browser tabs, navigate independently, go Back and branch history; close the last tab and verify the old host page stops.
- Resize with a browser tab list or menu open and keep that state intact.
- Mute and unmute each volume channel without changing the other's level; keep the panel open during a slider drag.
- Change a lock status provider, close the settings page and verify the selected content on the lock screen.
- Hide one app's Action Center items without deleting them, then show and dismiss that app's group.
- Select several local mail items, delete them, restore one from deleted items and permanently delete another.
- Search a thread by body text, forward a held message, cancel composition and delete only that message.

## Primary references

1. [Microsoft: Deep dive into Windows Phone 8.1 Start screen backgrounds, 29 April 2014](https://blogs.windows.com/devices/2014/04/29/deep-dive-windows-phone-8-1-start-screen-backgrounds/)
2. [Microsoft: Windows Phone 8.1 Update, 30 July 2014](https://blogs.windows.com/windowsexperience/2014/07/30/windows-phone-8-1-update-brings-cortana-to-new-markets-new-features/)
3. [Microsoft: Windows Phone folders, 5 August 2014](https://blogs.windows.com/windowsexperience/2014/08/05/windows-phone-folders-a-new-way-to-organize-your-start-screen/)
4. [Microsoft Mobile: Lumia with Windows Phone 8.1 Update 2 user guide, 2015](https://www.bhphotovideo.com/lit_files/294643.pdf), printed pages 11–17 and 32–35. Shared 8.1 behavior is used; the later Settings taxonomy is excluded.
5. [Microsoft: Action Center, 14 April 2014](https://blogs.windows.com/devices/2014/04/14/put-your-lumia-at-the-heart-of-windows-phone-action-center/)
6. [Microsoft MSDN Magazine: Windows Phone design and navigation, December 2011](https://learn.microsoft.com/en-us/archive/msdn-magazine/2011/december/windows-phone-how-to-translate-common-design-principles-to-the-windows-phone)
7. [Microsoft: Internet Explorer 11 for Windows Phone, 7 May 2014](https://blogs.windows.com/windowsexperience/2014/05/07/a-closer-look-at-internet-explorer-11-for-windows-phone/)
8. [Microsoft: Word Flow, 9 May 2014](https://blogs.windows.com/devices/2014/05/09/worlds-fastest-texting-word-flow/)
9. [Microsoft: Windows Phone design resources, 27 July 2010](https://blogs.windows.com/windowsdeveloper/2010/07/27/windows-phone-7-design-resources-ui-guide-and-design-templates/)
10. [Microsoft: Windows Phone 8.1 support ended](https://learn.microsoft.com/en-us/lifecycle/announcements/windows-phone-8-1-end-of-support)
11. [Microsoft: Windows Phone Store package and update lifecycle](https://blogs.windows.com/windowsdeveloper/2018/08/20/important-dates-regarding-apps-with-windows-phone-8-x-and-earlier-and-windows-8-8-1-packages-submitted-to-microsoft-store/)
12. [MDN: MediaDevices.getUserMedia](https://developer.mozilla.org/en-US/docs/Web/API/MediaDevices/getUserMedia)
13. [MDN: Contact Picker API](https://developer.mozilla.org/en-US/docs/Web/API/Contact_Picker_API)
14. [MDN: X-Frame-Options](https://developer.mozilla.org/en-US/docs/Web/HTTP/Reference/Headers/X-Frame-Options)

15. [Microsoft: Introducing Files for Windows Phone 8.1, 30 May 2014](https://blogs.windows.com/windowsexperience/2014/05/30/introducing-files-a-file-management-app-for-windows-phone-8-1/)
16. [Microsoft Devices: Files official file manager, 9 June 2014](https://blogs.windows.com/devices/2014/06/09/files-official-file-manager-windows-phone-8-1/)
17. [Microsoft Mobile: Lumia with Windows Phone 8.1 Update user guide, 2015](https://www.bhphotovideo.com/lit_files/294642.pdf), printed pages 63 and 68–70 (contacts/messages), 74–76 (mail), 108–110 (Office) and 124–125 (Files). This is the Update guide, distinct from the Update 2 guide above.
18. [Microsoft: 4 ways to personalize the sounds on your Windows Phone, 19 June 2014](https://blogs.windows.com/windowsexperience/2014/06/19/4-ways-to-personalize-the-sounds-on-your-windows-phone/)
19. [Microsoft: 6 tips for using Lock screen and Action Center, 7 November 2014](https://blogs.windows.com/windowsexperience/2014/11/07/fast-not-furious-6-tips-for-using-lock-screen-and-action-center-on-the-go/)
20. [Microsoft: Lumia 430 reviewer guide](https://news.microsoft.com/wp-content/uploads/prod/2016/04/Lumia_430-ReviewersGuide.pdf), PDF pages 8–11 (hardware buttons, Action Center, Battery Saver and SMS merge/forwarding).
21. [Microsoft Devices: Making the most of Live Tiles, 23 September 2014](https://blogs.windows.com/devices/2014/09/23/making-live-tiles/)
22. [Microsoft: What's new in Internet Explorer 10 for Windows Phone, 7 December 2012](https://blogs.windows.com/windowsexperience/2012/12/07/whats-new-in-internet-explorer-10-for-windows-phone/), address-bar shortcut behavior retained in the 8.1 target.

23. [Microsoft Selawik source and known differences](https://github.com/microsoft/Selawik) and [release 1.01](https://github.com/microsoft/Selawik/releases/tag/1.01), the open Segoe UI replacement used as the portable font. Original Segoe WP metrics are a separate comparison target.

Additional original overview: [Microsoft's Windows Phone 8.1 announcement, 2 April 2014](https://blogs.windows.com/windowsexperience/2014/04/02/cortana-yes-and-many-many-other-great-features-coming-in-windows-phone-8-1/).
