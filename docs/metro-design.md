# Metro design and interaction evidence

Research baseline: **2026-11-24**, with shell, Pivot and progress follow-up on **2026-11-27**. Target: **Windows Phone 8.1 Update (2014)**, as defined in [the reference specification](reference.md). This document records measured source constants and implementation requirements. It does not certify complete parity. Original-image setup and capture instructions are in [original-emulator.md](original-emulator.md).

## Keep the historical layers distinct

| Evidence | Applicable scope | Limit |
| --- | --- | --- |
| Windows Phone 8/8.1 Silverlight documentation | Theme resources, ApplicationBar, Pivot, Panorama, navigation | Does not specify every first-party application's layout. |
| Microsoft Windows Phone Toolkit, August 2013 | Public control templates, tilt, picker and navigation storyboards | An authentic WP7/WP8 SDK reference; not the private WP8.1 shell implementation. |
| BUILD2011 design-team presentation | Metro principles and the intended meaning of navigation/motion | Predates WP8.1, Action Center and live folders. |
| Original WP8.1 Update device/emulator captures | Final authority for the targeted shell and built-in app states | Must record exact build, resolution, scale, locale and theme. Not available from this session. |

The Toolkit source examined is pinned to commit [`4e4fd5d36c769c004f7ccc7daf78a5ffb8ef18df`](https://github.com/microsoftarchive/WindowsPhoneToolkit/tree/4e4fd5d36c769c004f7ccc7daf78a5ffb8ef18df). Its README identifies the August 2013 release and MS-PL licensing. Use the source to establish behavior and geometry; any copied source must retain its applicable license notices. New Rust implementation must not imply that the original operating system or its proprietary assets are bundled.

## Coordinate system and typography

The following values come from Microsoft's [theme resource table](https://learn.microsoft.com/en-us/previous-versions/windows/apps/ff769552(v=vs.105)). The right column is our arithmetic conversion for a **400-unit-wide** presentation of the original **480-unit-wide** design: `value × 400 / 480`. It is not a new Microsoft metric. Keep fractional geometry through layout; snap only where the renderer requires it. Do not scale animation time or rotation angles.

| Resource | Original reference | At 400 units |
| --- | ---: | ---: |
| Horizontal/vertical margin and ordinary touch overhang | 12 | 10 |
| Large touch overhang | 12 × 20 | 10 × 16.667 |
| Border/stroke thickness | 3 | 2.5 |
| Font Small | 18.667 | 15.556 |
| Font Normal | 20 | 16.667 |
| Font Medium | 22.667 | 18.889 |
| Font MediumLarge | 25.333 | 21.111 |
| Font Large | 32 | 26.667 |
| Font ExtraLarge | 42.667 | 35.556 |
| Font ExtraExtraLarge | 72 | 60 |
| Font Huge | 186.667 | 155.556 |

Segoe WP regular, Light, Semilight and Semibold are distinct named resources. Title1 uses Semilight/72; Title2 uses Semilight/32; Title3 uses Semilight/22.667; the huge style uses Light. A substitute font or synthetic weight changes advances, clipping and baselines, so matching numeric font sizes alone does not prove visual parity. Resolve font distribution separately from the geometry reference.

## Application bar

[Microsoft's app-bar guidance](https://learn.microsoft.com/en-us/previous-versions/windows/apps/ff431813(v=vs.105)) specifies a bottom command row whose ellipsis reveals labels and optional text commands. The menu is flat, with lowercased labels; five menu items and 14–20 characters per item are recommended limits. The bar expands/collapses with animation. In landscape it occupies the side next to the phone's steering buttons. Opaque bars reserve layout space; translucent bars overlay content. Mini mode returns after collapse and is recommended for Panorama. Landscape uses Default regardless of the requested mode.

| Geometry | Original reference | At 400 units | Source |
| --- | ---: | ---: | --- |
| Default extent | 72 | 60 | [DefaultSize](https://learn.microsoft.com/en-us/previous-versions/windows/apps/hh312738(v=vs.105)) |
| Mini extent | 30 | 25 | [MiniSize](https://learn.microsoft.com/en-us/previous-versions/windows/apps/hh312716(v=vs.105)) |
| Icon image box | 48 × 48 | 40 × 40 | [Icon rules](https://learn.microsoft.com/en-us/previous-versions/windows/apps/ff431806(v=vs.105)) |
| Glyph inside image box | 26 × 26 | 21.667 × 21.667 | Same icon rules |

The [ApplicationBar API](https://learn.microsoft.com/en-us/previous-versions/windows/apps/ff402616(v=vs.105)) allows one to four icon buttons. Icons are monochrome, theme recolored, and surrounded by a circle drawn by the control. The circle must not be duplicated inside the glyph. The hardware/system Back action supplies backward navigation; a second app-bar Back button is not the original convention. Expanded labels, command disabling, pointer cancellation and Back-to-collapse need behavioral verification, not just a static screenshot.

The icon rules also specify a **single line for English captions**, with truncation when necessary. Our labels now retain one line and use character ellipsis within their existing 76-unit slot. Command and app-bar pressed states use immediate contrasting fill/ink without shrinking the input rectangle. This palette mapping is an explicit adaptation: the original OS button-state brush ARGB values and app-bar expansion easing have not been recovered.

## Pivot and Panorama

[Pivot guidance](https://learn.microsoft.com/en-us/previous-versions/windows/apps/hh202919(v=vs.105)) describes cyclical horizontal pan/flick navigation among related data views. It recommends four or fewer pages, brief one/two-word headers, and retaining an empty filter when later user actions can populate it. Header continuity should advertise adjacent content. Nested Pivot/Panorama controls and horizontal gesture conflicts are discouraged; maps, sliders and editors belong on appropriate detail pages. Vertical list scrolling remains a supported hosted-control interaction. A horizontal pointer sequence must not also activate its starting row.

[Panorama guidance](https://learn.microsoft.com/en-us/previous-versions/windows/apps/hh202912(v=vs.105)) describes a portrait canvas with independent background, title and content layers, wraparound navigation, at most five sections, and restoration of the previously visited section. The title moves slower than background art; sections track the finger. Mini app bars preserve content space. Vertical scrolling is appropriate within a narrow section; simultaneous horizontal section content scrolling introduces ambiguity. Do not replace the layered layout with a static title above unrelated tabs.

The [Panorama overview](https://learn.microsoft.com/en-us/previous-versions/windows/apps/ff941104(v=vs.105)) explicitly identifies a **24-pixel next-section preview**, equivalent to **20 units at 400 width**. A clipped neighboring header/content edge is meaningful navigation information.

Settings (`system` / `applications`) and Office (`recent` / `places`) now use the shared native cyclic Pivot. Each page owns a finite vertical ScrollViewer, so scrolling content leaves the headings fixed and switching panes retains their separate offsets while the hub remains mounted. Office search uses a separate page with its own native editor, consistent with the original guidance to keep edit controls outside a Pivot's horizontal gesture surface. Back returns to the selected hub pane; scroll restoration across reconstructed search/detail pages remains a separate requirement. Native headings, keyboard selection, wraparound touch, narrow dark/light layouts and reduced-motion fallback have dedicated rendered scenarios.

## Control geometry and input

The [WP8 Toolkit template](https://github.com/microsoftarchive/WindowsPhoneToolkit/blob/4e4fd5d36c769c004f7ccc7daf78a5ffb8ef18df/Microsoft.Phone.Controls.Toolkit.WP8/Themes/Generic.xaml) provides this square ToggleSwitch geometry. Its thumb moves horizontally with a 50 ms checked/unchecked visual transition. Disabled state opacity is 0.3. The generous transparent root is the touch target; its visible rectangle is smaller.

| Toggle part | Original reference | At 400 units |
| --- | ---: | ---: |
| Transparent root | 136 × 95 | 113.333 × 79.167 |
| Track | 89 × 34 | 74.167 × 28.333 |
| Inner background | 77 × 20 | 64.167 × 16.667 |
| Thumb including side border | 28 × 38 | 23.333 × 31.667 |
| Thumb/background travel | 69 | 57.5 |
| Outer border / inner inset | 3 / 4 | 2.5 / 3.333 |

The [ToggleSwitchButton implementation](https://github.com/microsoftarchive/WindowsPhoneToolkit/blob/4e4fd5d36c769c004f7ccc7daf78a5ffb8ef18df/Microsoft.Phone.Controls.Toolkit/Primitives/ToggleSwitchButton.cs) also supports dragging. Preserving tap toggling alone is an incomplete interaction match. CheckBox, RadioButton, slider and editor state matrices must separately cover enabled, pressed, checked/selected, disabled and focus states; a rounded host-theme template cannot establish Metro fidelity.

The default WP8 Silverlight CheckBox/RadioButton templates and exact TextBox state colors were not recovered here. Microsoft's [theme implementation documentation](https://learn.microsoft.com/en-us/previous-versions/windows/apps/ff402557(v=vs.105)) locates per-theme `ThemeResources.xaml` files under `C:\Program Files (x86)\Microsoft SDKs\Windows Phone\v8.0\Design`. Extract their values from a matching original SDK before treating approximate control templates as exact. The published resource table identifies edit/read-only TextBox and pressed/disabled check/radio brushes but does not provide their ARGB values.

### Recovered choice and editor templates

The Toolkit's [ListPickerPage.xaml](https://github.com/microsoftarchive/WindowsPhoneToolkit/blob/4e4fd5d36c769c004f7ccc7daf78a5ffb8ef18df/Microsoft.Phone.Controls.Toolkit.WP8/ListPicker/ListPickerPage.xaml) contains `AnyFontSizeCheckBoxStyle`. Its square is 32 × 32, its outline is 3, its check path occupies 23 × 21 with `Stretch=Fill`, and its indeterminate mark is 16 × 16. The content gap is 12 and the touch overhang is 12 × 20. Pressing replaces the fill/check resource roles; disabling replaces outline/check/content ink rather than fading the whole subtree. This is evidence for this **Toolkit ListPicker style**, not the unrecovered universal OS checkbox. The adapted check geometry retains [MS-PL attribution](../NOTICE.md); the radio geometry remains an application adaptation.

The Toolkit [PhoneTextBox template](https://github.com/microsoftarchive/WindowsPhoneToolkit/blob/4e4fd5d36c769c004f7ccc7daf78a5ffb8ef18df/Microsoft.Phone.Controls.Toolkit.WP8/Themes/Generic.xaml) specifies a normal-weight 25.333 font, 3-unit border/padding and 1 × 2 inner margin. Focus changes background and border resource roles immediately. The original disabled presentation swaps to a separate read-only TextBox with transparent chrome and disabled ink. Our adapter preserves one native TextPresenter/ScrollViewer, including composition, selection, undo and clipboard state, and applies those resource roles without swapping editors. Neutral color values, general read-only appearance, and the desktop keyboard-focus outline are documented adaptations. Forms retain their existing outer placement rather than receiving a second 12-unit margin.

Structural templates are installed separately from replaceable palette styles. Changing the accent or light/dark theme must preserve native button presenters, choice parts, slider tracks, picker wheels and active text composition. Palette changes must not synthesize a click, selection change or value commit.

### Date and time picker pages

The Toolkit [DatePickerPage.xaml](https://github.com/microsoftarchive/WindowsPhoneToolkit/blob/4e4fd5d36c769c004f7ccc7daf78a5ffb8ef18df/Microsoft.Phone.Controls.Toolkit.WP8/DateTimePickers/DatePickerPage.xaml), [TimePickerPage.xaml](https://github.com/microsoftarchive/WindowsPhoneToolkit/blob/4e4fd5d36c769c004f7ccc7daf78a5ffb8ef18df/Microsoft.Phone.Controls.Toolkit.WP8/DateTimePickers/TimePickerPage.xaml) and [shared page implementation](https://github.com/microsoftarchive/WindowsPhoneToolkit/blob/4e4fd5d36c769c004f7ccc7daf78a5ffb8ef18df/Microsoft.Phone.Controls.Toolkit/DateTimePickers/DateTimePickerPageBase.cs) establish a full phone page, chrome background, large looping selectors and check/cancel application-bar actions.

| Picker detail | Original 480-unit reference | Implementation boundary |
| --- | --- | --- |
| Columns/cell pitch | Three columns of 148; three columns total 444, centered | Scaled to available phone width; native presenters retain optional seconds support |
| Cell inset | 6 on each side, leaving a visible 136-unit square | Padding is inside the 148-unit pitch |
| Cell type | Primary 54 Semibold; secondary 20 normal | Portable phone font; original Segoe WP rasterization is unverified |
| Header | 22.667 Semibold; margin 24,16,24,24 | Safe status and hardware strips remain owned by the shell |
| Selection | Accent fill/white ink; unselected expanded outline 3 at opacity 0.8 | Native DateTimePickerPanel owns selection and scrolling |
| Open | X rotation −50° to 0°, 200 ms linear | Dedicated picker surface, input gated during the motion |
| Close | X rotation from current angle to +90°, 200 ms exponential-in | Exponent 2, the documented Silverlight default; lock/navigation teardown is immediate |
| Commit | Stage selection; check commits; cancel/Back abandons it | Native DatePicker/TimePicker value events remain authoritative |

The XAML leaves the close `ExponentialEase.Exponent` unset. Microsoft's [property documentation](https://learn.microsoft.com/en-us/previous-versions/windows/silverlight/dotnet-windows-silverlight/dd431601(v=vs.95)) establishes its default as **2**. This picker-specific storyboard differs from the generic swivel navigation storyboard below.

The original picker wraps month/day/hour/minute without carrying into an adjacent unit; AM/PM and year lists are finite, and month/year changes clamp the day. Culture controls field order and 24-hour display omits AM/PM. The native FerroUI picker is retained for calendar/range/increment validation, keyboard input and value staging. Its wheel inertia and snap curve are **not yet a verified match** for the original Toolkit `LoopingSelector` (330 ms expansion, 500 ms snap and distinct pan/friction behavior). Full-page composition and matching selected constants do not establish identical selector physics.

The adapter snaps selector pitch to the nearest **1/64 logical unit**, an adjustment of at most 1/128 unit. At fractional adapted widths, unrounded pitch could make the native panel's offset-to-index truncation return the preceding item (for example, a quotient just below 2), silently losing a keyboard step. Binary-exact pitch keeps each native step stable; column width uses the same pitch. This numerical adaptation is separately covered by a per-key month sequence and is not an original Microsoft geometry constant.

Hardware Back first collapses an expanded picker application bar, then cancels the picker, and only a subsequent Back may leave its editor. Home, route replacement, lock/privacy overlays and session teardown close picker surfaces synchronously without confirmation or focus restoration into a hidden editor. Normal dismissal may restore keyboard focus to the owning field.

[ListPicker source](https://github.com/microsoftarchive/WindowsPhoneToolkit/blob/4e4fd5d36c769c004f7ccc7daf78a5ffb8ef18df/Microsoft.Phone.Controls.Toolkit/ListPicker/ListPicker.cs) sets the inline threshold to five items when expansion is permitted and animates height/translation over 200 ms. Longer choices use a picker page. Date/time pickers, full-page list selection and picker cancellation need phone-specific composition rather than generic desktop popups.

[Microsoft's certification guidance](https://blogs.windows.com/windowsdeveloper/2014/01/10/top-windows-phone-app-certification-failures-and-how-to-avoid-them/) requires Back to dismiss dialogs/keyboards, navigate to a preceding page, and leave the app only at its root. Implement those outcomes through one ordered Back dispatcher. Do not let keyboard dismissal or menu cancellation also pop the navigation journal. Start preserves appropriate suspended app state. Browser and native host adapters must map their input into the same shell contracts.

## Motion constants and meaning

The following values are from the Toolkit's [navigation storyboards](https://github.com/microsoftarchive/WindowsPhoneToolkit/tree/4e4fd5d36c769c004f7ccc7daf78a5ffb8ef18df/Microsoft.Phone.Controls.Toolkit/Transitions/Storyboards), not timings measured from an original WP8.1 Update device.

| Transition | Reference trajectory | Duration | Easing |
| --- | --- | ---: | --- |
| Turnstile forward in | Y: −80° → 0° | 350 ms | Exponential out, exponent 6 |
| Turnstile forward out | Y: 0° → +50° | 250 ms | Exponential in, exponent 6 |
| Turnstile backward in | Y: +50° → 0° | 350 ms | Exponential out, exponent 6 |
| Turnstile backward out | Y: 0° → −80° | 250 ms | Exponential in, exponent 6 |
| Slide left in | X: +200 → 0; opacity 0 → 1 | 500 ms | Exponential out, exponent 6 |
| Slide right out | X: 0 → +200; opacity 1 → 0 | 300 ms | Exponential out, exponent 6 |
| Swivel forward in | X rotation: −45° → 0° | 350 ms | Exponential out, exponent 6 |
| Swivel forward out | X rotation: 0° → +90° | 250 ms | Exponential in, exponent 15 |

Turnstile entry reaches opacity 1 at 10 ms. Exit stays opaque until 240 ms and becomes invisible at 250 ms. Plain turnstile uses `PlaneProjection.CenterOfRotationX = 0` in [Transitions.cs](https://github.com/microsoftarchive/WindowsPhoneToolkit/blob/4e4fd5d36c769c004f7ccc7daf78a5ffb8ef18df/Microsoft.Phone.Controls.Toolkit/Transitions/Transitions.cs), with the default Y origin 0.5. [TurnstileFeatherEffect](https://github.com/microsoftarchive/WindowsPhoneToolkit/blob/4e4fd5d36c769c004f7ccc7daf78a5ffb8ef18df/Microsoft.Phone.Controls.Toolkit/Effects/TurnstileFeatherEffect.cs) instead uses a horizontal rotation origin of −0.2; stagger increments are 40 ms for forward-in/backward-out and 50 ms for forward-out/backward-in. Preserve forward/backward distinctions rather than playing one generic fade for every route.

Our page choreography now preserves one rendered image of the settled outgoing page for the 250 ms exit, then starts the 350 ms incoming turnstile. Route mutation and old app disposal remain synchronous; the destination is the only live app tree. During the exit its input is disabled to prevent activation through the retiring image. The image is noninteractive and bounded to 2,097,152 pixels (at most 8 MiB in RGBA), clears at completion, and is discarded on supersession, resize, lock/privacy coverage, unload or reduced motion/Battery Saver. If capture is unavailable, the incoming animation proceeds without an outgoing image. This is a bounded cross-host rendering strategy, not the original operating system's page-cache implementation.

Start/Apps paging uses that same bounded image alongside the live destination. A committed horizontal swipe carries its released displacement into two translations sharing one clock; the page edges stay exactly one viewport apart. It no longer starts a competing snap-back before attempting capture. Rejected swipes still settle, input remains blocked during committed paging, and completion or cancellation clears both transforms and the image. The existing 350 ms exponential-out paging policy is retained; it is not a timing newly measured from WP8.1. Held-drag neighboring-page preview and original-device paging physics remain open.

### Progress control

The Toolkit's [WP7 `PerformanceProgressBar` template](https://github.com/microsoftarchive/WindowsPhoneToolkit/blob/4e4fd5d36c769c004f7ccc7daf78a5ffb8ef18df/Microsoft.Phone.Controls.Toolkit.WP7/Themes/Generic.xaml) defines five square dots, each **4 × 4 reference units**, in a **4.4-second cycle** with **200 ms staggering**. Each dot moves from 0 to 33% in 500 ms with exponential-out exponent 1, continues linearly to 66% at 2 seconds, then reaches 100% at 2.5 seconds with exponential-in exponent 1. The dot is hidden for the remainder of its cycle. These are WP7 Toolkit constants, not recovered private WP8.1 OS control constants.

The `progress` helper applies that activity strip to a native FerroUI ProgressBar. Known work keeps native minimum, maximum, value coercion, percentage and automation semantics; unknown work is explicitly indeterminate. The native pixel grid rounds the adapted 3.333-unit dot extent at the actual display density. Templates survive palette changes. Compositor-owned animations stop when hidden, disabled, detached or motion-suppressed, and resize replaces the bounded animation set. Reduced motion uses a stationary five-dot indicator without manufacturing a completion percentage. The determinate strip, vertical form and motion suppression are adaptations; the Toolkit wrapper's delayed show/fade behavior is not implemented.

### Position-sensitive tilt

Simplifying the [Toolkit TiltEffect calculation](https://github.com/microsoftarchive/WindowsPhoneToolkit/blob/4e4fd5d36c769c004f7ccc7daf78a5ffb8ef18df/Microsoft.Phone.Controls.Toolkit/Effects/TiltEffect.cs), for a normalized pointer `u = clamp(x/width, 0, 1)`, `v = clamp(y/height, 0, 1)`:

- X-axis rotation: `(v − 0.5) × 0.3` radians.
- Y-axis rotation: `−(u − 0.5) × 0.3` radians.
- Depth: `−25 × (1 − abs(u − 0.5) − abs(v − 0.5))` reference pixels.

The center depresses 25 pixels with no rotation; an edge midpoint rotates about 8.59° and depresses 12.5; a corner has about 8.59° on each axis and no depression. At 400 width the maximum depression is 20.833. The default return waits 200 ms, then interpolates back over 100 ms. `UseLogarithmicEase` is an uninitialized static Boolean, so the default is **linear**. Its optional easing overrides `EaseInCore`; the base [EasingMode default is EaseOut](https://learn.microsoft.com/en-us/previous-versions/windows/silverlight/dotnet-windows-silverlight/dd431616(v=vs.95)). Do not silently substitute the optional curve.

Microsoft's [Silverlight projection explanation](https://learn.microsoft.com/en-us/archive/msdn-magazine/2010/april/ui-frontiers-projection-transforms-sans-math) specifies a camera 1000 units from the XY plane, centered on the element regardless of the rotation origin, X/Y/Z rotation order and projection independent from layout. Uniform scaling gives a distance of 833.333 at 400 width. A left-edge turnstile pivot must therefore be independent from the centered perspective camera. Renderer composition and global-center compensation still require comparison; using the correct angle alone is insufficient.

The adaptation must retain untransformed layout and stable hit testing, reset transforms after cancellation/detachment, and respect reduced motion. Reduced-motion behavior is an explicit modern accessibility adaptation, not evidence of an original system preference.

## Lists, tiles and first-party app evidence

Microsoft's [WP8 LongListSelector article](https://blogs.windows.com/windowsdeveloper/2013/05/23/windows-phone-8-xaml-longlistselector/) establishes sticky group headers with push-off by the next group, virtualized grouped grids, item-realization hooks, and locale-aware grouping. Validate those states on Apps and People using long lists and non-ASCII names; implementing only an English A–Z overlay is not full locale parity.

[FlipTileData documentation](https://learn.microsoft.com/en-us/previous-versions/windows/apps/jj208605(v=vs.105)) distinguishes front/back content and small/medium/wide fields. A count of zero hides the count; 1–99 is its documented count range, and titles apply to medium/wide tiles. Tile data contracts do not establish precise native shell flip timing. The WP8.1 Update live-folder, lock, Action Center, Office and built-in app requirements remain in [reference.md](reference.md); do not infer their full behavior from the earlier Toolkit.

## Video inspection record

[Bryan Agnetta, BUILD2011: Windows Phone user experience design](https://learn.microsoft.com/en-us/shows/build-build2011/app-832t) was inspected through Microsoft's retrieved caption track at **10:04–10:43**, **23:00–24:30**, **38:00–40:10**, and **45:13–47:36**; the actual video frame at **46:10** was also extracted and visually examined. These are selected excerpts, not full playback. The automatic transcript contains transcription errors; exact numeric requirements above come from API/source evidence.

The inspected material connects grid/typography, responsiveness and action feedback. Pivot supports focused recurring tasks; Panorama supports exploration. Turnstile marks movement between larger spaces; continuum carries context into detail; swivel denotes transient UI; slide accompanies contextual selection. It discourages persistent decorative movement that distracts from content.

[Corrina Black, TechEd Europe 2012: Application UI Design Principles](https://learn.microsoft.com/en-us/shows/teched-2012/wph204) was discovered and its published description read; its video frames/transcript were **not** inspected. Microsoft's [2010 design-resource announcement](https://blogs.windows.com/windowsdeveloper/2010/07/27/windows-phone-7-design-resources-ui-guide-and-design-templates/) was read. The old guide download redirect did not retrieve the original PDF, so no new claim here depends on having inspected its full contents.

## Validation matrix

These are acceptance checks, not a list of already passed comparisons. Each comparison needs an original capture and a recreation capture at the same scenario state.

| Area | Required cases | Evidence to retain |
| --- | --- | --- |
| Typography/layout | Dark/light; short/long text; portrait/landscape where supported; 480 reference and 400 adaptation | Font identity, baselines, measured bounds, clipping and image diff |
| Application bar | 1–4 icons; Mini; expansion; disabled action; Back/outside dismissal | Collapsed/expanded captures; reserved area; event trace |
| Pivot/Panorama | Both directions; wrap; partial/cancelled pan; vertical scroll; return to prior pane | Frame sequence, selected item, offsets and input ownership |
| Controls | Tap/drag/cancel; checked/unchecked; disabled; keyboard focus; selection commit/cancel | Value changes, pointer capture and screenshots |
| Tilt/transitions | Center/edges/corners; press/release; cancellation; interrupted navigation; reduced motion | Timed transform samples and original-frame comparison |
| Lists | Sticky push-off; jump; empty groups; search; long and localized data | Scroll positions, headers and bounded realization counts |
| Shell/apps | Start/apps/back; lock; notifications; folder/tile edit; mail/message/Office workflows | Exact OS build, app version, scenario and resulting data |
| Host integration | Browser, desktop, Android, iOS capability/permission outcomes | Real host acknowledgements and explicit unsupported paths |

Documented constants, working local tests and recreation screenshots establish useful progress. They cannot establish **100% visual, interaction or OS feature parity** without original WP8.1 Update captures, matched typography/rasterization, full state coverage and device integration checks. Live services and privileged hardware operations require their own implementations and evidence.
