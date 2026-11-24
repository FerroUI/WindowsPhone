# Original Windows Phone emulator and capture procedure

Research/probe date: **2026-11-24**. The recreation targets **Windows Phone 8.1 Update (2014)**. An original emulator is a reference source for that operating system; running the FerroUI browser application is not an original emulator run.

## Actual result in this environment

The Linux environment was inspected without installing software or changing virtualization settings:

| Check | Observation |
| --- | --- |
| Operating system / architecture | Linux x86_64 |
| Kernel | 6.18.57 |
| `/dev/kvm` | Absent |
| CPU `vmx` / `svm` flags exposed in `/proc/cpuinfo` | Neither exposed |
| `qemu-system-x86_64` | Not found on PATH |
| Wine | Not found on PATH |
| PowerShell | Not found on PATH |
| Original Microsoft XDE launched | **No** |
| Original WP8.1 Update image booted or captured | **No** |

These are observations of this execution environment, not a claim that its underlying physical machine lacks virtualization. Microsoft XDE for WP8/8.1 requires the corresponding Windows/Hyper-V environment, which is unavailable here. A compatibility layer or a browser recreation does not supply those prerequisites. No original emulator screenshot has been produced in this session.

## Repeatable read-only probe

From the repository root:

```bash
python scripts/probe-original-emulator.py
python scripts/probe-original-emulator.py --json
```

On Windows, the script uses bounded, non-interactive PowerShell/CIM queries for OS architecture, hypervisor presence, processor SLAT/firmware indicators, Hyper-V optional-feature states, and relevant service states. It searches conventional Program Files locations for SDK roots, XDE executables and phone images. Custom installations may not be found. Permission failures remain **unknown** instead of being converted to false.

The script performs no network access, installation, elevation, registry writes, feature changes, service changes, VM creation or VM launch. It does not use `Win32_Product`, whose inventory behavior can trigger installer repair. File presence is reported separately from successful boot; a zero exit code means the inventory completed, not that the emulator is ready. The Windows branch must be exercised on a real Windows host before its output is treated as host qualification.

## Legitimate original packages and historical requirements

| Microsoft source | Verified page information | Availability limit |
| --- | --- | --- |
| [Windows SDK archive](https://learn.microsoft.com/en-us/windows/apps/windows-sdk/downloads-archive) | Lists WP8.1 tools, WP8.1 emulators, WP8.1 Update emulators, WP8 and WP7.x packages | Listing was retrieved. All linked installer payloads were not downloaded/installed. |
| [WP8 SDK download center](https://www.microsoft.com/en-us/download/details.aspx?id=35471) | Windows 8 x64, 4 GB RAM, 6.5 GB free space; emulator requires Pro or higher and SLAT | WP8 prerequisites, not a statement that this is the target WP8.1 Update image. |
| [Visual Studio 2013 system requirements](https://learn.microsoft.com/en-us/visualstudio/releases/2013/vs2013-sysrequirements-vs) | Phone emulators require Windows 8.1 x64 Professional or higher, client Hyper-V and SLAT | Historical supported setup; current Windows versions must be qualified separately. |
| [Introducing WP8 SDK](https://devblogs.microsoft.com/visualstudio/introducing-windows-phone-sdk-8-0/) | WP8 WVGA/WXGA/720p images, plus older WP7.1 images that do not require SLAT | WP7.1 is an earlier behavioral baseline and cannot verify WP8.1 features. |

The archive describes WP8.1 emulator packages as additions to Visual Studio 2013 Update 2 or later, with a separate Update 1 package. It also points to Visual Studio Community 2015 for phone tools. Select the package whose **guest OS version** matches the intended reference. A product name or installed SDK folder alone does not prove that the guest runs WP8.1 Update.

Use Microsoft distribution links or an existing properly licensed installation. The SDK archive explicitly warns that these old installers are unserviced; use a dedicated reference machine or VM with controlled networking. This is practical guidance for running an obsolete OS, not a requirement to alter the development workstation. Do not redistribute emulator OS images or licensed fonts into this repository.

## Windows setup and first boot

The following is a manual procedure for a compatible Windows host. None of these setup actions were executed here.

1. **Choose the host.** Use a licensed x64 Windows installation and hardware meeting the selected SDK's documented requirements. Confirm CPU virtualization/SLAT and firmware virtualization availability. If using a Windows VM, explicitly verify that its hypervisor exposes the nested virtualization needed by Hyper-V; do not assume that an ARM host running an x64 application provides it.
2. **Run the probe.** Save the JSON report alongside the reference capture record. Treat missing CIM data as a reason to inspect Windows features/firmware, not permission to declare success. Hypervisor-present and a running service still do not prove that a particular phone image boots.
3. **Prepare Hyper-V using Windows tools.** Inspect Windows Features and the appropriate Hyper-V management tools. Enable required features and reboot only on the selected reference host, with its administrator's authorization. The probe deliberately does not do this.
4. **Install matching original phone tools.** Follow the Microsoft archive and package prerequisites. For the VS2013 path, install the required Visual Studio update, phone development tools and the matching WP8.1 Update emulator package. Check installer provenance and the final installed version.
5. **Inventory the installation again.** Typical locations include `Program Files (x86)/Microsoft XDE/<version>/XDE.exe` and `Program Files (x86)/Microsoft SDKs/Windows Phone/<version>/Emulation/Images/`. Use paths actually reported by the installation. Do not assume a specific image filename, guessed XDE arguments or a default VM name.
6. **Launch through the matching Visual Studio target selector.** Open a minimal project supported by that SDK, select an original phone emulator resolution and start deployment. The installed tools supply the appropriate image, differencing disk and VM parameters. Preserve the first boot/deployment error if launch fails; do not delete unrelated VMs or network switches as a generic repair.
7. **Verify the guest.** Inspect Settings → about / more information and record the OS build, emulator image, resolution, locale, theme, accent, navigation-bar mode and installed app versions. Navigate to Start and built-in apps to confirm the reference UI is running. Verify WP8.1 Update features such as live folders independently.
8. **Keep generations separate.** A WP7.1 image can validate earlier Metro details; WP8 validates WP8 behavior. Neither establishes Action Center or live-folder parity for the selected WP8.1 Update target. Similarly, a Windows 10 Mobile emulator is a different reference.

The SDK emulator does not prove real cellular, radio, sensor, lock-screen security or third-party service behavior. Use original hardware for those comparisons and record OEM/build differences.

## Capture recipe

Create one evidence record per original build and device profile. Record capture date, OS build, image/package version, physical pixel dimensions, emulator zoom, language, text-size setting, dark/light theme, accent and the scenario's test data. Keep reference and recreation captures in separate directories with the same scenario identifiers.

1. Capture the guest display using the installed emulator's screenshot facility, where available; otherwise capture the guest viewport at 1:1 and record the exact crop. Exclude bezel/desktop chrome from layout comparisons.
2. Begin with a **480-wide original logical reference**. For the recreation's 400-wide design, apply the explicit `5/6` conversion documented in [metro-design.md](metro-design.md). Do not compare physical screenshot pixels directly across different scale factors.
3. Capture Start, Apps search/group jump, Settings pivots, app-bar collapsed/expanded/Mini states, toggle/check/radio/slider states, People/mail/message lists, lock/Action Center and representative Office/picker flows. Include empty, long-text, disabled and error states.
4. Record complete pointer sequences: down position/time, move positions, up/cancel, expected state mutation and scroll/navigation result. Include short taps, held presses, slow partial pans, reverse/cancel gestures and flicks in both directions.
5. Record motion at a known frame rate using a lossless or high-quality capture. Preserve original timestamps. Compare normalized progress, angle/depth, opacity, stagger, gesture follow and completion; one still frame cannot verify easing or duration.
6. Match test data and settings in the recreation, then compare geometry and text baselines before pixel diffs. Separate font/rasterization differences from layout errors. Repeat at high pixel density and on real touch hardware for input and rendering qualification.
7. Retain a short discrepancy log with scenario, original evidence, recreation commit, observed difference, expected result and verification status. Mark inaccessible cloud functionality and privileged host features separately from local UI behavior.

The [validation matrix](metro-design.md#validation-matrix) gives the minimum state coverage. Pixel-identical claims require a matched reference and a declared tolerance; API compatibility and successful tests alone do not supply that proof.

## Videos: discovered versus inspected

| Original source | Work actually performed | What it supports |
| --- | --- | --- |
| [BUILD2011, Bryan Agnetta](https://learn.microsoft.com/en-us/shows/build-build2011/app-832t) | Retrieved Microsoft's media metadata/captions; inspected selected transcript ranges and an actual video frame at 46:10 | Original Metro design rationale, pivot/panorama roles and motion semantics. Not WP8.1 frame timing. |
| [TechEd Europe 2012, Corrina Black](https://learn.microsoft.com/en-us/shows/teched-2012/wph204) | Read the published session description | A primary research lead. Video content not yet inspected. |

The BUILD2011 [public metadata endpoint](https://learn.microsoft.com/api/video/public/v1/entries/4f79d713-47ca-4784-b655-2cdbfb380c38) returned a caption track and downloadable/streaming video URLs during research. The [caption track](https://videoencodingpublic-hgeaeyeba8gycee3.b01.azurefd.net/public-4f79d713-47ca-4784-b655-2cdbfb380c38/transcript.vtt) reports 58:24. Inspected ranges were 10:04–10:43, 23:00–24:30, 38:00–40:10 and 45:13–47:36. Automatic captions contain errors; use the Toolkit/API sources for exact constants. The full hour was not watched, and no video of an original WP8.1 emulator running in this environment exists.
