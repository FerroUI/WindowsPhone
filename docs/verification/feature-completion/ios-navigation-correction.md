# Correction of the retained iOS navigation assessment

The earlier statement that restored Start and Settings screenshots have blank
Back/Start/Search icons was incorrect. The retained originals contain all three
glyphs. No rendering fix follows from those images.

On 26 November 2026, two independent checks decoded the original PNGs, verified
their file hashes, cropped the navigation row from recorded native geometry and
compared its raw RGBA bytes. All **16 inspected captures**, covering `81fabb3`,
`dc9e2a9`, `8749c8a` and `e430b21`, have an identical **828 × 108-pixel** row. Its
alpha is uniformly 255, and its SHA-256 is:

`1d770f09717890e24d6e687920de59d2f5ced8049ad25421d5f028e4c542206d`

The three button regions contain **535 / 1,860 / 464** bright opaque pixels,
respectively. The gray strip below the 54-point navigation row is the recorded
34-point UIKit bottom safe area.

![Unmodified crop of the retained navigation row](ios-navigation-inspection.png)

The [machine-readable correction](ios-navigation-correction.json) preserves each
image hash, observation sequence, crop rectangle, raw-region hash and per-button
count. Its temporary directory names identify the extraction used for analysis;
source artifacts remain identified by the original verification records.

This correction explicitly supersedes the appearance prose in
[`81fabb3/release.json`](../metro-followup/published/81fabb3/release.json), that
publication's frozen README, and the earlier
[capture comparison](../metro-followup/verified/dc9e2a9/ios-navigation-capture-comparison.json).
Those original evidence files and artifact hashes are preserved unchanged.

The comparison proves that the icons are visible in these retained frames. It
does not establish successful button interaction, every lifecycle path, or
original Windows Phone visual parity. The next iOS qualification extends the
existing six URL activations with real **Search → Back → Search → Start** taps,
fresh route observations and per-button pixel checks. Results belong to the
revision on which those additional checks execute.
