# Published main shell paging: d8557a8

This review covers the **two fresh Start/All Apps traces, 126 observations in
total**, from the [main browser job](https://github.com/FerroUI/WindowsPhone/actions/runs/45964709343/job/140390520527)
for `d8557a8a576c34c82dff761ba99ebe82f4f3059c`. It is an inspection of the
preserved run artifacts; no application execution or Cargo tests were repeated.

[provenance.json](provenance.json) binds the traces to artifact `15183619231`
and independently verified ZIP SHA-256
`6143525949e98c0136f18149ae6b3a455013dc0e71f0c90e99f5ab1d1c0cbb9a`.
Both original JSONs match their ZIP members byte for byte and total **202,475
bytes**. Main and accepted candidate `d515348c699c22110def2dded8f7b8aea4b9cd03`
have the same locally verified tree,
`ad870fb6a20b630070b6b9e0287dd0eec665f22c`. The parent directory retains the
full run and delivery metadata. The UI ZIP has no embedded commit marker.

## Fresh measurements

[review.json](review.json) records the calculations and every initial native /
compositor mismatch. The resting page is `(440, 87, 400, 740)` in root
coordinates; every captured source reports logical dimensions of 400 × 740.

| Original trace | All observations | Paired native observations | Matching compositor observations | First clean observation, ms |
| --- | ---: | ---: | ---: | ---: |
| [Start → All Apps](raw/metro-shell-paging-apps-frames.json) | 62 | 17 | 15 | 653.6 |
| [All Apps → Start](raw/metro-shell-paging-start-frames.json) | 64 | 17 | 16 | 632.4 |

All **34 paired native observations** preserve a directed separation of exactly
400 in the recorded translation values, with incoming displacement moving
monotonically toward zero. The first incoming/outgoing pair is `(+232, −168)`
for Start → All Apps and `(−232, +168)` for the reverse. The last sampled pairs
are approximately `(+0.156, −399.844)` and `(−0.293, +399.707)` respectively.

Both surfaces remain fully opaque, disabled and excluded from hit testing in
every paired observation. The outgoing surface is never a native hit target.
The 31 matching compositor observations show both surfaces moving together,
with a maximum adjacency residual of `5.7 × 10⁻¹⁴` logical pixels. This is a
floating-point coordinate residual, not physical display precision.

Both terminal observations restore the destination to its resting bounds with
opacity 1, enabled input and hit testing. The incoming/outgoing transforms are
removed, the outgoing surface has opacity zero, and its image source is absent.

## Readback qualification and scope

Start → All Apps has **two initial paired inspections**, at 301.0 and 316.1 ms,
whose native translations are already set while both compositor bounds still
report `x = 440`. The reverse has **one**, at 289.4 ms. All subsequent paired
readbacks match the native translations. These original samples remain in the
archive; this review does not claim zero-frame release latency or pixel
adjacency at every instant.

Elapsed values are recorder timestamps including input and scheduling, not
physical latency measurements. Rendered bounds describe compositor geometry;
these JSONs are not per-frame raster images or clipping/occlusion comparisons.
The scope is the browser release/settle phase, without an original Windows Phone
hardware, font, raster or transition-video comparison.

The separate [candidate full motion review](../../candidate-d515348/motion/README.md)
covers turnstiles, interruption, locking and motion preferences. Its observations
remain identified as candidate evidence and are not relabelled as this fresh
main-run review. Neither record establishes full OS or 100 percent visual parity.
