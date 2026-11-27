# Published Metro shell and SDK: `d8557a8`

[Open the app](https://ferroui.com/WindowsPhone/) · [Merged PR #6](https://github.com/FerroUI/WindowsPhone/pull/6) · [Main build and deployment](https://github.com/FerroUI/WindowsPhone/actions/runs/45964709343) · [Implementation and fidelity boundaries](../README.md)

## Source and execution identity

The published runtime source is `d8557a8a576c34c82dff761ba99ebe82f4f3059c`. Its complete tree, `ad870fb6a20b630070b6b9e0287dd0eec665f22c`, is identical to accepted candidate `d515348c699c22110def2dded8f7b8aea4b9cd03` and candidate CI checkout `75d8b175260015c8eb61c82ab87d525bbba15221`. FerroUI remains pinned to `ec86b9c0204b8dee243eb300135cb0834f0a84b3`. The [identity record](identity.json) preserves these bindings.

The main workflow built a fresh browser package, ran the complete rendered interaction suite and deployed Pages successfully. Its native job and the separate mobile workflows were skipped by the existing workflow policy. Complete Windows/macOS/Linux builds, Android ARM App/HOME packaging, Android mounted UI and iOS Simulator verification belong to the [accepted identical candidate tree](../candidate-d515348/README.md); they are not represented as fresh main executions. Documentation committed afterward does not change this published runtime revision.

## Fresh main verification

| Gate | Observed result | Evidence |
| --- | --- | --- |
| Rust | 469 passed, zero failed, one ignored | [Summary](browser-summary.json), [complete decoded job log](browser.log.gz) |
| JavaScript | 80 passed, zero failed | [Summary](browser-summary.json), [complete decoded job log](browser.log.gz) |
| Rendered browser | 112 checks passed, 198 captures, no recorded browser errors; 406,000 ms for the whole suite | [Unmodified results](browser-results.json), [UI artifact identity](ui-artifact.json) |
| Pages | Build job `140390520527` and deployment job `140406547599` passed | [Run and job metadata](ci.json), [deployment log](deployment.log.gz) |
| Public delivery | All 40 HTTP requests passed byte-identity and required content-type checks | [Public observations](public-delivery.json) |

Counts apply to the stated main browser job. Candidate, local and platform repetitions are not added into a larger unique-test count. The rendered-suite duration measures the verification run, not application performance. Selected [main captures and inspections](captures/README.md) and [both paired paging traces](motion/README.md) are preserved separately from the fuller candidate reviews.

The complete UI artifact is `15183619231`, ZIP SHA-256 `6143525949e98c0136f18149ae6b3a455013dc0e71f0c90e99f5ab1d1c0cbb9a`. The original `results.json` and selected members are copied byte for byte. The [artifact metadata](browser-artifacts.json) also identifies the compiled and Pages packages.

## Compiled and publicly served bytes

Compiled artifact `15183782822` has ZIP SHA-256 `0e35b60e5144e9eb4df3ec2ecd8aba6b78a3e6809c889b7e5c9a51dc25de1dc2`. Its [43-file manifest](compiled-artifact.json) and [candidate comparison](candidate-comparison.json) establish **41 byte-identical files** relative to the accepted candidate. Only `index.html` and `app-sw.js` differ, and only by the checkout-derived stamp `75d8b17` → `d8557a8`.

The **23,732,116-byte** WebAssembly module has SHA-256 `1877871a56eb0bb6ae6663e705ce5639499df8c9da0fac68b665029a2b622341` in the accepted candidate, fresh main package and observed public response.

Pages reported successful deployment at **2026-11-27T08:56:58.9471701Z**. Public observations began at **2026-11-27T08:57:50.285668+00:00**. The [verification program](../verify-publication.py) independently requested the public root document, all 38 generated precache files and the service worker: **40 requests, all passed**. It checked status, byte length and SHA-256 against the compiled package and required appropriate HTML, JavaScript, CSS and WebAssembly content types. The root document, explicit index and worker all identify `d8557a8`.

Original response bodies are retained for the [root document](public-delivery-responses/root-document.html), [explicit index](public-delivery-responses/index.html) and [service worker](public-delivery-responses/app-sw.js). These are direct public HTTP identity observations. The rendered interaction evidence comes from the fresh exact-source CI browser run; no additional live-site browser interaction run is claimed.

For an existing installation, **Settings → phone update** checks the new build. Closing all old app tabs and installed app windows lets the waiting service worker activate when the app is reopened.

## Evidence and limits

The [checksum manifest](manifest.json) covers the retained release evidence. GitHub job logs are connector-decoded UTF-8 with normalized newlines; their compressed and decoded hashes identify the archived text, not GitHub's original log ZIP. PNG, inspection, trace and results members retain their original artifact bytes.

The [candidate record](../candidate-d515348/README.md) contains the seven passing platform gates and broader motion, progress, hub, notification and native-host reviews. The [preflight record](../preflight/README.md) preserves two earlier failed browser runs, their fixes, the real document-restoration capture regression and its failing-to-passing test. No assertions or existing runtime deadlines were relaxed to obtain acceptance.

This release improves the recreation's behavior and visual consistency. It does not establish original Windows Phone pixel or motion equality, physical-device qualification, complete Panorama/continuum behavior, held-drag neighboring-page previews, original selector physics, or exact original font/raster values. The progress control adapts the public WP7 Toolkit inner dot storyboard; its outer entry delay and exit fade are still absent. Detailed boundaries remain in the [implementation record](../README.md).
