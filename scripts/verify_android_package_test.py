#!/usr/bin/env python3
"""Regressions for APK-decoded enum representation and package boundaries."""
import importlib.util
from pathlib import Path
import sys
import unittest
import xml.etree.ElementTree as ET

SCRIPT = Path(__file__).with_name("verify-android-package.py")
sys.dont_write_bytecode = True
SPEC = importlib.util.spec_from_file_location("verify_android_package", SCRIPT)
VERIFY = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(VERIFY)
A = VERIFY.A


def manifest(launcher=False):
    platform = SCRIPT.parent.parent / "crates/phone-android/platform"
    root = ET.parse(platform / "AndroidManifest.xml").getroot()
    package = "org.ferroui.windowsphone" + (".launcher" if launcher else "")
    root.set("package", package)
    ET.SubElement(root, "uses-sdk", {A + "minSdkVersion": "26", A + "targetSdkVersion": "36"})
    root.find("application/provider").set(A + "authorities", package + ".assets")
    if launcher:
        overlay = ET.parse(platform / "launcher/AndroidManifest.xml").getroot()
        for item in overlay.findall("application/activity/intent-filter"):
            root.find("application/activity").append(item)
    return root


class ManifestContract(unittest.TestCase):
    def test_named_decimal_and_hex_encoded_enum_values_are_equivalent(self):
        for launcher in [False, True]:
            for launch_mode in ["singleTask", "2", "0x00000002"]:
                for service_type in ["mediaPlayback", "2", "0x00000002"]:
                    with self.subTest(launcher=launcher, launch=launch_mode, service=service_type):
                        root = manifest(launcher)
                        root.find("application/activity").set(A + "launchMode", launch_mode)
                        root.find("application/service").set(A + "foregroundServiceType", service_type)
                        VERIFY.verify_manifest(root, launcher)

    def test_other_launch_modes_are_rejected(self):
        for value in ["0", "1", "3", "4", "singleTop", "singleInstance", "", "2|8"]:
            with self.subTest(value=value):
                root = manifest()
                root.find("application/activity").set(A + "launchMode", value)
                with self.assertRaisesRegex(ValueError, "single-task"):
                    VERIFY.verify_manifest(root, False)

    def test_extra_service_type_bits_and_exported_services_are_rejected(self):
        for value in ["0", "8", "10", "0x0000000a", "mediaPlayback|location"]:
            with self.subTest(value=value):
                root = manifest()
                root.find("application/service").set(A + "foregroundServiceType", value)
                with self.assertRaisesRegex(ValueError, "media service boundary"):
                    VERIFY.verify_manifest(root, False)
        root = manifest()
        root.find("application/service").set(A + "exported", "true")
        with self.assertRaisesRegex(ValueError, "media service boundary"):
            VERIFY.verify_manifest(root, False)

    def test_home_filter_and_private_asset_provider_remain_required(self):
        root = manifest(True)
        root.set("package", "org.ferroui.windowsphone")
        with self.assertRaisesRegex(ValueError, "HOME role"):
            VERIFY.verify_manifest(root, False)
        root = manifest()
        root.find("application/provider").set(A + "exported", "true")
        with self.assertRaisesRegex(ValueError, "asset provider boundary"):
            VERIFY.verify_manifest(root, False)


if __name__ == "__main__":
    unittest.main()
