#!/usr/bin/env python3
"""Bounded stack sampling of this harness's installed simulator app only."""
import json
import pathlib
import plistlib
import re
import subprocess
import sys
import time
import uuid


def executable_for_bundle(app, bundle_id):
    app = pathlib.Path(app).resolve(strict=True)
    with (app / "Info.plist").open("rb") as stream:
        contents = stream.read(1024 * 1024 + 1)
    if len(contents) > 1024 * 1024:
        raise ValueError("The installed bundle metadata exceeded the diagnostic limit.")
    info = plistlib.loads(contents)
    name = info.get("CFBundleExecutable")
    if info.get("CFBundleIdentifier") != bundle_id:
        raise ValueError("The installed bundle identity does not match this harness.")
    if not isinstance(name, str) or not re.fullmatch(r"[A-Za-z0-9_.-]{1,128}", name):
        raise ValueError("The installed executable name is invalid.")
    executable = (app / name).resolve(strict=True)
    if executable.parent != app or not executable.is_file():
        raise ValueError("The executable must belong to the installed app bundle.")
    return executable


def owned_pid(processes, executable):
    matches = []
    for row in processes.splitlines():
        fields = row.strip().split(None, 1)
        if len(fields) != 2 or not fields[0].isdigit():
            continue
        if int(fields[0]) > 1 and fields[1] == str(executable):
            matches.append(int(fields[0]))
    if len(matches) != 1:
        raise ValueError("Expected exactly one process for the owned installed executable.")
    return matches[0]


def collect(simulator, bundle_id, output):
    # Never sample by an executable basename, a global pgrep or a stale proof PID.
    uuid.UUID(simulator)
    if bundle_id != "org.ferroui.windowsphone":
        raise ValueError("This collector only samples the Windows Phone test application.")
    output = pathlib.Path(output)
    output.mkdir(parents=True, exist_ok=True)
    record = {"schema": 1, "simulator": simulator, "bundle_id": bundle_id,
              "captured_at_unix_ms": time.time_ns() // 1_000_000}
    try:
        installed = subprocess.run(
            ["/usr/bin/xcrun", "simctl", "get_app_container", simulator, bundle_id, "app"],
            capture_output=True, text=True, check=True, timeout=6).stdout.strip()
        # The container comes from this disposable simulator's own app registry.
        executable = executable_for_bundle(installed, bundle_id)
        processes = subprocess.run(["/bin/ps", "-axww", "-o", "pid=,comm="],
                                   capture_output=True, text=True, check=True, timeout=4).stdout
        pid = owned_pid(processes, executable)
        current = subprocess.run(["/bin/ps", "-p", str(pid), "-o", "comm="],
                                 capture_output=True, text=True, check=True, timeout=4).stdout.strip()
        if current != str(executable):
            raise ValueError("The selected process exited or changed identity before sampling.")
        record.update({"pid": pid, "executable": executable.name,
                       "identity": "exact installed executable path, rechecked before sample"})
        sampled = subprocess.run(
            ["/usr/bin/sample", str(pid), "3", "10", "-file", str(output / "owned-process.sample.txt")],
            capture_output=True, text=True, timeout=8)
        record.update({"sample_exit_code": sampled.returncode,
                       "sample_output": (sampled.stdout + sampled.stderr)[-4096:],
                       "sample_duration_seconds": 3, "sample_interval_ms": 10})
    except (OSError, ValueError, plistlib.InvalidFileException, subprocess.SubprocessError) as error:
        record["diagnostic_error"] = str(error)[-4096:]
    (output / "owned-process.json").write_text(json.dumps(record, indent=2) + "\n")


if __name__ == "__main__":
    if len(sys.argv) != 4:
        raise SystemExit("Usage: collect-ios-failure.py SIMULATOR BUNDLE_ID OUTPUT")
    collect(*sys.argv[1:])
