#!/usr/bin/env python3
"""Check the ownership boundary before a failed simulator process is sampled."""
import importlib.util
import json
import os
import pathlib
import plistlib
import shlex
import signal
import subprocess
import sys
import tempfile
import unittest
from unittest import mock

sys.dont_write_bytecode = True
spec = importlib.util.spec_from_file_location("ios_failure", pathlib.Path(__file__).with_name("collect-ios-failure.py"))
module = importlib.util.module_from_spec(spec)
spec.loader.exec_module(module)


class OwnedProcessTests(unittest.TestCase):
    def test_ignores_another_simulator_and_matching_command_arguments(self):
        executable = pathlib.Path("/simulator/ours/Phone.app/phone-ios")
        processes = "\n".join([
            "71 /simulator/other/Phone.app/phone-ios",
            "72 /usr/bin/sample /simulator/ours/Phone.app/phone-ios",
            "73 /simulator/ours/Phone.app/phone-ios",
            "74 /simulator/ours/Phone.app/phone-ios --extra",
        ])
        self.assertEqual(module.owned_pid(processes, executable), 73)
        for invalid in ("", "1 " + str(executable),
                        "73 " + str(executable) + "\n75 " + str(executable)):
            with self.subTest(processes=invalid), self.assertRaises(ValueError):
                module.owned_pid(invalid, executable)

    def test_rejects_wrong_bundle_and_executable_escape(self):
        with tempfile.TemporaryDirectory() as directory:
            app = pathlib.Path(directory) / "Phone.app"
            app.mkdir()
            binary = app / "phone-ios"
            binary.write_bytes(b"owned")
            info = {"CFBundleIdentifier": "org.ferroui.windowsphone", "CFBundleExecutable": "phone-ios"}
            (app / "Info.plist").write_bytes(plistlib.dumps(info))
            self.assertEqual(module.executable_for_bundle(app, info["CFBundleIdentifier"]), binary.resolve())
            with self.assertRaises(ValueError):
                module.executable_for_bundle(app, "org.other.app")
            for name in ("../phone-ios", "/usr/bin/sample", "", ".."):
                with self.subTest(name=name), self.assertRaises(ValueError):
                    (app / "Info.plist").write_bytes(plistlib.dumps({**info, "CFBundleExecutable": name}))
                    module.executable_for_bundle(app, info["CFBundleIdentifier"])
            binary.unlink()
            outside = pathlib.Path(directory) / "other"
            outside.write_bytes(b"not owned")
            binary.symlink_to(outside)
            (app / "Info.plist").write_bytes(plistlib.dumps(info))
            with self.assertRaises(ValueError):
                module.executable_for_bundle(app, info["CFBundleIdentifier"])

    def test_console_rejects_restart_or_ambiguous_processes(self):
        self.assertEqual(module.launch_pid("[ios-smoke-entry] pid=42 process.main\n"
                                          "org.ferroui.windowsphone: 42\n"), 42)
        self.assertIsNone(module.launch_pid("unrelated output\n"))
        with self.assertRaises(ValueError):
            module.launch_pid("[ios-smoke-phase] pid=42 sequence=1 begin\n"
                              "org.ferroui.windowsphone: 43\n")
        with self.assertRaises(ValueError):
            module.launch_pid("org.ferroui.windowsphone: 0\n")

    def test_console_observation_preserves_the_version_seen_before_late_output(self):
        with tempfile.TemporaryDirectory() as directory:
            path = pathlib.Path(directory) / "console.log"
            path.write_bytes(b"")
            before = module.console_observation(path)
            path.write_bytes(b"org.ferroui.windowsphone: 34323\n")
            after = module.console_observation(path)
            self.assertEqual(before["bytes"], 0)
            self.assertIsNone(before["parsed_pid"])
            self.assertEqual(after["bytes"], 32)
            self.assertEqual(after["parsed_pid"], 34323)
            self.assertNotEqual(before["read_sha256"], after["read_sha256"])
            self.assertEqual(after["modified_at_unix_ns"], path.stat().st_mtime_ns)
            self.assertGreaterEqual(after["observed_at_unix_ms"], before["observed_at_unix_ms"])
            for invalid in (b"org.ferroui.windowsphone: 343", b"org.ferroui.windowsphone: 42\n"
                            b"org.ferroui.windowsphone: 43\n", b"x" * (module.MAX_CONSOLE_BYTES + 1)):
                with self.subTest(bytes=len(invalid)):
                    path.write_bytes(invalid)
                    result = module.console_observation(path)
                    self.assertIn("identity_error", result)
                    self.assertIsNone(result["parsed_pid"])
                    self.assertLessEqual(result["read_bytes"], module.MAX_CONSOLE_BYTES)

    def test_launch_driver_records_do_not_sample_or_claim_a_native_process(self):
        with tempfile.TemporaryDirectory() as directory:
            root = pathlib.Path(directory)
            (root / "console.log").write_text("org.ferroui.windowsphone: 34323\n")
            with mock.patch.object(module.subprocess, "run") as commands:
                module.record_launch_driver(42, "started", "running", root)
                module.record_launch_driver(42, "driver-exited", "not_running", root, exit_code=17)
            commands.assert_not_called()
            started = json.loads((root / "launch-driver-started.json").read_text())
            self.assertEqual(started["driver_pid"], 42)
            self.assertEqual(started["console"]["parsed_pid"], 34323)
            self.assertIsNone(started["observed_exit_code"])
            ended = json.loads((root / "launch-driver-driver-exited.json").read_text())
            self.assertEqual(ended["observed_exit_code"], 17)
            for pid, event, state, code in ((1, "started", "running", None),
                                            (42, "../elsewhere", "running", None),
                                            (42, "started", "running", 0),
                                            (42, "driver-exited", "not_running", 256)):
                with self.subTest(event=event, code=code), self.assertRaises(ValueError):
                    module.record_launch_driver(pid, event, state, root, exit_code=code)

    def test_crash_requires_exact_simulator_installation_bundle_process_and_time(self):
        simulator = "31E50930-5D50-4FC2-A00B-35E79219E55D"
        executable = f"/Users/runner/Library/Developer/CoreSimulator/Devices/{simulator}/data/Containers/Bundle/Application/APP-ID/Phone.app/phone-ios"
        identity = {"simulator": simulator, "bundle_id": module.BUNDLE_ID, "executable": executable,
                    "prepared_at_unix_ms": 1767225600000}
        metadata = {"bug_type": "309", "platform": 7, "bundleID": module.BUNDLE_ID}
        report = {"procName": "phone-ios", "procPath": executable, "pid": 42,
                  "bundleInfo": {"CFBundleIdentifier": module.BUNDLE_ID},
                  "procLaunch": "2026-01-01 00:00:01.000 +0000", "captureTime": "2026-01-01 00:00:02.000 +0000"}
        def encoded(value, header=metadata):
            return (json.dumps(header) + "\n" + json.dumps(value)).encode()
        self.assertEqual(module.owned_crash(encoded(report), identity, 42)["pid"], 42)
        self.assertIsNotNone(module.owned_crash(encoded({**report, "bundleInfo": None}), identity, 42))
        redacted = {**report, "procPath": executable.replace("/Users/runner/", "/Users/USER/")}
        self.assertIsNotNone(module.owned_crash(encoded(redacted), identity, 42))
        for change in ({"pid": 43}, {"pid": True}, {"procPath": executable.replace("APP-ID", "OTHER-INSTALL")},
                       {"procPath": executable.replace(simulator, "DIFFERENT-DEVICE")},
                       {"bundleInfo": {"CFBundleIdentifier": "org.other"}},
                       {"procLaunch": "2025-12-31 23:59:59.000 +0000"}, {"captureTime": "invalid"}):
            with self.subTest(change=change):
                self.assertIsNone(module.owned_crash(encoded({**report, **change}), identity, 42))
        self.assertIsNone(module.owned_crash(encoded(report, {**metadata, "platform": 1}), identity, 42))
        with self.assertRaises(ValueError):
            module.simulator_root(pathlib.Path(executable), "BC403C94-288A-44E8-AF3C-0CFD6D3A174E")

    def test_failure_sampler_uses_cached_identity_without_core_simulator_ipc(self):
        simulator = "31E50930-5D50-4FC2-A00B-35E79219E55D"
        with tempfile.TemporaryDirectory() as directory:
            root = pathlib.Path(directory).resolve()
            app = root / "Devices" / simulator / "data/Containers/Bundle/Application/APP-ID/Phone.app"
            app.mkdir(parents=True)
            executable = app / "phone-ios"
            executable.write_bytes(b"owned app")
            (app / "Info.plist").write_bytes(plistlib.dumps({
                "CFBundleIdentifier": module.BUNDLE_ID, "CFBundleExecutable": "phone-ios"}))
            metadata = executable.stat()
            (root / "owned-installation.json").write_text(json.dumps({
                "simulator": simulator, "bundle_id": module.BUNDLE_ID, "executable": str(executable),
                "inode": metadata.st_ino, "bytes": metadata.st_size, "prepared_at_unix_ms": 1767225600000}))
            (root / "console.log").write_text("[ios-smoke-entry] pid=42 process.main\n")
            def process(command, **options):
                self.assertNotEqual(command[0], "/usr/bin/xcrun")
                self.assertLessEqual(options["timeout"], 8)
                if command[0] == "/usr/bin/sample":
                    self.assertEqual(command[1], "42")
                    output = "sample completed"
                elif "pid=,comm=" in command:
                    output = f"42 {executable}\n73 /unrelated/phone-ios\n"
                else:
                    output = str(executable) + "\n"
                return subprocess.CompletedProcess(command, 0, output, "")
            with mock.patch.object(module.subprocess, "run", side_effect=process) as calls, \
                    mock.patch.object(module, "collect_crashes", return_value=[]):
                module.collect(simulator, module.BUNDLE_ID, root)
            result = json.loads((root / "owned-process.json").read_text())
            self.assertEqual(result["pid"], 42)
            self.assertEqual(result["sample_exit_code"], 0)
            self.assertEqual(calls.call_count, 3)
            self.assertEqual(result["exact_process_matches"]["classification"], "unique")
            self.assertEqual(result["console_observation"]["parsed_pid"], 42)

            for candidates, classification in (([], "none"), ([42, 43], "multiple"), ([43], "unique")):
                with self.subTest(candidates=candidates):
                    rows = "\n".join(f"{pid} {executable}" for pid in candidates)
                    with mock.patch.object(module.subprocess, "run", return_value=
                            subprocess.CompletedProcess([], 0, rows, "")) as calls, \
                            mock.patch.object(module, "collect_crashes", return_value=[]):
                        module.collect(simulator, module.BUNDLE_ID, root)
                    failed = json.loads((root / "owned-process.json").read_text())
                    self.assertEqual(failed["exact_process_matches"]["count"], len(candidates))
                    self.assertEqual(failed["exact_process_matches"]["classification"], classification)
                    self.assertIn("diagnostic_error", failed)
                    self.assertNotIn("pid", failed)
                    self.assertEqual(calls.call_count, 1)
                    self.assertNotIn("/usr/bin/sample", calls.call_args.args[0])

    def test_crash_discovery_accepts_both_filename_delimiters_with_exact_ownership(self):
        simulator = "31E50930-5D50-4FC2-A00B-35E79219E55D"
        with tempfile.TemporaryDirectory() as directory:
            root = pathlib.Path(directory).resolve()
            reports = root / "Library/Logs/DiagnosticReports"
            reports.mkdir(parents=True)
            executable = root / "Devices" / simulator / "data/Containers/Bundle/Application/APP-ID/Phone.app/phone-ios"
            identity = {"simulator": simulator, "bundle_id": module.BUNDLE_ID,
                        "executable": str(executable), "prepared_at_unix_ms": 1767225600000}
            header = {"bug_type": "309", "platform": 7, "bundleID": module.BUNDLE_ID}
            report = {"procName": "phone-ios", "procPath": str(executable), "pid": 42,
                      "bundleInfo": {"CFBundleIdentifier": module.BUNDLE_ID},
                      "procLaunch": "2026-01-01 00:00:01.000 +0000",
                      "captureTime": "2026-01-01 00:00:02.000 +0000"}
            expected = []
            for delimiter, captured in (("_", "02"), ("-", "03")):
                contents = (json.dumps(header) + "\n" + json.dumps({
                    **report, "captureTime": f"2026-01-01 00:00:{captured}.000 +0000"})).encode()
                (reports / f"phone-ios{delimiter}2026-01-01-0000{captured}.ips").write_bytes(contents)
                expected.append(contents)
            for index, (metadata, changed) in enumerate((
                ({**header, "bundleID": "org.other"}, report),
                (header, {**report, "procPath": str(executable).replace("APP-ID", "OTHER-INSTALL")}),
                (header, {**report, "pid": 43}),
            )):
                (reports / f"phone-ios_2026-01-01-00001{index}.ips").write_bytes(
                    (json.dumps(metadata) + "\n" + json.dumps(changed)).encode())
            with mock.patch.object(module.pathlib.Path, "home", return_value=root):
                retained = module.collect_crashes(identity, root, 42)
            self.assertEqual(len(retained), 2)
            self.assertCountEqual([(root / item["file"]).read_bytes() for item in retained], expected)
            self.assertTrue(all(item["pid"] == 42 for item in retained))


class SmokeCleanupTests(unittest.TestCase):
    def exercise_cleanup(self, result, mode):
        # Execute the production EXIT-trap function with real owned children.
        # Only simulator/diagnostic commands and the poll delay are substituted.
        source = pathlib.Path(__file__).with_name("smoke-ios.sh").read_text()
        cleanup = source[source.index("cleanup() {\n"):source.index("\ntrap cleanup EXIT\n")]
        with tempfile.TemporaryDirectory() as directory:
            root = pathlib.Path(directory)
            (root / "scripts").mkdir()
            (root / "phase.txt").write_text("Starting the XCTest driver\n")
            (root / "driver.py").write_text('''import os, pathlib, signal, sys, time
root, mode = pathlib.Path(sys.argv[1]), sys.argv[2]
def record(event):
    with (root / "events").open("a") as stream: stream.write(event + "\\n")
def terminated(signum, frame):
    record("driver-terminated")
    raise SystemExit(0)
signal.signal(signal.SIGTERM, signal.SIG_IGN if mode == "ignore" else terminated)
(root / "driver.pid").write_text(str(os.getpid()))
if mode == "graceful":
    while not (root / "finished").exists(): time.sleep(0.005)
    record("driver-finished")
else:
    while True: signal.pause()
''')
            (root / "scripts" / "collect-ios-failure.py").write_text('''import json, os, pathlib, sys
root = pathlib.Path(sys.argv[3])
def alive(pid):
    try: os.kill(pid, 0); return True
    except ProcessLookupError: return False
(root / "diagnostics.json").write_text(json.dumps({
    "driver_running": alive(int((root / "driver.pid").read_text())),
    "native_running": alive(int(os.environ["SMOKE_TEST_NATIVE_PID"]))}))
with (root / "events").open("a") as stream: stream.write("diagnostics\\n")
''')
            native = subprocess.Popen([sys.executable, "-c", "import time; time.sleep(30)"])
            script = "\n".join([
                "set -uo pipefail", cleanup,
                "root=" + shlex.quote(str(root)), "out=$root",
                "udid=owned-simulator", "bundle_id=org.ferroui.windowsphone",
                'ui_done="$root/finished"',
                'record_launch_driver() { printf "%s\\n" "$1" >> "$root/events"; }',
                "save_observations() { :; }",
                'xcrun() { printf "simctl:%s\\n" "$*" >> "$root/events"; }',
                "sleep() { command sleep 0.01; }",
                shlex.quote(sys.executable) + ' "$root/driver.py" "$root" ' + shlex.quote(mode) + " &",
                "ui_test=$!",
                'while [ ! -f "$root/driver.pid" ]; do sleep 0.01; done',
                "(exit " + str(result) + ")", "cleanup", "cleanup_result=$?",
                'printf "%s" "$ui_test" > "$root/remaining-driver"',
                'exit "$cleanup_result"',
            ])
            process = subprocess.Popen(
                ["bash", "-c", script], stdout=subprocess.PIPE, stderr=subprocess.PIPE,
                text=True, start_new_session=True,
                env={**os.environ, "SMOKE_TEST_NATIVE_PID": str(native.pid)})
            try:
                stdout, stderr = process.communicate(timeout=8)
                self.assertEqual(process.returncode, result, stderr)
                self.assertIsNone(native.poll(), "cleanup must stop its XCTest child, not the native process")
                self.assertEqual((root / "remaining-driver").read_text(), "")
                diagnostics = root / "diagnostics.json"
                return (root / "events").read_text().splitlines(), (
                    json.loads(diagnostics.read_text()) if diagnostics.exists() else None), stdout
            finally:
                # Keep even a broken regression from leaving its fixture children.
                try:
                    os.killpg(process.pid, signal.SIGKILL)
                except ProcessLookupError:
                    pass
                process.communicate()
                native.terminate()
                native.wait(timeout=2)

    def test_failed_smoke_reaps_driver_before_sampling_and_preserves_failure(self):
        events, diagnostics, stdout = self.exercise_cleanup(37, "terminate")
        self.assertEqual(diagnostics, {"driver_running": False, "native_running": True})
        self.assertLess(events.index("driver-terminated"), events.index("diagnostics"))
        self.assertIn("Owned XCTest launch driver", stdout)

    def test_unresponsive_driver_is_killed_and_reaped_before_sampling(self):
        events, diagnostics, stdout = self.exercise_cleanup(37, "ignore")
        self.assertEqual(diagnostics, {"driver_running": False, "native_running": True})
        self.assertNotIn("driver-terminated", events)
        self.assertIn("exited with status 137", stdout)

    def test_success_preserves_finished_marker_handshake(self):
        events, diagnostics, stdout = self.exercise_cleanup(0, "graceful")
        self.assertIsNone(diagnostics)
        self.assertIn("driver-finished", events)
        self.assertNotIn("driver-terminated", events)
        self.assertIn("exited with status 0", stdout)


if __name__ == "__main__":
    unittest.main()
