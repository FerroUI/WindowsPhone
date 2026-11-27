# Android mounted UI: candidate d515348

The [mounted-UI job](https://github.com/FerroUI/WindowsPhone/actions/runs/45959413903/job/140373412992)
passed for candidate `d515348c699c22110def2dded8f7b8aea4b9cd03`. The artifact's
checkout marker is synthetic merge `75d8b175260015c8eb61c82ab87d525bbba15221`;
its complete tree equals the candidate's
`ad870fb6a20b630070b6b9e0287dd0eec665f22c` tree. This record qualifies the
Android run and does not establish acceptance of the candidate's other CI jobs.

[provenance.json](provenance.json) records artifact `15180472180`, verified ZIP
SHA-256 `a51c0262dd135bb4f7bdb76796958702997042d470e10307fd29a37f5e4acc1a`,
source identities and checksums for every preserved original.
[review.json](review.json) contains the detailed inspection and scope limits.

## Observed notification behavior

The native Shell fixture is admitted with both notification operations available,
then its title and body render within the measured Action Center bounds.
Revoking the listener removes all three admitted native rows, both operations
and the fixture text. The revoked screenshot preserves local Messaging and
Outlook groups. A subsequent native Back input closes Action Center; a fresh
Start observation retains the unread Messaging badge at 1. Listener access and
the HOME role are restored to their recorded prior holders.

| Original capture | Observation |
| --- | --- |
| [Action Center with native notification](raw/device-action-center.png) | Unique Shell fixture visible; three native rows admitted. |
| [Action Center after revocation](raw/device-action-center-revoked.png) | Native fixture absent; local groups remain. |
| [Wallet with active HOME role](raw/home-role-wallet.png) | Wallet renders while host mode is launcher and HOME is active. |
| [Start after HOME and Back](raw/home-role-back-start.png) | Start remains mounted after Android HOME followed by Back at Start. |

The notification scenario performs a native status-bar tap, listener revocation
and Back. It does **not** perform horizontal group dismissal, two-finger clear,
clear-all or group-close activation. The fixture already fits at scroll offset
zero; no fallback scrolling occurred. Provider-key gesture ownership and
lock-screen privacy require their separately identified evidence.

## Observed shell navigation

Both the application package and launcher package record Wallet → Start through
real Android Back input. With the actual HOME role selected, Android resolves
the launcher without an explicit component; Android HOME returns Wallet to
Start, and Back at Start keeps the selected HOME focused. Native process
identity remains stable within each of these separate phases.

The inspected Start, Wallet and notification frames contain the same nonblank
720 × 108-pixel shell navigation row. This demonstrates those retained frames;
the Android run does not exercise Start/All Apps horizontal paging, document-
restore capture ownership or motion-frame continuity.

## Preservation and limits

The compact subset contains **19 original JSON files, four original PNGs and
10 small original text files**, totaling **862,445 bytes**. Every `raw/` file
matches its downloaded ZIP member byte for byte. Images were not cropped or
recompressed. Five screenshots were visually inspected; the additional
`app-back.png` is hashed in the review and remains in the identified artifact.

The target is an Android API 36 x86_64 emulator with software rendering,
720 × 1440 physical pixels and a 360 × 628-point application viewport. Android
system bars remain visible outside the emulated shell. The preserved result
also reports incoming-call and MMS-codec checks, but this review independently
examines notification and navigation evidence. No original Windows Phone
reference-device, font or raster comparison was performed, and no full OS or
100 percent visual-parity claim follows from these captures.
