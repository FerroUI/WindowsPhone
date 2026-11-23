#!/usr/bin/env python3
"""Bounded stack sampling of this harness's installed simulator app only."""
import datetime
import hashlib
import json
import pathlib
import plistlib
import re
import subprocess
import sys
import time
import uuid

BUNDLE_ID = "org.ferroui.windowsphone"
MAX_REPORT_BYTES = 4 * 1024 * 1024


def executable_for_bundle(app, bundle_id):
    app = pathlib.Path(app).resolve(strict=True)
    with (app / "Info.plist").open("rb") as stream:
        contents = stream.read(1024 * 1024 + 1)
    if len(contents) > 1024 * 1024:
        raise ValueError("The installed bundle metadata exceeded the diagnostic limit.")
    info = plistlib.loads(contents)
    if not isinstance(info, dict):
        raise ValueError("The installed bundle metadata is not a dictionary.")
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


def simulator_root(executable, simulator):
    parts = executable.parts
    matches = [index for index, part in enumerate(parts[:-2])
               if part == "Devices" and parts[index + 1].upper() == simulator.upper()
               and parts[index + 2] == "data"]
    if len(matches) != 1:
        raise ValueError("The executable is not in this disposable simulator.")
    return pathlib.Path(*parts[:matches[0] + 3])


def prepare(simulator, bundle_id, output):
    """Capture installation identity while simctl is responsive, before launch."""
    uuid.UUID(simulator)
    if bundle_id != BUNDLE_ID:
        raise ValueError("This collector only samples the Windows Phone test application.")
    output = pathlib.Path(output)
    output.mkdir(parents=True, exist_ok=True)
    installed = subprocess.run(
        ["/usr/bin/xcrun", "simctl", "get_app_container", simulator, bundle_id, "app"],
        capture_output=True, text=True, check=True, timeout=20).stdout.strip()
    executable = executable_for_bundle(installed, bundle_id)
    simulator_root(executable, simulator)
    metadata = executable.stat()
    record = {"schema": 1, "simulator": simulator, "bundle_id": bundle_id,
              "executable": str(executable), "inode": metadata.st_ino, "bytes": metadata.st_size,
              "prepared_at_unix_ms": time.time_ns() // 1_000_000}
    (output / "owned-installation.json").write_text(json.dumps(record, indent=2) + "\n")


def launch_pid(console):
    identifiers = set()
    for line in console.splitlines():
        match = re.fullmatch(re.escape(BUNDLE_ID) + r": (\d+)", line.strip())
        if not match:
            match = re.match(r"\[ios-smoke-(?:entry|phase)\] pid=(\d+) ", line)
        if match:
            identifiers.add(int(match[1]))
    if len(identifiers) > 1 or any(value <= 1 for value in identifiers):
        raise ValueError("The launch console contains ambiguous process identities.")
    return next(iter(identifiers), None)


def report_time(value):
    if not isinstance(value, str) or len(value) > 64:
        return None
    try:
        parsed = datetime.datetime.fromisoformat(re.sub(r" ([+-]\d{4})$", r"\1", value))
        return int(parsed.timestamp() * 1000) if parsed.tzinfo else None
    except ValueError:
        return None


def owned_crash(contents, identity, pid=None):
    # Apple's IPS schema has one metadata JSON line followed by crash JSON.
    # https://developer.apple.com/documentation/xcode/interpreting-the-json-format-of-a-crash-report
    header, separator, body = contents.partition(b"\n")
    if not separator or len(header) > 64 * 1024 or len(contents) > MAX_REPORT_BYTES:
        return None
    metadata, report = json.loads(header), json.loads(body)
    if not isinstance(metadata, dict) or not isinstance(report, dict):
        return None
    if metadata.get("bug_type") != "309" or metadata.get("platform") != 7:
        return None
    bundle = report.get("bundleInfo")
    identifiers = (metadata.get("bundleID"), bundle.get("CFBundleIdentifier") if isinstance(bundle, dict) else None)
    if identity["bundle_id"] not in identifiers or any(value not in (None, identity["bundle_id"]) for value in identifiers):
        return None
    executable = pathlib.Path(identity["executable"])
    simulator_root(executable, identity["simulator"])
    allowed_paths = {str(executable)}
    # macOS may redact only the user component. Every simulator/container/app
    # component after it must still match the exact prelaunch installation.
    parts = executable.parts
    if len(parts) > 3 and parts[1] == "Users":
        allowed_paths.add(str(pathlib.Path("/Users/USER", *parts[3:])))
    if report.get("procPath") not in allowed_paths or report.get("procName") != executable.name:
        return None
    reported_pid = report.get("pid")
    if type(reported_pid) is not int or reported_pid <= 1 or (pid is not None and pid != reported_pid):
        return None
    launched = report_time(report.get("procLaunch"))
    captured = report_time(report.get("captureTime"))
    if launched is None or captured is None or launched < identity["prepared_at_unix_ms"] or captured < launched:
        return None
    return {"pid": reported_pid, "proc_launch": report["procLaunch"], "capture_time": report["captureTime"],
            "exception": report.get("exception"), "termination": report.get("termination")}


def collect_crashes(identity, output, pid=None):
    executable = pathlib.Path(identity["executable"])
    device_data = simulator_root(executable, identity["simulator"])
    directories = [pathlib.Path.home() / "Library/Logs/DiagnosticReports",
                   device_data / "Library/Logs/CrashReporter"]
    retained, seen = [], set()
    for directory in directories:
        # Only this executable's recent reports are examined; no global log dump.
        # Apple documents <AppBinaryName>_<DateTime>; modern reports also use '-'.
        for path in sorted(directory.glob(executable.name + "[-_]*.ips"))[-32:]:
            if len(retained) >= 3:
                return retained
            if path.is_symlink() or not path.is_file():
                continue
            metadata = path.stat()
            if metadata.st_size > MAX_REPORT_BYTES or metadata.st_mtime_ns // 1_000_000 < identity["prepared_at_unix_ms"]:
                continue
            try:
                with path.open("rb") as stream:
                    contents = stream.read(MAX_REPORT_BYTES + 1)
                report = owned_crash(contents, identity, pid)
            except (OSError, ValueError):
                continue
            if report is None:
                continue
            digest = hashlib.sha256(contents).hexdigest()
            if digest in seen:
                continue
            seen.add(digest)
            name = f"owned-crash-{len(retained) + 1}.ips"
            (output / name).write_bytes(contents)
            retained.append({**report, "file": name, "sha256": digest, "bytes": len(contents)})
    return retained


def collect(simulator, bundle_id, output):
    # No simctl IPC is attempted after failure. Never sample by a basename,
    # a global pgrep, or an unvalidated console/proof PID.
    uuid.UUID(simulator)
    if bundle_id != BUNDLE_ID:
        raise ValueError("Unexpected diagnostic bundle identity.")
    output = pathlib.Path(output)
    record = {"schema": 1, "simulator": simulator, "bundle_id": bundle_id,
              "captured_at_unix_ms": time.time_ns() // 1_000_000}
    identity, expected_pid, crash_identity_verified = None, None, False
    try:
        identity = json.loads((output / "owned-installation.json").read_text())
        if not isinstance(identity, dict) or identity.get("simulator") != simulator or identity.get("bundle_id") != bundle_id:
            raise ValueError("Cached installation belongs to another simulator or application.")
        if type(identity.get("prepared_at_unix_ms")) is not int or identity["prepared_at_unix_ms"] <= 0:
            raise ValueError("Cached installation lacks a valid prelaunch timestamp.")
        executable = pathlib.Path(identity["executable"])
        simulator_root(executable, simulator)
        with (output / "console.log").open() as stream:
            expected_pid = launch_pid(stream.read(1024 * 1024))
        crash_identity_verified = True
        record["console_pid"] = expected_pid
        actual = executable_for_bundle(executable.parent, bundle_id)
        metadata = actual.stat()
        if actual != executable or metadata.st_ino != identity["inode"] or metadata.st_size != identity["bytes"]:
            raise ValueError("The installed executable changed after the prelaunch observation.")
        processes = subprocess.run(["/bin/ps", "-axww", "-o", "pid=,comm="],
                                   capture_output=True, text=True, check=True, timeout=4).stdout
        pid = owned_pid(processes, executable)
        if expected_pid is not None and expected_pid != pid:
            raise ValueError("The launch console and live installed executable have different process identities.")
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
    except (OSError, ValueError, KeyError, plistlib.InvalidFileException, subprocess.SubprocessError) as error:
        record["diagnostic_error"] = str(error)[-4096:]
    if crash_identity_verified:
        try:
            record["crash_reports"] = collect_crashes(identity, output, expected_pid)
        except (OSError, ValueError) as error:
            record["crash_diagnostic_error"] = str(error)[-4096:]
    (output / "owned-process.json").write_text(json.dumps(record, indent=2) + "\n")


if __name__ == "__main__":
    if len(sys.argv) == 5 and sys.argv[1] == "--prepare":
        prepare(*sys.argv[2:])
    elif len(sys.argv) == 4:
        collect(*sys.argv[1:])
    else:
        raise SystemExit("Usage: collect-ios-failure.py [--prepare] SIMULATOR BUNDLE_ID OUTPUT")
