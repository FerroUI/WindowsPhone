# iOS Simulator: candidate d515348

The [iOS job](https://github.com/FerroUI/WindowsPhone/actions/runs/45959414018/job/140373413519)
passed for candidate `d515348c699c22110def2dded8f7b8aea4b9cd03` through synthetic
checkout `75d8b175260015c8eb61c82ab87d525bbba15221`. Both have tree
`ad870fb6a20b630070b6b9e0287dd0eec665f22c`.
[provenance.json](provenance.json) records artifact `15181330522`, verified ZIP
SHA-256 `4e949354ad6ded58924d3107a437558425dedfdb0ed4d5bf16c6f489bbab230d`,
source identity and original file hashes. [review.json](review.json) contains
the full inspection and its qualifications.

## Device and native navigation results

[device-tests.json](raw/device-tests.json) records **seven passing checks**,
no error and no cleanup errors. These include Contacts create/query,
update/query and delete/query, observed permission/capability states and the
Simulator's read-only brightness behavior. Six URL activations and four real
`XCUIElement.tap` inputs completed within the same native launch/process.

| Native input | Fresh observed route | Observation timing |
| --- | --- | --- |
| Search | `Search` | After tap start and return. |
| Back | `Start` | After tap start; 129 ms before the input API returned. |
| Search again | `Search` | After tap start and return. |
| Start | `Start` | After tap start and return. |

Every result has a newer observation sequence than its matching request and a
timestamp after that request and tap start. The Back result does not support
a claim that all observations followed API return; native state can change
while the tap call is still completing. The original request, driver and result
JSONs preserve these times unchanged.

## Original captures and navigation pixels

| Original capture | Scope |
| --- | --- |
| [Start](raw/start.png) | Tiles, shell status and navigation within the recorded viewport. |
| [Settings device-services leaf](raw/settings.png) | `DeviceSettingsRoot`, not the new Settings root hub. |
| [People device contacts](raw/people.png) | Device-contact list/search and app-bar commands. |
| [Wallet entrance frame](raw/wallet.png) | Visible entrance perspective; **not a settled geometry/parity frame**. |

The Wallet search field and app-bar edges are visibly slanted. This is consistent
with an entrance projection, but the native observation does not record the
animation phase. The original is retained to show the observed frame, not to
claim final resting layout.

Independent decoding of **nine original frames** confirms a nonblank
828 × 108-pixel navigation row at `(0, 1616, 828, 1724)` in right/bottom-exclusive
pixel coordinates. Every row has raw RGBA SHA-256
`1d770f09717890e24d6e687920de59d2f5ced8049ad25421d5f028e4c542206d`,
uniform alpha 255 and **535 / 1,860 / 464** bright opaque pixels in Back / Start /
Search regions. This matches the previously
[corrected navigation assessment](../../../feature-completion/ios-navigation-correction.md).
The older blank-icon claim remains retracted. The gray strip below the row is
the recorded **34-point UIKit bottom safe area**.

## Preservation and boundaries

This subset preserves **32 original top-level JSONs, four original PNGs and
two original text files**, totaling **947,255 bytes**. Each file matches its
downloaded ZIP member byte for byte. Images were not cropped or recompressed;
xcresult and executable binaries are not duplicated. Four full screenshots were
visually reviewed; five additional frames were decoded for navigation pixels.

The target is **iOS 26.5, iPhone 11 Simulator**, with a 414 × 814-point application
viewport at scale 2. No calls or messages were sent. Physical brightness,
carrier/entitlement behavior and actual hardware remain outside these simulator
results; network-path availability is not Internet reachability.

This run qualifies the recorded native paths and taps. It does not newly
exercise the Settings/Office root hubs, Action Center mixed-store gestures,
progress motion or Start/All Apps paired paging. Those have separately scoped
browser/headless evidence. No original Windows Phone reference-device, font or
raster comparison, full OS equivalence or all-platform publication acceptance
is inferred from this record.
