# Original Windows Phone emulator and capture procedure

Research/probe date: **2026-11-24**. The recreation targets **Windows Phone 8.1 Update (2014)**. An original emulator is a reference source for that operating system; running the FerroUI browser application is not an original emulator run.

**Reference setup outcome:** the authentic Microsoft offline ISO installed the original phone image, but vendor configuration suspended setup with a required restart and no XDE executable detected. **No original guest was booted or captured.** The [attempt index](verification/metro-refinement/original-emulator.json) keeps the host probes, web payload failure, corrected automation failure and final offline vendor outcome separate.

## Actual results on the available hosts

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

The same read-only probe also ran on the real `windows-2022` CI host at source [`c9923b2`](https://github.com/FerroUI/WindowsPhone/commit/c9923b2730e9fde63d4c822452f69ad46af30959). The [unmodified Windows report](verification/metro-refinement/original-emulator-windows.json) records **Windows Server 2022 Datacenter, AMD64**, `HypervisorPresent=true`, enabled Hyper-V features and running `vmms`/`vmcompute` services. CIM reported firmware virtualization enabled, but SLAT and VM-monitor extensions false. Those observations on an already virtualized host are not a conclusive nested-virtualization or guest-boot test.

Neither XDE, original phone SDK roots nor emulator images were found in the conventional Program Files locations; the recorded assessment is **`installation_not_detected`**. The target Update package lists Windows 8.1/10 and Visual Studio 2015 or Visual Studio 2013 Update 2 or later. This Server 2022 runner is not a listed host. Hyper-V service availability alone therefore does not supply an installed or supported original-phone reference environment. No SDK installation, emulator image launch, virtual-switch modification or reboot was attempted in that read-only probe job.

The probe artifact is [14909285800](https://github.com/FerroUI/WindowsPhone/actions/runs/45370106537/artifacts/14909285800), ZIP SHA-256 `552d595777971c97d6ab325f290465c9e6750e404521b1a8e64409c683cc96f3`. The Linux and Windows reports establish host inventory only. The earlier `c9923b2` Android and iOS runtime checks are separate platform-host evidence and do not establish an original WP8.1 boot.

### Original Microsoft web-installer attempt

A subsequent fresh `windows-2022` job actually attempted normal vendor setup at source [`535c376`](https://github.com/FerroUI/WindowsPhone/commit/535c376981e0e53088f77023f7cac8991260a264). [Run 45376175224](https://github.com/FerroUI/WindowsPhone/actions/runs/45376175224), job `138382958154`, reached the vendor installer and failed while acquiring its legacy payloads. The [unmodified result](verification/metro-refinement/original-emulator-web/result.json), [before](verification/metro-refinement/original-emulator-web/before.json)/[after](verification/metro-refinement/original-emulator-web/after.json) inventories and [watchdog record](verification/metro-refinement/original-emulator-web/watchdog.json) distinguish each stage:

| Stage | Observed result |
| --- | --- |
| Authentic catalog | Microsoft Download Center **43719**, guest/package version **8.10.14141.167** |
| Bootstrapper download | Exact advertised `windowsphone81sdkupdate1.exe`, **1,164,600 bytes**, HTTPS `download.microsoft.com`; SHA-256 `3b2f954d2bccb97ac4288f122b69ecc241ddae9bae743b5ec3a4b9c1a0f7f137` |
| Authenticode | **Valid**, publisher **Microsoft Corporation**, timestamped Microsoft signature |
| Bootstrapper identity | PE file/product version **12.0.40507.0**, original filename `MobileTools_EmulatorWP81GDR1.exe`; vendor log identifies Burn **3.7.3424.0**. These bootstrapper versions are distinct from the guest/package version. |
| Normal setup options | `/quiet /norestart /log <owned-log-path>`, following [Microsoft's VS2013 command-line setup documentation](https://learn.microsoft.com/en-us/previous-versions/visualstudio/visual-studio-2013/e2h7fzkw(v=vs.120)#using-command-line-parameters) |
| Vendor outcome | Exit **1603** (`0x00000643`); final Apply result **`0x80070002`**; no restart requested |
| Time bound | Download/setup phase **50.952 seconds**; independent parent watchdog **did not time out** |
| Installed reference | Before and after: **no XDE, phone SDK roots or emulator images detected** |
| Boot / parity | **No emulator launch, guest boot, reboot or original-device parity validation** |

The [numbered vendor-log excerpt](verification/metro-refinement/original-emulator-web/vendor-setup-excerpt.txt) records failed acquisition of `WindowsPhone81GDR1SDK_VHD` through Microsoft `LinkId=500159`, followed by failure of the required `CoreConExtraGDR1` package through `LinkId=497754`. BITS returned **`0x80190194`**, Microsoft's [`BG_E_HTTP_ERROR_404`](https://learn.microsoft.com/en-us/windows/win32/bits/bits-return-values). HTTP fallback/source resolution then returned **`0x80070002`** for `packages\WindowsPhone81GDR1VHD\WindowsPhoneSDK-VHD.msi` and `packages\CoreCon\MobileTools_WPImagesGDR1.msi`. The vendor log—not the general exit code 1603—establishes payload acquisition as the observed terminal cause.

This was **not an observed prerequisite rejection**: detection and planning completed, and the vendor log set `HypervisorSupported=1` and `HypervisorEnabled=1`. It also found no matching VS2013/Express application-key paths. The unsupported listed-host combination and missing matching Visual Studio prerequisites remain unresolved; successful detection does not validate them. This fresh runner's CIM firmware-virtualization, SLAT and VM-monitor fields were all false despite `HypervisorPresent=true` and running Hyper-V services. The earlier probe above reported firmware virtualization true on a different runner. Neither report is a nested guest-boot test.

The complete source artifact is [14911346543](https://github.com/FerroUI/WindowsPhone/actions/runs/45376175224/artifacts/14911346543), **29,460 bytes**, independently verified ZIP SHA-256 `6a677155e718f4f2b8051347458aa07fa3fd1f70c6f9c17c289a872427015d87`. The [evidence manifest](verification/metro-refinement/original-emulator-web/manifest.json) records original file hashes and clearly identifies the selected log excerpt; the full unmodified vendor log is retained in the source artifact. The artifact contains observations and logs, with no EXE, ISO, VHD, MSI, CAB or font redistribution.

### Official offline ISO investigation

The web-installer failure does not establish that all original media is unavailable. The same authentic catalog advertises [`windowsphone81sdkupdate1.iso`](https://download.microsoft.com/download/e/7/d/e7d9744a-06a6-46fb-aea4-50faaec6ce73/windowsphone81sdkupdate1.iso), **986,316,800 bytes**. A read-only HTTP range request returned `206` with that total length. Its [recorded ISO directory](verification/metro-refinement/original-emulator-iso/remote-directory.json) contains the root `MobileTools_EmulatorWP81GDR1.exe`, both missing MSI payloads, the CoreCon CAB, and the VHD CAB sequence `Vhd1.cab` through `Vhd1q.cab`. A separate range read of the root setup EXE produced the same SHA-256 as the signature-verified web bootstrapper above. Directory presence alone does not verify the complete image or successful installation.

The experimental [workflow](../.github/workflows/original-emulator.yml) has separate opt-in modes: `[verify-original-emulator]` selects the web bootstrapper; `[verify-original-emulator-iso]` selects the official offline ISO; manual dispatch offers both. The ISO mode streams the exact advertised size, records its full SHA-256, mounts it read-only using [Windows `Mount-DiskImage`](https://learn.microsoft.com/en-us/powershell/module/storage/mount-diskimage?view=windowsserver2022-ps), inventories its contents, and requires the unambiguous root SDK setup EXE with a valid Microsoft signature and matching product identity. It runs normal quiet/no-restart setup from the mounted layout so the vendor installer can resolve local payloads. The combined ISO download/setup budget is **15 minutes**, with a **16-minute** independent watchdog inside a **20-minute** job. Normal and interruption cleanup dismount only that attempt's owned image. The wrapper supplies no prerequisite overrides, direct MSI substitution, feature-management commands, reboot command or XDE launch. The vendor installer's own configuration actions are recorded below.

The first ISO-mode run, [45378014366](https://github.com/FerroUI/WindowsPhone/actions/runs/45378014366) at [`c59ac667`](https://github.com/FerroUI/WindowsPhone/commit/c59ac667b6253206ac33fa45ee14b297f5052f68), failed in the PowerShell wrapper before mounting or starting vendor setup. An asynchronous stream-write completion value leaked into the function's output, so accessing its intended `Bytes` metadata failed. The correction explicitly discards that write-completion value. The [unmodified result](verification/metro-refinement/original-emulator-iso/history/45378014366/result.json) and [history manifest](verification/metro-refinement/original-emulator-iso/history/45378014366/manifest.json) retain this automation failure independently of the vendor web-installer outcome. Artifact [14911224364](https://github.com/FerroUI/WindowsPhone/actions/runs/45378014366/artifacts/14911224364) has independently verified SHA-256 `b0861fd767a383927f45d14c1ce6868ac99b4e81eba1141e86047bead27721f4`. No full-ISO hash or vendor outcome was recorded in that run, and its watchdog did not time out.

### Actual offline vendor outcome

The corrected fresh-run attempt, [45378950959](https://github.com/FerroUI/WindowsPhone/actions/runs/45378950959), job `138392608347`, tested source [`05ac895`](https://github.com/FerroUI/WindowsPhone/commit/05ac89500978f087ebc66e1782e6ac4f88688835). It downloaded and mounted the full official ISO, verified the setup signature, and ran normal vendor installation. The offline layout supplied the previously missing VHD and CoreCon payloads successfully. Setup then suspended during its XDE/Hyper-V configurator with **`0x80048BC7` — “Incomplete and reboot required.”** The workflow correctly remained failed. See the [original result](verification/metro-refinement/original-emulator-iso/result.json), [mounted inventory](verification/metro-refinement/original-emulator-iso/iso-inventory.json) and [numbered vendor-log excerpt](verification/metro-refinement/original-emulator-iso/vendor-setup-excerpt.txt).

| Observation | Exact evidence |
| --- | --- |
| Complete ISO | **986,316,800 bytes**; runner-computed SHA-256 `ce5cba83133aa71c58353fd9e34ae143555adf78cb200f7ec77217a77b5d244d` |
| Mounted layout | **29 files**, volume `WINDOWSPHONE81SDKUPDATE1`; paths and byte counts match the earlier read-only range inventory |
| Setup executable | Root `MobileTools_EmulatorWP81GDR1.exe`, **Valid Microsoft Corporation** Authenticode signature; same bootstrapper SHA-256 and PE version as the web attempt |
| Image installation | `WindowsPhone81GDR1SDK_VHD` completed with `0x0`; MSI product version **8.1.14141** |
| CoreCon installation | `CoreConExtraGDR1` completed with `0x0`; the installer logged its image-configuration XML/XSL paths |
| Configurator | `MobileTools_XDEConfig81` returned MSI **1604**, then the bundle returned **`0x80048BC7`** |
| Post-install inventory | `C:\Program Files (x86)\Microsoft SDKs\Windows Phone\v8.1\Emulation\Images\flash.update1.vhd`, **1,773,481,984 bytes**; SDK `v8.1` root present; **XDE absent** in the probe's conventional locations |
| Cleanup / time bound | ISO dismounted and temporary image removed; **124.477 seconds** for download/setup; watchdog **did not time out** |
| Boot status | Bundle's final log says **`restarting: No`**; no host reboot or XDE/guest launch was performed |

The configurator's original [MSI log](verification/metro-refinement/original-emulator-iso/logs/setup_002_MobileTools_XDEConfig81.log), lines 452–465, reports Hyper-V as not enabled, attempts its own DISM configuration with exit **`-2146498548` / `0x800F080C`**, adds the current user to Hyper-V Administrators, and sets **`REBOOT_REQUIRED=True`**. Lines 471–508 schedule `ForceReboot` and a RunOnce continuation; the MSI ends with **1604**. Microsoft documents this as [`ERROR_INSTALL_SUSPEND`](https://learn.microsoft.com/en-us/windows/win32/msi/error-codes), distinct from completed installation with a restart pending. Its [reboot documentation](https://learn.microsoft.com/en-us/windows/win32/msi/system-reboots) explains that this suspended state requires the installation to resume. The bundle used `/norestart` and exited without restarting; no wrapper command forced a feature change or resumed installation.

The [before](verification/metro-refinement/original-emulator-iso/before.json) and [after](verification/metro-refinement/original-emulator-iso/after.json) CIM feature/service observations are identical: Hyper-V feature flags were enabled and services running. The legacy configurator's contrary assessment is retained alongside those observations on this Server 2022 runner; these artifacts do not establish that a reboot alone would repair setup. The Update package also expects existing matching Visual Studio/phone tools. No XDE executable was available for help/version inspection, and the actual installed CoreCon configuration contents were not captured. The image's guest build remains unverified until a real boot; package metadata and a VHD filename are insufficient.

**Raw-result interpretation:** this run's unmodified JSON contains `restart_required=false` because the helper at that revision only recognized exit 3010. Bundle log line 319 and configurator line 464 explicitly establish the required restart. The [derived manifest](verification/metro-refinement/original-emulator-iso/manifest.json) records that correction without changing the original JSON. The current helper classifies the observed HRESULT as `setup_incomplete_restart_required`, sets `restart_required=true`, and keeps a nonzero process exit. Other unrecognized failure codes leave restart status unknown (`null`). This metadata correction does not claim an additional setup run or a successful installation.

Artifact [14911819459](https://github.com/FerroUI/WindowsPhone/actions/runs/45378950959/artifacts/14911819459), **75,937 bytes**, was downloaded and its ZIP SHA-256 independently verified as `3b0eee5079e0295bc040b56c970fae69ce6cf0833ec1e872223a84600a4c2c5c`. The manifest preserves original file hashes; the result, probes, mount inventory, watchdog, cleanup record and all three MSI logs are copied byte-for-byte. The selected readable excerpt is explicitly labeled. The ISO and installed VHD were not uploaded or redistributed.

### Next reference-machine step

Continue on a separately provisioned compatible **Windows client** reference host where matching base tools and normal required restarts can be completed. The historical VS2013 emulator requirements specify **Windows 8.1 x64 Professional or higher**, CPU/firmware virtualization, **Client Hyper-V and SLAT**; the Update package requires **Visual Studio 2013 Update 2 or later**, or lists **Visual Studio 2015** as an alternative. Its catalog also lists Windows 10, but that does not qualify Server 2022 or a current Visual Studio installation as substitutes. Sources: [VS2013 emulator requirements](https://learn.microsoft.com/en-us/visualstudio/releases/2013/vs2013-sysrequirements-vs), [Update package prerequisites](https://www.microsoft.com/en-us/download/details.aspx?id=43719), and [the SDK archive's matching phone-tools guidance](https://learn.microsoft.com/en-us/windows/apps/windows-sdk/downloads-archive).

On that client host, install the matching phone-development tools and official Update media normally, complete required restarts, rerun the probe, inspect the installed XDE/version and configuration, then use the matching Visual Studio target selector for the first boot. Record the guest's actual OS build before making comparison captures. No further installation, bypass, feature forcing or reboot is planned on the unsupported disposable Server runner.

## Repeatable read-only probe

From the repository root:

```bash
python scripts/probe-original-emulator.py
python scripts/probe-original-emulator.py --json
```

On Windows, the script uses bounded, non-interactive PowerShell/CIM queries for OS architecture, hypervisor presence, processor SLAT/firmware indicators, Hyper-V optional-feature states, and relevant service states. It searches conventional Program Files locations for SDK roots, XDE executables and phone images. Custom installations may not be found. Permission failures remain **unknown** instead of being converted to false.

The script performs no network access, installation, elevation, registry writes, feature changes, service changes, VM creation or VM launch. It does not use `Win32_Product`, whose inventory behavior can trigger installer repair. File presence is reported separately from successful boot; a zero exit code means the inventory completed, not that the emulator is ready. The Windows branch was exercised on the CI host described above; it must still be rerun on the intended reference machine, followed by actual guest boot validation.

## Legitimate original packages and historical requirements

| Microsoft source | Verified page information | Availability limit |
| --- | --- | --- |
| [Windows SDK archive](https://learn.microsoft.com/en-us/windows/apps/windows-sdk/downloads-archive) | Lists WP8.1 tools, WP8.1 emulators, WP8.1 Update emulators, WP8 and WP7.x packages | Listing was retrieved. All linked installer payloads were not downloaded/installed. |
| [WP8.1 Update and Emulators, Download Center 43719](https://www.microsoft.com/en-us/download/details.aspx?id=43719) | Lists version **8.10.14141.167**, `windowsphone81sdkupdate1.iso` (940.6 MB) and `windowsphone81sdkupdate1.exe` (1.1 MB), plus WVGA/WXGA/qHD/720p/1080p emulator profiles | Signed web setup failed acquiring legacy payloads. The complete official ISO installed the VHD/CoreCon packages, then vendor configuration suspended with restart required and no XDE detected. No image was booted or redistributed. Listed hosts are Windows 8.1/10, with VS2015 or VS2013 Update 2 or later. |
| [WP8 SDK download center](https://www.microsoft.com/en-us/download/details.aspx?id=35471) | Windows 8 x64, 4 GB RAM, 6.5 GB free space; emulator requires Pro or higher and SLAT | WP8 prerequisites, not a statement that this is the target WP8.1 Update image. |
| [Visual Studio 2013 system requirements](https://learn.microsoft.com/en-us/visualstudio/releases/2013/vs2013-sysrequirements-vs) | Phone emulators require Windows 8.1 x64 Professional or higher, client Hyper-V and SLAT | Historical supported setup; current Windows versions must be qualified separately. |
| [Introducing WP8 SDK](https://devblogs.microsoft.com/visualstudio/introducing-windows-phone-sdk-8-0/) | WP8 WVGA/WXGA/720p images, plus older WP7.1 images that do not require SLAT | WP7.1 is an earlier behavioral baseline and cannot verify WP8.1 features. |

The archive describes WP8.1 emulator packages as additions to Visual Studio 2013 Update 2 or later, with a separate Update 1 package. It also points to Visual Studio Community 2015 for phone tools. Select the package whose **guest OS version** matches the intended reference. A product name or installed SDK folder alone does not prove that the guest runs WP8.1 Update.

Use Microsoft distribution links or an existing properly licensed installation. The SDK archive explicitly warns that these old installers are unserviced; use a dedicated reference machine or VM with controlled networking. This is practical guidance for running an obsolete OS, not a requirement to alter the development workstation. Do not redistribute emulator OS images or licensed fonts into this repository.

## Windows setup and first boot

The following is a manual procedure for a compatible Windows host. The bounded attempts above reached partial image installation, but did not complete this supported-host preparation or a first boot.

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
