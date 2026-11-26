#!/usr/bin/env python3
"""Regress the observable boundaries that distinguish mounted UI from VM state."""
import copy
import importlib.util
import pathlib
import sys
import unittest

sys.dont_write_bytecode = True
spec = importlib.util.spec_from_file_location("ios_smoke", pathlib.Path(__file__).with_name("verify-ios-smoke.py"))
module = importlib.util.module_from_spec(spec)
spec.loader.exec_module(module)


def observation():
    return {
        "schema": 1, "platform": "ios", "ready": True, "closed": False, "sequence": 4,
        "observed_at_unix_ms": 4000,
        "native": {"pid": 123, "launch_id": "43BA0793-1276-4B2D-9819-549201309501",
                   "bundle_id": "org.ferroui.windowsphone", "application_state": 0,
                   "scenes": [{"activation_state": 0, "delegate_class": "FerroSceneDelegate", "session_id": "original-scene"}]},
        "route": "Start", "activation": None,
        "client": {"width": 414, "height": 896},
        "safe_area": {"left": 0, "top": 48, "right": 0, "bottom": 34},
        "inner": {"x": 0, "y": 48, "width": 414, "height": 814},
        "screen": {"x": 0, "y": 48, "width": 414, "height": 814},
        "mounted_controls": ["PhoneScreen", "PhonePage", "StartTiles"],
        "navigation_controls": [{
            "name": name, "visible": True, "enabled": True, "hit": True, "opacity": 1,
            "bounds": {"x": 138 * index, "y": 808, "width": 138, "height": 54},
            "glyph": {"visible": True, "opacity": 1, "has_geometry": True,
                      "bounds": {"x": 138 * index + 57, "y": 823, "width": 24, "height": 24}},
        } for index, name in enumerate(("HardwareBack", "HardwareStart", "HardwareSearch"))],
    }


def baseline():
    return {**observation(), "sequence": 1, "observed_at_unix_ms": 1000}


class UIKitProofTests(unittest.TestCase):
    def test_navigation_rejects_hidden_disabled_unhittable_or_blank_controls(self):
        module.verify(observation(), "home", 0)
        for field, value in (("visible", False), ("enabled", False), ("hit", False),
                             ("opacity", 0), ("opacity", float("nan")), ("glyph", None)):
            data = observation()
            data["navigation_controls"][1][field] = value
            with self.subTest(field=field), self.assertRaisesRegex(ValueError, "HardwareStart"):
                module.verify(data, "home", 0)
        for field, value in (("visible", False), ("has_geometry", False), ("opacity", 0)):
            data = observation()
            data["navigation_controls"][2]["glyph"][field] = value
            with self.subTest(glyph=field), self.assertRaisesRegex(ValueError, "HardwareSearch"):
                module.verify(data, "home", 0)

    def test_navigation_rejects_duplicate_controls_and_clipped_or_unusable_cells(self):
        data = observation()
        data["navigation_controls"][2]["name"] = "HardwareBack"
        with self.assertRaisesRegex(ValueError, "exactly once"):
            module.verify(data, "home", 0)
        for changes in ({"x": 180}, {"y": 860}, {"width": 1}, {"height": 0}):
            data = observation()
            data["navigation_controls"][1]["bounds"].update(changes)
            with self.subTest(changes=changes), self.assertRaisesRegex(ValueError, "navigation cell"):
                module.verify(data, "home", 0)
        data = observation()
        data["navigation_controls"][0]["glyph"]["bounds"]["x"] = 200
        with self.assertRaisesRegex(ValueError, "inside"):
            module.verify(data, "home", 0)

    def test_native_navigation_preserves_the_active_scene_not_only_the_process(self):
        data = observation()
        data["native"]["scenes"][0]["session_id"] = "replacement-scene"
        with self.assertRaisesRegex(ValueError, "scene session"):
            module.verify(data, "home", 0, baseline=baseline())

    def test_real_navigation_requires_tap_receipt_fresh_route_and_visible_button_ink(self):
        original = baseline()
        original["activation"] = {"uri": "ferroui-phone://home", "outcome": "accepted"}
        data = observation()
        data["activation"] = copy.deepcopy(original["activation"])
        data["route"] = "Search"
        data["mounted_controls"][-1] = "CortanaInput"
        request = {"sequence": 1, "control": "HardwareSearch", "after_sequence": 3, "issued_at_unix_ms": 3000}
        driver = {"navigation_driver": "XCUIElement.tap", "navigation": [{
            "sequence": 1, "control": "HardwareSearch", "tap_completed_at_unix_ms": 3500, "bright_opaque_pixels": 200}]}
        module.verify_navigation(data, "search", request, driver, original)
        for changed in ({**driver, "navigation_driver": "injected"}, {**driver, "navigation": []}):
            with self.subTest(driver=changed), self.assertRaisesRegex(ValueError, "real navigation tap"):
                module.verify_navigation(data, "search", request, changed, original)
        for changes in ({"bright_opaque_pixels": 0}, {"tap_completed_at_unix_ms": 2000}, {"control": "HardwareStart"}):
            changed = copy.deepcopy(driver)
            changed["navigation"][0].update(changes)
            with self.subTest(changes=changes), self.assertRaises(ValueError):
                module.verify_navigation(data, "search", request, changed, original)
        for changes in ({"observed_at_unix_ms": 3000}, {"activation": None}, {"sequence": 3}, {"route": "Start"}):
            with self.subTest(changes=changes), self.assertRaises(ValueError):
                module.verify_navigation({**data, **changes}, "search", request, driver, original)

    def test_requires_fresh_accepted_uri_and_mounted_wallet_controls(self):
        data = observation()
        module.verify(data, "home", 0)
        uri = "ferroui-phone://app/org.ferroui.wallet"
        data["route"] = {"Extension": {"app": "org.ferroui.wallet", "path": ""}}
        data["activation"] = {"uri": uri, "outcome": "accepted"}
        with self.assertRaises(ValueError):
            module.verify(data, "org.ferroui.wallet", 3, uri, baseline())
        data["mounted_controls"][-1] = "WalletSearch"
        module.verify(data, "org.ferroui.wallet", 3, uri, baseline())
        with self.assertRaises(ValueError):
            module.verify(data, "org.ferroui.wallet", 4, uri, baseline())
        data["activation"]["outcome"] = "received"
        with self.assertRaises(ValueError):
            module.verify(data, "org.ferroui.wallet", 3, uri, baseline())

    def test_warm_activation_rejects_another_process_or_restarted_launch(self):
        original = baseline()
        data = observation()
        uri = "ferroui-phone://home"
        data["activation"] = {"uri": uri, "outcome": "accepted"}
        module.verify(data, "home", 3, uri, original)
        for changes in ({"pid": 124}, {"launch_id": "299FD601-E24F-484F-BB03-6B53BAF8D653"},
                        {"bundle_id": "org.other.application"}):
            changed = copy.deepcopy(data)
            changed["native"].update(changes)
            with self.subTest(changes=changes), self.assertRaises(ValueError):
                module.verify(changed, "home", 3, uri, original)
        with self.assertRaisesRegex(ValueError, "original Start"):
            module.verify(data, "home", 3, uri)
        data["observed_at_unix_ms"] = original["observed_at_unix_ms"]
        with self.assertRaisesRegex(ValueError, "fresh"):
            module.verify(data, "home", 3, uri, original)

    def test_rejects_a_background_application_or_inactive_scene(self):
        for changes in ({"application_state": 2}, {"application_state": 1},
                        {"scenes": []}, {"scenes": [{"activation_state": 1}]}):
            data = observation()
            data["native"].update(changes)
            with self.subTest(changes=changes), self.assertRaisesRegex(ValueError, "foreground-active"):
                module.verify(data, "home", 0)

    def test_rejects_original_screen_overlapping_uikit_insets(self):
        data = observation()
        data["screen"] = {"x": 0, "y": 0, "width": 414, "height": 896}
        with self.assertRaisesRegex(ValueError, "safe area"):
            module.verify(data, "home", 0)
        data["screen"] = copy.deepcopy(data["inner"])
        data["screen"]["y"] -= 1
        module.verify(data, "home", 0)
        data["screen"]["y"] -= 0.1
        with self.assertRaises(ValueError):
            module.verify(data, "home", 0)

    def test_rejects_closed_unarranged_or_nonfinite_observations(self):
        for changes in ({"closed": True}, {"ready": False}, {"platform": "browser"}):
            with self.subTest(changes=changes), self.assertRaises(ValueError):
                module.verify({**observation(), **changes}, "home", 0)
        data = observation()
        data["inner"]["height"] = float("nan")
        with self.assertRaises(ValueError):
            module.verify(data, "home", 0)

    def test_builtin_phone_requires_its_native_dial_uri_and_populated_control(self):
        data = observation()
        uri = "ferroui-phone://app/phone.phone?path=dial/KzEgMjAyIDU1NSAwMTQ3"
        data["route"] = {"App": "Phone"}
        data["activation"] = {"uri": uri, "outcome": "accepted"}
        data["mounted_controls"][-1] = "DevicePhoneNumber"
        data["device_fields"] = {"DevicePhoneNumber": "+1 202 555 0147"}
        module.verify(data, "phone.phone", 3, uri, baseline())
        data["device_fields"]["DevicePhoneNumber"] = ""
        with self.assertRaisesRegex(ValueError, "dial"):
            module.verify(data, "phone.phone", 3, uri, baseline())

    def test_provider_proof_requires_cleanup_and_no_outgoing_communication(self):
        spec = importlib.util.spec_from_file_location("ios_devices", pathlib.Path(__file__).with_name("verify-ios-devices.py"))
        devices = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(devices)
        data = {"schema": 1, "platform": "ios", "status": "passed", "simulator_only": True,
                "error": None, "cleanup_errors": [], "sent_calls": 0, "sent_messages": 0,
                "checks": [{"name": name, "passed": True} for name in (
                    "observed_simulator_capabilities", "brightness_read_only", "contact_create_and_query",
                    "contact_update_and_query", "contact_delete_and_query")]}
        data["checks"].append({"name": "location_authorization_observed", "passed": True, "status": "prompt"})
        data["checks"].insert(0, {"name": "network_path_observed", "passed": True, "online": False})
        devices.verify(data)
        data["checks"][0]["online"] = True
        devices.verify(data)
        for value in (None, "true", "false", 0, 1):
            changed = copy.deepcopy(data)
            changed["checks"][0]["online"] = value
            with self.subTest(network=value), self.assertRaisesRegex(ValueError, "network path"):
                devices.verify(changed)
        for status in (None, "unknown", "unsupported", "granted", "denied", "restricted"):
            changed = copy.deepcopy(data)
            changed["checks"][-1]["status"] = status
            with self.subTest(status=status), self.assertRaisesRegex(ValueError, "unrequested location"):
                devices.verify(changed)
        data["sent_messages"] = 1
        with self.assertRaises(ValueError):
            devices.verify(data)
        data["sent_messages"] = 0
        data["checks"].pop()
        with self.assertRaises(ValueError):
            devices.verify(data)


if __name__ == "__main__":
    unittest.main()
