#!/usr/bin/env python3
"""Check the ownership boundary before a failed simulator process is sampled."""
import importlib.util
import pathlib
import plistlib
import sys
import tempfile
import unittest

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


if __name__ == "__main__":
    unittest.main()
