#!/usr/bin/env python3
"""Check the ownership boundary before a failed simulator process is sampled."""
import importlib.util
import json
import pathlib
import plistlib
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


if __name__ == "__main__":
    unittest.main()
