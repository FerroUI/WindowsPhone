# Candidate d515348: acceptance and evidence

**All seven candidate gates passed.** The browser, Linux, macOS, Windows,
Android host, Android mounted-UI and iOS checks completed successfully.
PR #6 was merged as `d8557a8a576c34c82dff761ba99ebe82f4f3059c`; its complete
tree is identical to the verified candidate. This dossier records candidate
acceptance. Publication from `main` is recorded separately in the parent dossier.

## Source and CI identity

The reviewed source is `d515348c699c22110def2dded8f7b8aea4b9cd03`. CI checked out
synthetic merge `75d8b175260015c8eb61c82ab87d525bbba15221`; both have tree
`ad870fb6a20b630070b6b9e0287dd0eec665f22c`. [Identity verification](identity.json)
records the merge parents, exact tree equality and pinned FerroUI revision.
[Compiled artifact provenance](compiled-artifact.json) records the browser
artifact, build marker and individual compiled-file hashes. A synthetic build
marker is therefore explained by the verified checkout, rather than treated as
an unexplained source mismatch.

| CI run | Recorded scope and status |
| --- | --- |
| [Build and publish — 45959413873](https://github.com/FerroUI/WindowsPhone/actions/runs/45959413873) | Browser verification and all three complete desktop builds passed. |
| [Android hosts — 45959413886](https://github.com/FerroUI/WindowsPhone/actions/runs/45959413886) | Passed native Android host build verification. |
| [Android mounted UI — 45959413903](https://github.com/FerroUI/WindowsPhone/actions/runs/45959413903) | Passed emulator installation, native input and device-service scenarios. |
| [iOS host — 45959414018](https://github.com/FerroUI/WindowsPhone/actions/runs/45959414018) | Passed host build and Simulator device/navigation scenarios. |

## Counts and acceptance scope

The [browser-job summary](browser-summary.json) records **469 Rust tests passed,
0 failed and 1 ignored**, plus **80 JavaScript tests passed**. Its compiled
browser suite passed **112 checks**, produced **198 captures**, and recorded no
browser errors; [complete results](browser-results.json) retain each check.
These are counts from that job. Local preflight repetitions, other platform
jobs and the review subsets overlap; they are not added into a larger claimed
test total. Suite elapsed time is verification runtime, not app-performance
measurement.

Build/link success establishes compilation and the specified native test gates.
It does not establish that every new screen or interaction was rendered on
every platform. The following reviews identify the actual rendered and input
coverage.

| Evidence review | What was inspected |
| --- | --- |
| [Shell motion](motion/README.md) | Eight original traces, paired Start/Apps release motion, forward/back turnstiles, interruption, lock cancellation and motion preferences. |
| [Progress](progress/README.md) | Real pending host responses, five moving/static dots, Weather footer alignment, app switching, response cleanup and refresh. |
| [Hubs and notifications](hubs-notifications/README.md) | Settings/Office native headings and pane behavior, narrow themes, focused search, chronological groups, dismissal and privacy fixtures. |
| [Android](android/README.md) | Actual notification-listener admission/rendering/revocation, preserved local groups, Android Back/HOME and launcher-role behavior. |
| [iOS](ios/README.md) | Simulator device checks, URL activations, native taps, original captures and decoded navigation-row pixels. |

The Android notification scenario does **not** exercise horizontal group
gestures or two-finger clearing; browser touch fixtures and native ownership
tests supply separate evidence. Android also does not exercise the new paired
Start/Apps paging here. The iOS Settings image is a device-services **leaf**, not
the new root hub. Its Wallet image captures an **entrance**, not settled layout.
Those distinctions are retained in the platform reviews.

## Timing, fidelity and preservation limits

In each browser paging direction, the first paired native observation has new
translations while compositor readback still reports the preceding bounds.
Subsequent paired observations agree. This establishes continued paired motion,
not zero-frame release latency or pixel adjacency at every instant. The mounted
headless regression reaches the production capture path and verifies lifetime,
input ownership and dimensions; its default renderer supplies size-only bitmap
contents, so it is not pixel evidence.

Progress adapts the WP7 Toolkit's inner dot storyboard; the outer 250 ms entrance
and 750 ms fade are absent. Native opacity inspection does not independently
measure compositor-animated opacity. No original Windows Phone device, font,
raster or transition-video comparison establishes full OS or 100% visual parity.

The [frozen preflight history](../preflight/README.md) preserves earlier failures,
fixture corrections and the document-restoration capture regression before and
after its fix. It is investigation history, not substituted final acceptance.
Per-review provenance identifies artifact archives and unchanged PNG/JSON/text
members with hashes. Raw artifact bytes are distinguished from decoded logs:
the retained `.log.gz` files contain compressed log text and are not claimed to
be byte-identical copies of GitHub's original log ZIPs. Authored reviews and
summaries are interpretation; original observations remain available alongside
them.
