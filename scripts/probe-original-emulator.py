#!/usr/bin/env python3
"""Read-only inventory for an original Microsoft Windows Phone emulator host.

No packages are downloaded, features enabled, services started, or VMs changed.
Windows CIM queries may be unavailable under restricted accounts; unknown is
reported separately from false. Exit status describes the probe, not readiness.
"""

from __future__ import annotations

import argparse
import base64
import datetime as dt
import json
import os
import platform
import shutil
import subprocess
from pathlib import Path
from typing import Any


WINDOWS_QUERY = r"""
$ErrorActionPreference = 'Stop'
[Console]::OutputEncoding = New-Object System.Text.UTF8Encoding($false)
$result = [ordered]@{}
try {
  $result.os = Get-CimInstance Win32_OperatingSystem |
    Select-Object Caption, Version, OSArchitecture
} catch { $result.os_error = $_.Exception.Message }
try {
  $result.computer = Get-CimInstance Win32_ComputerSystem |
    Select-Object HypervisorPresent
} catch { $result.computer_error = $_.Exception.Message }
try {
  $result.processors = @(Get-CimInstance Win32_Processor |
    Select-Object Architecture, SecondLevelAddressTranslationExtensions,
      VirtualizationFirmwareEnabled, VMMonitorModeExtensions)
} catch { $result.processors_error = $_.Exception.Message }
try {
  $result.hyperv_features = @(Get-CimInstance Win32_OptionalFeature |
    Where-Object { $_.Name -like 'Microsoft-Hyper-V*' } |
    Select-Object Name, InstallState)
} catch { $result.hyperv_features_error = $_.Exception.Message }
try {
  $result.hyperv_services = @(Get-Service -Name vmms,vmcompute -ErrorAction SilentlyContinue |
    Select-Object Name, @{Name='Status';Expression={$_.Status.ToString()}})
} catch { $result.hyperv_services_error = $_.Exception.Message }
$result | ConvertTo-Json -Depth 6 -Compress
"""


def cpu_virtualization_flags() -> list[str] | None:
    """Linux flags are host-visible evidence, not a firmware capability test."""
    try:
        tokens = set(Path("/proc/cpuinfo").read_text(encoding="utf-8").split())
    except OSError:
        return None
    return sorted(tokens.intersection({"vmx", "svm"}))


def windows_inventory(timeout: float) -> dict[str, Any]:
    executable = shutil.which("powershell.exe") or shutil.which("pwsh.exe")
    if not executable:
        return {"status": "unavailable", "reason": "PowerShell was not found on PATH."}
    command = base64.b64encode(WINDOWS_QUERY.encode("utf-16-le")).decode("ascii")
    try:
        result = subprocess.run(
            [executable, "-NoProfile", "-NonInteractive", "-EncodedCommand", command],
            capture_output=True,
            timeout=timeout,
            check=False,
        )
    except (OSError, subprocess.TimeoutExpired) as exc:
        return {"status": "unavailable", "reason": str(exc)}
    if result.returncode:
        return {
            "status": "unavailable",
            "reason": result.stderr.decode("utf-8", errors="replace").strip()[:2000],
            "exit_code": result.returncode,
        }
    try:
        data = json.loads(result.stdout.decode("utf-8-sig"))
    except (UnicodeError, json.JSONDecodeError) as exc:
        return {"status": "unavailable", "reason": f"CIM response was not JSON: {exc}"}
    if not isinstance(data, dict):
        return {"status": "unavailable", "reason": "CIM response was not an object."}
    return {"status": "observed", "data": data}


def sdk_installations() -> dict[str, Any]:
    """Inspect bounded, conventional installation paths without executing XDE."""
    roots = sorted(
        {Path(value) for key in ("ProgramFiles(x86)", "ProgramFiles", "ProgramW6432")
         if (value := os.environ.get(key))},
        key=str,
    )
    result: dict[str, Any] = {"searched_roots": [str(p) for p in roots], "xde": [], "images": [], "sdk_roots": [], "errors": []}
    for root in roots:
        patterns = {
            "xde": "Microsoft XDE/*/[Xx][Dd][Ee].[Ee][Xx][Ee]",
            "sdk_roots": "Microsoft SDKs/Windows Phone/v*",
            "images": "Microsoft SDKs/Windows Phone/v*/Emulation/Images/*",
        }
        for category, pattern in patterns.items():
            try:
                for entry in sorted(root.glob(pattern), key=str):
                    if category == "images" and entry.suffix.lower() not in {".vhd", ".vhdx", ".bin"}:
                        continue
                    if category == "sdk_roots" and not entry.is_dir():
                        continue
                    if category != "sdk_roots" and not entry.is_file():
                        continue
                    result[category].append(str(entry))
                    if len(result[category]) >= 128:
                        break
            except OSError as exc:
                result["errors"].append(str(exc))
    for category in ("xde", "images", "sdk_roots"):
        result[category] = sorted(set(result[category]))[:128]
    return result


def assess(report: dict[str, Any]) -> tuple[str, list[str]]:
    if report["platform"]["system"] != "Windows":
        return "unsupported_host_os", [
            "The original Windows Phone 8/8.1 XDE requires a compatible x64 Windows host with Hyper-V and SLAT.",
            "Use docs/original-emulator.md to prepare an original Windows emulator or hardware reference.",
        ]
    if report["platform"]["machine"].lower() not in {"amd64", "x86_64"}:
        return "host_architecture_unverified", [
            "An x64 Windows process or emulated application does not establish support for the original x86 phone VM.",
            "Use a compatible x64 Windows/Hyper-V host and verify the original SDK requirements.",
        ]
    if not report["installations"]["xde"] or not report["installations"]["images"]:
        return "installation_not_detected", [
            "XDE and phone images were not both found in conventional installation locations; a custom location may exist.",
            "Consult the Microsoft SDK archive and docs/original-emulator.md for the appropriate licensed SDK/image.",
        ]
    return "boot_validation_required", [
        "XDE and image files are present; this does not prove that virtualization, the selected OS image, or boot works.",
        "Review the CIM feature/CPU results, then launch the selected original image through its matching Visual Studio tools.",
        "Record the actual OS build and capture the reference scenarios in docs/original-emulator.md.",
    ]


def probe(timeout: float) -> dict[str, Any]:
    system = platform.system()
    report: dict[str, Any] = {
        "schema_version": 1,
        "observed_at_utc": dt.datetime.now(dt.timezone.utc).isoformat(),
        "read_only": True,
        "platform": {"system": system, "release": platform.release(), "machine": platform.machine()},
        "cpu_virtualization_flags": cpu_virtualization_flags(),
        "kvm_device_present": Path("/dev/kvm").exists(),
        "tools": {name: shutil.which(name) for name in ("qemu-system-x86_64", "wine", "powershell.exe", "pwsh")},
        "windows": windows_inventory(timeout) if system == "Windows" else {"status": "not_applicable"},
        "installations": sdk_installations() if system == "Windows" else {"searched_roots": [], "xde": [], "images": [], "sdk_roots": [], "errors": []},
    }
    report["assessment"], report["next_steps"] = assess(report)
    return report


def positive_timeout(value: str) -> float:
    result = float(value)
    if not 0 < result <= 60:
        raise argparse.ArgumentTypeError("timeout must be greater than zero and at most 60 seconds")
    return result


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--json", action="store_true", help="print a machine-readable report")
    parser.add_argument("--timeout", type=positive_timeout, default=15.0, help="Windows CIM timeout in seconds (default: 15; maximum: 60)")
    args = parser.parse_args()
    report = probe(args.timeout)
    if args.json:
        print(json.dumps(report, indent=2, ensure_ascii=False))
        return
    print("Original Windows Phone emulator: read-only environment probe")
    print(f"Host: {report['platform']['system']} {report['platform']['release']} {report['platform']['machine']}")
    print(f"Assessment: {report['assessment']}")
    print(f"Visible VMX/SVM flags: {report['cpu_virtualization_flags']}")
    print(f"/dev/kvm present: {report['kvm_device_present']}")
    for name, path in report["tools"].items():
        print(f"{name}: {path or 'not found'}")
    if report["platform"]["system"] == "Windows":
        print("Windows CIM observations (InstallState: 1=enabled, 2=disabled, 3=absent, 4=unknown):")
        print(json.dumps(report["windows"], indent=2, ensure_ascii=False))
        print("Detected SDK/XDE/image locations:")
        print(json.dumps(report["installations"], indent=2, ensure_ascii=False))
    for step in report["next_steps"]:
        print(f"- {step}")


if __name__ == "__main__":
    main()
