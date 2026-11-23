"""Session/configuration boundary tests; no host policy is installed or changed."""
from __future__ import annotations

import importlib.util
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest
import xml.etree.ElementTree as ET

PLATFORM = Path(__file__).resolve().parents[1]


def load(name: str, path: Path):
    spec = importlib.util.spec_from_file_location(name, path)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


linux = load("stage_linux_session", PLATFORM / "linux/stage-session.py")
windows = load("generate_windows_shell", PLATFORM / "windows/generate-shell-config.py")


class ShellConfigurationTests(unittest.TestCase):
    def test_windows_policy_is_per_account_and_keeps_explorer_default(self):
        root = ET.fromstring(windows.configuration(r"C:\Program Files\Phone & UI\phone-desktop.exe", r".\Dedicated Phone"))
        ns = {"s": windows.NS, "v2": windows.V2}
        self.assertEqual(root.find("s:Profiles/s:DefaultProfile/s:Shell", ns).get("Shell"), r"%SystemRoot%\explorer.exe")
        accounts = root.findall("s:Configs/s:Config/s:Account", ns)
        self.assertEqual([account.get("Name") for account in accounts], [r".\Dedicated Phone"])
        self.assertIsNone(root.find(".//s:AutoLogonAccount", ns))
        shell = root.find("s:Profiles/s:Profile/s:Shell", ns)
        self.assertEqual(shell.get("Shell"), '"C:\\Program Files\\Phone & UI\\phone-desktop.exe" --mode shell')
        self.assertEqual(shell.get(f"{{{windows.V2}}}AllAppsFullScreen"), "false")
        self.assertEqual(shell.find("s:DefaultAction", ns).get("Action"), "DoNothing")
        self.assertEqual(shell.find("s:ReturnCodeActions/s:ReturnCodeAction", ns).get("Action"), "RestartShell")

    def test_windows_executable_cannot_be_a_command_or_environment_expansion(self):
        invalid = ["phone-desktop.exe", r"\\server\share\phone.exe", r"C:\..\phone.exe", r"C:\%TEMP%\phone.exe", 'C:\\a.exe" -argument', "C:\\a\nphone.exe", r"C:\phone*.exe", r"C:\alternate:stream\phone.exe", r"C:\NUL\phone.exe", r"C:\Phone\CON.exe"]
        for executable in invalid:
            with self.subTest(executable=executable), self.assertRaises(ValueError):
                windows.configuration(executable, r".\Phone")
        with self.assertRaises(ValueError):
            windows.configuration(r"C:\Phone\phone-desktop.exe", "bad\naccount")

    @unittest.skipIf(os.name == "nt", "POSIX session scripts are exercised on Linux/macOS")
    def test_linux_client_preserves_executable_path_and_closed_argument_set(self):
        with tempfile.TemporaryDirectory(prefix="phone-session-test-") as temporary:
            root = Path(temporary)
            executable = root / "phone app ' quoted $(must-not-run)"
            executable.write_text(f"#!{sys.executable}\nimport json, os, sys\nwith open(os.environ['PHONE_STAGE_TEST_OUTPUT'], 'w') as f: json.dump(sys.argv[1:], f)\n")
            executable.chmod(0o755)
            output = root / "stage"
            linux.stage(executable, output)
            self.assertFalse((root / "must-not-run").exists())
            capture = root / "args.json"
            subprocess.run(["/bin/sh", str(output / "usr/local/libexec/windowsphone/windowsphone-session-client")],
                           env={**os.environ, "PHONE_STAGE_TEST_OUTPUT": str(capture)}, check=True, timeout=5)
            self.assertEqual(json.loads(capture.read_text()), ["--mode", "shell", "--backend", "wayland"])
            for script in ["install.sh", "remove.sh", "usr/local/libexec/windowsphone/windowsphone-session", "usr/local/libexec/windowsphone/windowsphone-session-client"]:
                subprocess.run(["/bin/sh", "-n", str(output / script)], check=True, timeout=5)
            # Generation is exclusive; it cannot overwrite an earlier review set.
            with self.assertRaises(FileExistsError):
                linux.stage(executable, output)
            preview = subprocess.run(["/bin/sh", str(output / "install.sh")], capture_output=True, text=True, check=True, timeout=5)
            self.assertIn("does not change the default session", preview.stdout)

    @unittest.skipIf(os.name == "nt", "POSIX session guard")
    def test_session_entry_refuses_to_replace_a_running_desktop(self):
        result = subprocess.run(["/bin/sh", str(PLATFORM / "linux/windowsphone-session")],
            env={**os.environ, "DISPLAY": ":fixture", "WAYLAND_DISPLAY": ""},
            capture_output=True, text=True, timeout=5)
        self.assertEqual(result.returncode, 1)
        self.assertIn("does not replace a running desktop", result.stderr)

    def test_linux_home_shortcut_targets_existing_window(self):
        root = ET.parse(PLATFORM / "linux/labwc/rc.xml").getroot()
        binding = root.find("keyboard/keybind[@key='W-space']/action")
        self.assertEqual(binding.get("name"), "ForEach")
        self.assertEqual(binding.find("query").get("identifier"), "com.ferroui.WindowsPhone")
        self.assertEqual([action.get("name") for action in binding.findall("then/action")], ["Focus", "Raise"])
        self.assertIsNone(binding.find(".//action[@name='Execute']"))


if __name__ == "__main__":
    unittest.main()
