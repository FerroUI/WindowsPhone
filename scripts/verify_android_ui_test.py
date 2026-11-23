#!/usr/bin/env python3
"""Negative contracts ensure a successful activation alone cannot pass the UI gate."""
import copy
import sys
import unittest

sys.dont_write_bytecode = True
from verify_android_ui import verify

NONCE = "ab" * 16
HOME = "ferroui-phone://home"
WALLET = "ferroui-phone://app/org.ferroui.wallet"


def mounted():
    return {
        "schema": 1, "platform": "android", "nonce": NONCE, "sequence": 5,
        "process_id": 123, "observed_at_unix_ms": 1000, "ready": True, "closed": False,
        "host": {"mode": "app", "launcher": "available", "shell": "unsupported"},
        "activation": {"uri": HOME, "outcome": "accepted"}, "route": "Start",
        "mounted_controls": ["PhoneScreen", "PhonePage", "StartTiles"],
        "client": {"width": 360, "height": 720}, "render_scaling": 2,
        "safe_area": {"left": 0, "top": 24, "right": 0, "bottom": 24},
        "wrapper": {"x": 0, "y": 0, "width": 360, "height": 720},
        "inner": {"x": 0, "y": 24, "width": 360, "height": 672},
        "screen": {"x": 0, "y": 24, "width": 360, "height": 672},
        "viewport": {"width": 360, "height": 672},
    }


class AndroidUiContract(unittest.TestCase):
    def test_selected_home_requires_both_actual_role_observations(self):
        proof = mounted()
        with self.assertRaises(ValueError):
            verify(proof, NONCE, HOME, 4, home_active=True)
        proof["host"].update(mode="launcher", launcher="active")
        proof["device_snapshot"] = {"roles": [{"role": "home", "status": "available"}]}
        with self.assertRaisesRegex(ValueError, "independently"):
            verify(proof, NONCE, HOME, 4, home_active=True)
        proof["device_snapshot"]["roles"][0]["status"] = "active"
        self.assertEqual(verify(proof, NONCE, HOME, 4, home_active=True), 5)
        with self.assertRaises(ValueError):
            verify(proof, NONCE, HOME, 4)

    def test_active_home_back_at_start_keeps_real_start_controls(self):
        proof = mounted()
        proof["host"].update(mode="launcher", launcher="active")
        proof["device_snapshot"] = {"roles": [{"role": "home", "status": "active"}]}
        self.assertEqual(verify(proof, NONCE, "back:" + HOME, 4, home_active=True), 5)
        proof["route"] = {"Extension": {"app": "org.ferroui.wallet", "path": ""}}
        with self.assertRaisesRegex(ValueError, "Start"):
            verify(proof, NONCE, "back:" + HOME, 4, home_active=True)

    def test_real_home_and_wallet_controls_with_safe_geometry_pass(self):
        proof = mounted()
        self.assertEqual(verify(proof, NONCE, HOME, 4), 5)
        proof.update(route={"Extension": {"app": "org.ferroui.wallet", "path": ""}},
                     activation={"uri": WALLET, "outcome": "accepted"},
                     mounted_controls=["PhoneScreen", "PhonePage", "WalletSearch"])
        self.assertEqual(verify(proof, NONCE, WALLET, 4), 5)

    def test_back_requires_start_after_the_original_wallet_activation(self):
        proof = mounted()
        proof["activation"]["uri"] = WALLET
        self.assertEqual(verify(proof, NONCE, "back:" + WALLET, 4), 5)
        proof["route"] = {"Extension": {"app": "org.ferroui.wallet", "path": ""}}
        with self.assertRaisesRegex(ValueError, "Start"):
            verify(proof, NONCE, "back:" + WALLET, 4)

    def test_previous_outer_bounds_inset_bug_is_rejected(self):
        proof = mounted()
        proof["screen"]["height"] = 720
        proof["viewport"]["height"] = 720
        with self.assertRaisesRegex(ValueError, "PhoneScreen"):
            verify(proof, NONCE, HOME, 4)

    def test_receiving_a_uri_without_rendering_its_route_cannot_pass(self):
        proof = mounted()
        proof["activation"]["uri"] = WALLET
        with self.assertRaisesRegex(ValueError, "route"):
            verify(proof, NONCE, WALLET, 4)
        proof["route"] = {"Extension": {"app": "org.ferroui.wallet", "path": ""}}
        with self.assertRaisesRegex(ValueError, "controls"):
            verify(proof, NONCE, WALLET, 4)

    def test_stale_or_foreign_process_observations_cannot_pass(self):
        with self.assertRaisesRegex(ValueError, "stale"):
            verify(mounted(), NONCE, HOME, 5)
        with self.assertRaisesRegex(ValueError, "another smoke"):
            verify(mounted(), "cd" * 16, HOME, 4)

    def test_zero_insets_nonfinite_geometry_and_home_selection_are_rejected(self):
        variants = []
        proof = mounted()
        proof["safe_area"] = dict.fromkeys(("left", "top", "right", "bottom"), 0)
        variants.append(proof)
        proof = mounted()
        proof["inner"]["height"] = float("nan")
        variants.append(proof)
        proof = mounted()
        proof["host"]["launcher"] = "active"
        variants.append(proof)
        for proof in variants:
            with self.subTest(proof=proof), self.assertRaises(ValueError):
                verify(copy.deepcopy(proof), NONCE, HOME, 4)


if __name__ == "__main__":
    unittest.main()
