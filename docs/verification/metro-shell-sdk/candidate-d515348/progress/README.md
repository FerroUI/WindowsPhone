# Progress capture review

**Pass for the reviewed scope.** These unchanged CI captures show the Weather
footer aligned with its page content and five-dot activity in Weather and
Podcasts. The Podcasts capture no longer has the subscribe hover tooltip.

## Captures and native measurements

| Capture | Activity bounds `(x, y, width, height)` | Loading label bounds | Observation |
| --- | --- | --- | --- |
| [Weather PNG](metro-progress-weather-moving.png) / [native JSON](metro-progress-weather-moving.json) | `(460, 383, 360, 4)` | `(460, 403, 360, 22)` | Five moving accent squares; label and current-location action align with city content. |
| [Podcasts PNG](metro-progress-podcast-reduced-motion.png) / [native JSON](metro-progress-podcast-reduced-motion.json) | `(460, 212, 360, 4)` | `(460, 232, 360, 22)` | Five static centered accent squares; application bar is unobscured. |

Both phone surfaces have `x=440`, `width=400`, inside a `1280 × 936` browser
viewport. Both activity strips therefore have 20 logical pixels of inset on
each side. Weather's current-location text starts at `(460, 449)`. The five
named native dots in each JSON measure `4 × 4` after layout pixel rounding;
their native hit-test results are false. Both bars have descriptive automation
names: `Loading weather` and `Loading podcast`.

The candidate source sets both active loading labels to font size 18 at text
scale 1; their observed native line boxes are 22 pixels high. The JSON does not
expose font size directly. This establishes consistency within the current
application, not a measured match to an original device font.

## Pending-operation and lifecycle evidence

The [browser results](../browser-results.json) record both progress checks as
passed: Weather in 6,887 ms and Podcasts in 2,218 ms. The complete suite passed
112 checks and produced 198 captures.

`scripts/ui-metro-progress.mjs` holds actual host HTTP responses while normal
application commands own loading state. The Weather check samples rendered dot
positions across 4.6 seconds, requires all five retained dots to move across the
strip, releases the forecast response, and verifies committed data and removal
of the progress visual tree. The Podcasts check verifies static positions over
400 ms with reduced motion, leaves and reopens the app while its response is
pending, releases the response, and verifies cleanup and refresh without a
duplicate subscription. The captures are evidence from these checks; a still
image alone does not establish their temporal behavior.

## Provenance and limits

[Provenance](provenance.json) records source commit
`d515348c699c22110def2dded8f7b8aea4b9cd03`, the synthetic merge and tree,
workflow/job/artifact identifiers, archive SHA-256, individual file hashes,
and the passing check records. All four capture files were compared byte for
byte with their downloaded archive entries before preservation. They have not
been cropped, repainted, or regenerated.

The dot timeline adapts the inner `PerformanceProgressBar` storyboard from the
[WP7 Toolkit source](https://github.com/microsoftarchive/WindowsPhoneToolkit/blob/4e4fd5d36c769c004f7ccc7daf78a5ffb8ef18df/Microsoft.Phone.Controls.Toolkit.WP7/Themes/Generic.xaml).
Its outer 250 ms entrance and 750 ms fade-out are not implemented. Browser
geometry uses compositor readback, but `inspectView.opacity` reports native
visual opacity rather than the compositor's animated opacity; these captures
do not independently verify opacity timing. The centered reduced-motion dots
and current application typography are adaptations. No original-device raster
comparison or complete Windows Phone 8.1 OS parity is claimed.
