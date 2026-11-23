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
        "route": "Start", "activation": None,
        "client": {"width": 414, "height": 896},
        "safe_area": {"left": 0, "top": 48, "right": 0, "bottom": 34},
        "inner": {"x": 0, "y": 48, "width": 414, "height": 814},
        "screen": {"x": 0, "y": 48, "width": 414, "height": 814},
        "mounted_controls": ["PhoneScreen", "PhonePage", "StartTiles"],
    }


class UIKitProofTests(unittest.TestCase):
    def test_requires_fresh_accepted_uri_and_mounted_wallet_controls(self):
        data = observation()
        module.verify(data, "home", 0)
        uri = "ferroui-phone://app/org.ferroui.wallet"
        data["route"] = {"Extension": {"app": "org.ferroui.wallet", "path": ""}}
        data["activation"] = {"uri": uri, "outcome": "accepted"}
        with self.assertRaises(ValueError):
            module.verify(data, "org.ferroui.wallet", 3, uri)
        data["mounted_controls"][-1] = "WalletSearch"
        module.verify(data, "org.ferroui.wallet", 3, uri)
        with self.assertRaises(ValueError):
            module.verify(data, "org.ferroui.wallet", 4, uri)
        data["activation"]["outcome"] = "received"
        with self.assertRaises(ValueError):
            module.verify(data, "org.ferroui.wallet", 3, uri)

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


if __name__ == "__main__":
    unittest.main()
