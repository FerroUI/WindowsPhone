# Metro and Outlook visual review — 41d7e3c

Reviewed the 14 original PNGs below and their corresponding native element snapshots from [run 45387122214](https://github.com/FerroUI/WindowsPhone/actions/runs/45387122214), source `41d7e3ceddb2d317edf3f1b8f22a7358d38bb7e6`. The review found the corrected dialog captions readable at the captured sizes, with no additional blocker in this selected set. The existing compact date-picker clipping remains visible.

The original [results.json](results.json) is retained unchanged. [manifest.json](manifest.json) records each retained artifact entry, byte count and SHA-256. All 29 original files were compared byte-for-byte with the downloaded ZIP entries and extracted originals. ZIP artifact `14919000632` is 7,535,748 bytes with SHA-256 `1a393ff06935c11d4a9a06c810b6e1782ab446ce5f66b6771a42a02e2eaf4976`. No image was cropped, resized or re-encoded; no snapshot was reformatted.

## Frames actually inspected

| Original PNG | Native snapshot | Observation |
| --- | --- | --- |
| [Discard confirmation, 320×740](outlook-unsaved-back-confirmation.png) | [JSON](outlook-unsaved-back-confirmation.json) | “discard unsaved changes?” wraps into two readable lines. Both **discard** and **cancel** captions are visible, without needing an ellipsis or interpreting an icon. Each enabled, hit-testable button is 132×45; its caption lies inside the button and phone bounds. |
| [Delete confirmation](outlook-delete-draft-confirmation.png) | [JSON](outlook-delete-draft-confirmation.json) | **delete** and **cancel** are visible beneath “delete this draft?”. Both enabled, hit-testable buttons are 172×45 in the 400-wide phone. The destructive command has no save glyph. |
| [Native multiline editor](outlook-native-multiline-editor.png) | [JSON](outlook-native-multiline-editor.json) | Recipient, subject and message are readable. The focused 360×220 body field contains two paragraphs separated by a blank line. The page explicitly describes local drafts and using the user's email app to send. |
| [Local draft list](outlook-local-drafts.png) | [JSON](outlook-local-drafts.json) | The saved item shows its subject, recipient, first-line preview and time. The instruction distinguishes tapping to edit from holding to delete. |
| [Restored draft](outlook-restored-draft.png) | [JSON](outlook-restored-draft.json) | Recipient, subject and the full multiline body match the editor snapshot, including the blank paragraph separator. The text is visibly present after restoration. |
| [Draft deleted](outlook-draft-deleted.png) | [JSON](outlook-draft-deleted.json) | The list displays “No drafts” and the local-save instruction after confirmed deletion. |
| [Narrow dark controls](metro-controls-320-dark.png) | [JSON](metro-controls-320-dark.json) | Ringer, Vibrate and Mute media show their off state. All three native switches occupy x20–300 inside the 320-wide phone; the volume tracks and values remain readable. |
| [Narrow light controls](metro-controls-320-light.png) | [JSON](metro-controls-320-light.json) | The same control geometry uses dark foregrounds on the light surface. Switch state, headings and volume values remain distinguishable. |
| [Switch state after touch/restoration](metro-native-switch-touch-restored.png) | [JSON](metro-native-switch-touch-restored.json) | The 400-wide settings surface shows three off switches, matching native `checked: false`; the two volume controls and navigation rows are visible. This still records the settled result, not the complete drag trajectory. |
| [Native Games pivot, player](metro-pivot-native-player.png) | [JSON](metro-pivot-native-player.json) | The selected **player** heading leads **collection** in the native `GamesPivot`; local-player content is readable. The page explicitly states that Xbox account, friends and online achievements are not connected. |
| [Reduced-motion Games, 320×740](metro-pivot-reduced-motion-320.png) | [JSON](metro-pivot-reduced-motion-320.json) | The static heading strip retains **collection**, **player** order with player selected. Local-player copy wraps within the narrow surface. This is the intentional reduced-motion path. |
| [Collapsed application bar](metro-app-bar-collapsed.png) | [JSON](metro-app-bar-collapsed.json) | The add icon and ellipsis are visible; the add caption is absent while the actual application bar is collapsed. |
| [Expanded application-bar labels](metro-app-bar-labels-overlay.png) | [JSON](metro-app-bar-labels-overlay.json) | The **add** caption appears. The bar grows upward from y767/height60 to y748/height79; the alarm row remains x460/y220/274×109 in both snapshots. The underlying row stops receiving hits while the overlay is open. |
| [Native date picker](shell-native-date-picker.png) | [JSON](shell-native-date-picker.json) | The popup remains usable but its month column visibly truncates long names such as September, November and December. This existing limitation is retained as evidence. |

## Scope and remaining limitation

The narrow settings frames retain the large, horizontally clipped page title. The control bounds described above do not imply that every title fits the viewport. The date-picker presenter is 256 pixels wide; the captured long month labels have 97-pixel text bounds. The compact popup has not been replaced with the original Windows Phone full-screen picker.

These are browser-rendered captures of this implementation. They establish the visible states and snapshot geometry listed here, not complete original-device parity or animation timing. Interaction, persistence and cancellation assertions are recorded separately in the unmodified run results. An original Windows Phone emulator was not booted for this comparison; see the [original-emulator investigation](../../../original-emulator.md).
