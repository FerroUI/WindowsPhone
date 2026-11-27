# Final browser hubs and notification review

The [browser job](https://github.com/FerroUI/WindowsPhone/actions/runs/45959413873/job/140373412884)
for candidate `d515348c699c22110def2dded8f7b8aea4b9cd03` passed with **112 checks,
198 captures and no recorded browser errors**. This scoped review covers seven
passing Settings/Office and Action Center scenarios. Motion and progress have
separate reviews.

[provenance.json](provenance.json) records artifact `15180379881`, ZIP SHA-256
`1582850c76a9d11b892dfa01a783b9d4d6bf2b28b56cce9ef2126d9273f93211`, the
candidate/synthetic-checkout identity and every preserved file hash.
[review.json](review.json) retains the exact checks, geometry and observations.
The parent dossier archives the complete browser results.

## Findings

Settings and Office remain readable at the 400-pixel shell width and at
320 pixels in both dark and light themes. Native headers remain above the
scrolling pane; the recorded System pane is scrolled to y430 while its header
stays at y125. Narrow header targets remain reachable. Office's focused search
page has no native Pivot mounted, and the passing interaction check returns to
the selected hub section.

The reduced-motion Office capture retains static section commands, readable
Places content and the app bar. The recorded reduced-motion check also exercises
Settings commands; there is no separate Settings reduced-motion screenshot in
this artifact, so no additional visual claim is made for that frame.

Action Center shows the expected newest-item group order. Group dismissal
removes Outlook while retaining Messaging and Phone; parallel clear yields the
empty state. The private-lock frame contains no notification groups and has
Clear all disabled. Its immediately following Alarms inspection contains
neither `LockScreen` nor `ActionCenter`, confirming fixture cleanup.

No concrete regression was found within these inspected states.

## Selected original captures

| Capture | Purpose |
| --- | --- |
| [Settings, 320 px light](raw/metro-settings-320-light.png) | Narrow native headings and row readability. |
| [Office Places, 400 px](raw/metro-office-places-native.png) | Native section layout and commands. |
| [Office, reduced motion at 320 px](raw/metro-office-reduced-motion-320.png) | Static section selection and reachable commands. |
| [Office search](raw/metro-office-search-page.png) | Focused native editor outside the Pivot. |
| [Chronological notification groups](raw/action-center-chronological-groups.png) | Group and item hierarchy. |
| [After group dismissal](raw/action-center-native-group-dismissal.png) | Unrelated groups remain. |
| [Private-lock Action Center](raw/action-center-private-lock-content.png) | Hidden content and disabled clearing. |

Thirteen original PNGs were visually inspected. The compact archive preserves
**seven PNGs and fourteen original inspection JSONs**, totaling **1,241,779
bytes**, including the other width/theme/empty-state inspections and the
immediate cleanup follow-up. Every `raw/` file matches its ZIP member byte for
byte. The additional inspected PNGs remain identified by hash in the review.

The notification fixtures contain document-backed local data. Real browser
touch input operates the retained native controls; it does not manufacture
Android/iOS provider authority. Native provider-key ownership and revocation
have separate Rust/Android evidence. Static captures and these checks do not
establish original Windows Phone font, raster or complete OS parity, nor every
Office ScrollViewer offset after hub reconstruction.
