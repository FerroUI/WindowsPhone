# Browser shell motion: candidate d515348

This review covers **531 observations from eight original motion traces**, plus
four original PNG/inspection-JSON pairs, from the [browser job](https://github.com/FerroUI/WindowsPhone/actions/runs/45959413873/job/140373412884)
for candidate `d515348c699c22110def2dded8f7b8aea4b9cd03`. Both directions of
Start/All Apps paging now contain the real outgoing image and the live incoming
page. The recorded turnstiles, interruption, privacy cancellation and motion
preferences also satisfy their existing assertions.

[review.json](review.json) contains the calculated measurements and qualifications.
[provenance.json](provenance.json) identifies artifact `15180379881`, its verified
ZIP SHA-256 `1582850c76a9d11b892dfa01a783b9d4d6bf2b28b56cce9ef2126d9273f93211`,
and the size/hash of every preserved file. All 16 `raw/` files match their ZIP
members byte for byte, totaling **1,531,012 bytes**. No image or frame JSON was
edited, cropped or recompressed.

The workflow checkout was synthetic merge
`75d8b175260015c8eb61c82ab87d525bbba15221`; its complete tree and the candidate's
tree both equal `ad870fb6a20b630070b6b9e0287dd0eec665f22c`, independently checked
locally. Run/job/artifact association comes from the separately archived CI
metadata; this UI ZIP does not contain an embedded commit marker.

## Paging and compositor geometry

The browser viewport is 1280 × 936. The phone surface occupies `(440, 61, 400,
820)` and the resting page occupies `(440, 87, 400, 740)`, in root coordinates.
Every observed outgoing image reports logical dimensions of 400 × 740.

| Original trace | All observations | Paired native observations | Subsequent matching compositor observations | First clean observation, ms |
| --- | ---: | ---: | ---: | ---: |
| [Start → All Apps](raw/metro-shell-paging-apps-frames.json) | 66 | 19 | 18 | 647.9 |
| [All Apps → Start](raw/metro-shell-paging-start-frames.json) | 67 | 21 | 20 | 645.8 |

In both directions, the first native pair carries the 168-pixel drag into a
232-pixel remaining entrance. Start → All Apps progresses from incoming/outgoing
translations `(+232, −168)` to the final observed pair `(+0.195, −399.805)`.
The reverse progresses from `(−232, +168)` to `(−0.020, +399.980)`.
The directed separation remains exactly 400 in the recorded native values,
and the incoming displacement decreases monotonically toward zero.

All **40 paired native observations** keep both surfaces at opacity 1, with
enabled and hit-test-visible states false. The outgoing surface is never a
native hit target. The 38 subsequent compositor observations independently show
both surfaces moving, with their adjacent edges separated by at most
`5.7 × 10⁻¹⁴` from the expected numerical coordinate relationship. That small
residual describes floating-point bookkeeping; it is not a claim of physical
display precision.

There is a visible distinction between native state and completed-render
readback: the **first paired inspection in each direction** already contains
the new translations while both compositor bounds still report `x = 440`.
The next inspection and every remaining paired inspection agree with the
translations. These observations support continued paired motion after that
initial readback. They do not establish zero-frame release latency or pixel
adjacency at every instant. Raw observations, including the initial mismatch,
are preserved without filtering.

## Turnstiles, interruption and privacy

| Original trace | Observed phases and result |
| --- | --- |
| [Forward turnstile](raw/metro-navigation-forward-frames.json) | 15 outgoing-image observations, then 19 incoming-projection observations. Outgoing angle increases from 0° to the last sampled +34.017°; incoming begins at −80°. Outgoing rendered width changes from 400 to as little as 303.702. |
| [Backward turnstile](raw/metro-navigation-backward-frames.json) | 13 outgoing-image observations, then 18 incoming-projection observations. Outgoing angle decreases from 0° to the last sampled −59.931°; incoming begins at +50°. Outgoing rendered width reaches 200.711. |
| [Rapid Back](raw/metro-navigation-interrupted-frames.json) | Three captured-image observations precede the return to Settings at 99.0 ms. All 60 observations from the return onward contain no outgoing image; the superseding incoming rotation appears in 20 observations and then releases. |
| [Lock cancellation](raw/metro-navigation-lock-frames.json) | Two captured-image observations precede the first lock observation at 77.8 ms. All 63 observations from that point have no visible private page or image and have a transparent, untransformed outgoing surface. |
| [Reduced motion](raw/metro-navigation-reduced_motion-frames.json) | All 68 observations have no outgoing image or incoming/outgoing projection; the destination remains usable. |
| [Battery saver](raw/metro-navigation-battery_saver-frames.json) | All 68 observations have no outgoing image or incoming/outgoing projection; the destination remains usable. |

During every sampled turnstile outgoing phase, the live destination is mounted,
hidden and disabled. The outgoing image retires before the live incoming
rotation; input ownership is then restored. The recorded hinge is −200 and
camera depth is 833⅓ for the 400-wide page, with no numerical deviation from the
implemented relationships. The last sampled outgoing angles above precede
retirement; they are not substitute endpoint specifications.

The lock peer appears in 62 observations. One intermediate inspection at
96.3 ms omits that peer, while still exposing no private page/image; the stronger
privacy check above includes that observation. This does not claim uninterrupted
raster visibility of the lock screen between samples.

All seven unlocked traces finish with a usable, fully opaque page at its resting
bounds, both transforms removed, outgoing opacity zero and no outgoing image
source. The lock trace finishes with the private page absent and the retiring
image/transform released. The existing `assertShellPaging`,
`assertNavigationChoreography` and `assertUnprojectedFrames` functions were
replayed on the six corresponding preserved traces and all passed. Additional
analysis checked the complete interruption and post-lock sequences.

## Screenshot inspection and scope

| Original PNG and inspection | Visual observation |
| --- | --- |
| [Restored Start](raw/metro-shell-paging-restored.png) · [inspection](raw/metro-shell-paging-restored.json) | Populated tile surface, status/navigation rows, no visible retiring overlay. |
| [Restored Settings](raw/metro-navigation-restored.png) · [inspection](raw/metro-navigation-restored.json) | System/applications headings and the full-width settings list are restored without a visible leftover projection. |
| [Lock after cancellation](raw/metro-navigation-lock-cancelled.png) · [inspection](raw/metro-navigation-lock-cancelled.json) | Lock artwork, clock/date and notification counts cover the phone; no Settings content or retiring page is visible. |
| [Separately settled navigation](raw/metro-turnstile-navigation-settled.png) · [inspection](raw/metro-turnstile-navigation-settled.json) | Settings and shell navigation render at the same resting bounds. |

All four PNGs were visually inspected at their original 1280 × 936 resolution.
The three unlocked captures contain identical RGB bytes in the 400 × 54 shell
navigation row: 882 nonblack pixels, including 523 with every RGB channel at
least 245. This is direct raster evidence for those retained settled frames;
the JSON motion traces are not a PNG sequence. Their rendered bounds are the
compositor's transformed axis-aligned bounds, not a pixel-level clipping or
occlusion measurement.

Elapsed times are recorder timestamps that include input and scheduling. They
do not independently calibrate physical display latency or the configured
350 ms paging and 250/350 ms turnstile tracks. The touch scenarios establish
release/settle behavior; an adjacent destination preview while the finger is
still down is outside the implemented paired-snapshot path.

No original Windows Phone hardware, font/raster reference, or original-device
transition video was compared. These captures establish the stated browser-host
behavior and rendering observations, not full OS coverage or 100 percent
visual parity.
