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
    for provider in root.findall("application/provider"):
        provider.set(A + "authorities", provider.get(A + "authorities").replace("${applicationId}", package))
    if launcher:
        overlay = ET.parse(platform / "launcher/AndroidManifest.xml").getroot()
        for item in overlay.findall("application/activity/intent-filter"):
            root.find("application/activity").append(item)
    return root


class ManifestContract(unittest.TestCase):
    def test_call_service_requires_only_the_phone_call_type_and_permission(self):
        for value in ["phoneCall", "4", "0x00000004"]:
            root = manifest()
            call = next(v for v in root.findall("application/service")
                        if v.get(A + "name") == "org.ferroui.phone.PhoneInCallService")
            call.set(A + "foregroundServiceType", value)
            VERIFY.verify_manifest(root, False)
        for value in ["", "0", "2", "6", "phoneCall|microphone"]:
            root = manifest()
            call = next(v for v in root.findall("application/service")
                        if v.get(A + "name") == "org.ferroui.phone.PhoneInCallService")
            call.set(A + "foregroundServiceType", value)
            with self.subTest(value=value), self.assertRaisesRegex(ValueError, "phone-call foreground service type"):
                VERIFY.verify_manifest(root, False)

    def test_call_service_cannot_omit_its_foreground_permission(self):
        root = manifest()
        permission = next(v for v in root.findall("uses-permission")
                          if v.get(A + "name") == "android.permission.FOREGROUND_SERVICE_PHONE_CALL")
        root.remove(permission)
        with self.assertRaisesRegex(ValueError, "Phone-call foreground service permission"):
            VERIFY.verify_manifest(root, False)

    def test_named_decimal_and_hex_encoded_enum_values_are_equivalent(self):
        for launcher in [False, True]:
            for launch_mode in ["singleTask", "2", "0x00000002"]:
                for service_type in ["mediaPlayback", "2", "0x00000002"]:
                    with self.subTest(launcher=launcher, launch=launch_mode, service=service_type):
                        root = manifest(launcher)
                        root.find("application/activity").set(A + "launchMode", launch_mode)
                        root.find("application/service").set(A + "foregroundServiceType", service_type)
                        VERIFY.verify_manifest(root, launcher)

    def test_carrier_services_and_receivers_require_their_system_permissions(self):
        for name in ["PhoneInCallService", "PhoneNotificationListener", "PhoneRespondViaMessageService", "PhoneMessageJobService", "PhoneSmsReceiver", "PhoneMmsReceiver"]:
            root = manifest()
            component = next(value for value in root.find("application") if value.get(A + "name") == "org.ferroui.phone." + name)
            component.attrib.pop(A + "permission")
            with self.subTest(name=name), self.assertRaisesRegex(ValueError, "protected native component"):
                VERIFY.verify_manifest(root, False)

    def test_message_transfer_and_callback_components_cannot_be_exported(self):
        for name in ["PhoneCallActivity", "PhoneCallActionReceiver", "PhoneSmsResultReceiver", "PhoneMmsResultReceiver", "PhoneMessageFileProvider"]:
            root = manifest()
            component = next(value for value in root.find("application") if value.get(A + "name") == "org.ferroui.phone." + name)
            component.set(A + "exported", "true")
            with self.subTest(name=name), self.assertRaises(ValueError):
                VERIFY.verify_manifest(root, False)

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
