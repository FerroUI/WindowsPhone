#!/usr/bin/env python3
"""First-boot and role-transition fixtures must not become restoration baselines."""
from pathlib import Path
import sys
import tempfile
import unittest

sys.dont_write_bytecode = True
from verify_android_home import FIELDS, identity, read_sample, verify_stable


def ready():
    return {"user": "0\n", "user-after": "0\n", "device-provisioned": "1\n",
            "user-setup-complete": "1\n", "holders": "example.launcher\n",
            "holders-after": "example.launcher\n", "resolved": "example.launcher/.Home\n",
            "activities": "example.launcher/example.launcher.Home\nexample.other/.Home\n"}


class AndroidHomeReadiness(unittest.TestCase):
    def test_ready_holder_is_derived_without_guessing_a_launcher_package(self):
        self.assertEqual(verify_stable(ready(), ready(), "0"), "example.launcher")

    def test_setup_holder_cannot_pass_before_provisioning_finishes(self):
        for field in ("device-provisioned", "user-setup-complete"):
            for value in ("0", "null", "", "true"):
                proof = ready()
                proof[field] = value
                with self.subTest(field=field, value=value), self.assertRaisesRegex(ValueError, "provisioning"):
                    identity(proof, "0")

    def test_recorded_setup_holder_without_enabled_home_is_rejected(self):
        proof = ready()
        proof.update(holders="com.google.android.googlesdksetup\n",
                     **{"holders-after": "com.google.android.googlesdksetup\n",
                        "resolved": "com.google.android.googlesdksetup/.MainActivity\n"})
        with self.assertRaisesRegex(ValueError, "enabled matching"):
            identity(proof, "0")

    def test_role_and_resolution_must_agree(self):
        proof = ready()
        proof["resolved"] = "example.other/.Home\n"
        with self.assertRaisesRegex(ValueError, "role holder"):
            identity(proof, "0")

    def test_holder_changes_within_or_between_samples_are_rejected(self):
        proof = ready()
        proof["holders-after"] = "example.other\n"
        with self.assertRaisesRegex(ValueError, "during sampling"):
            identity(proof, "0")
        proof = ready()
        proof.update(holders="example.other\n", **{"holders-after": "example.other\n",
                     "resolved": "example.other/.Home\n"})
        with self.assertRaisesRegex(ValueError, "still changing"):
            verify_stable(ready(), proof, "0")

    def test_invalid_intervening_sample_requires_a_new_valid_pair(self):
        pending = ready()
        pending["user-setup-complete"] = "0\n"
        for before, after in ((ready(), pending), (pending, ready())):
            with self.assertRaises(ValueError):
                verify_stable(before, after, "0")
        self.assertEqual(verify_stable(ready(), ready(), "0"), "example.launcher")

    def test_ambiguous_missing_or_malformed_queries_cannot_pass(self):
        for field, value in (("holders", ""), ("holders", "example.launcher\nexample.other\n"),
                             ("resolved", "No activity found\n"), ("activities", "No activities found\n"),
                             ("activities", "example.launcher/.Home\nError: unavailable\n")):
            proof = ready()
            proof[field] = value
            with self.subTest(field=field), self.assertRaises(ValueError):
                identity(proof, "0")

    def test_current_user_change_is_rejected(self):
        for field in ("user", "user-after"):
            proof = ready()
            proof[field] = "10\n"
            with self.assertRaisesRegex(ValueError, "user changed"):
                identity(proof, "0")

    def test_failed_adb_read_cannot_pass_with_plausible_stdout(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            for field in FIELDS:
                (root / (field + ".txt")).write_text(ready()[field])
                (root / (field + ".status")).write_text("0\n")
            self.assertEqual(identity(read_sample(root), "0")[1], "example.launcher")
            (root / "holders.status").write_text("1\n")
            with self.assertRaisesRegex(ValueError, "query failed"):
                read_sample(root)


if __name__ == "__main__":
    unittest.main()
