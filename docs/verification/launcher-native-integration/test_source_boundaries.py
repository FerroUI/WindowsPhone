#!/usr/bin/env python3
"""Focused evidence/projection regressions; these do not qualify any platform.

Retained successful records supply realistic immutable fixture shapes. Pending
mobile phase additions and temporary Git repositories below are explicitly
synthetic test data, never written into the release's verification records.
Run: python -B -m unittest discover -s docs/verification/launcher-native-integration -p 'test_source_boundaries.py' -v
"""
from __future__ import annotations

import argparse
import copy
import importlib.util
import json
from pathlib import Path
import subprocess
import tempfile
import unittest
from unittest.mock import patch

HERE = Path(__file__).resolve().parent
SPEC = importlib.util.spec_from_file_location("source_boundaries", HERE / "record_launcher_source_boundaries.py")
helper = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(helper)
SHA = "a" * 40
HASH = "b" * 64


def retained(name):
    return json.loads((HERE / name).read_bytes())


def ios_fixture():
    """Synthetic agreed success schema, not an assertion that iOS passed."""
    return {
        "repository": helper.REPOSITORY, "head_sha": SHA, "run_id": 1,
        "job_id": 2, "conclusion": "success", "workflow": "Verify iOS host",
        "steps": [{"name": name, "conclusion": "success"} for name in helper.IOS_STEPS],
        "gates": {name: "passed" for name in helper.IOS_GATES},
        "runtime": {
            "initial_mounted_start": {"passed": True},
            "provider_roundtrips": {"passed": True, "checks": [
                {"name": name, "passed": True} for name in helper.IOS_PROVIDER_CHECKS]},
            "warm_activations": {"passed": True, "count": 6, "routes": [
                {"uri": uri, "outcome": "accepted"} for uri in helper.IOS_URIS]},
            "invariants": {name: True for name in helper.IOS_INVARIANTS},
        },
        "artifacts": [{"id": index, "name": name, "sha256": HASH,
                       "workflow_run": {"id": 1, "head_sha": SHA}}
                      for index, name in enumerate(("ios-verification", "phone-ios-simulator"), 3)],
    }


def android_ui_fixture():
    """Retained old shape plus synthetic new bootstrap phases, for validators."""
    record = retained("history/android-ui-83e0c00.json")
    record["steps"] = [{"name": name, "conclusion": "success"} for name in helper.ANDROID_UI_STEPS]
    record["runtime"]["cold_start_unread"] = {
        "result": "passed", "route": "Start", "selected_thread": None,
        "native_thread_id": "fixture-thread", "unread": 1,
        "badge": {"tile_id": 1, "text": "1", "fully_visible": True},
        "observation": "fixture-cold.json", "capture": "fixture-cold.png",
    }
    record["runtime"]["notification_bootstrap"] = {
        "result": "passed", "component": "fixture/.Listener",
        "before": ["other/.Listener"],
        "granted": ["other/.Listener", "fixture/.Listener"],
        "revoked": ["other/.Listener"], "restored": ["other/.Listener"],
        "grant_exact": True, "restored_exact": True,
        "admitted": {"observation": "fixture-admitted.json", "key": "fixture-key",
                     "package_id": "com.android.shell", "title": "Fixture title", "body": "Fixture body"},
        "rendered": {"observation": "fixture-rendered.json", "capture": "fixture-rendered.png", "title_body_visible": True},
        "revocation": {"observation": "fixture-revoked.json", "capture": "fixture-revoked.png",
                       "read_operation": False, "store_count": 0, "fixture_labels": []},
    }
    return record


def scopes(**overrides):
    options = dict(browser=SHA, desktop=SHA, android_package=SHA, android_ui=SHA, ios=SHA,
                   browser_record="browser.json", desktop_record="desktop.json",
                   android_package_record="android-package.json", android_ui_record="android-ui.json",
                   ios_record="ios.json")
    options.update(overrides)
    return helper.scope_table(argparse.Namespace(**options))


class RecordValidationTests(unittest.TestCase):
    def validate(self, record, target):
        return helper.validate_record(record, record["head_sha"], target, "fixture")

    def test_retained_full_browser_desktop_and_arm64_records(self):
        for name, target in (("browser.json", "browser"),
                             ("history/desktop-b798631.json", "desktop"),
                             ("android-package.json", "android_arm64_package")):
            with self.subTest(target=target):
                self.validate(retained(name), target)

    def test_same_head_swapped_record_cannot_supply_another_target(self):
        record = retained("browser.json")
        for target in ("desktop", "android_arm64_package", "android_x86_64_ui", "ios"):
            with self.subTest(target=target), self.assertRaises(helper.BoundaryError):
                self.validate(record, target)

    def test_only_two_native_targets_cannot_pass(self):
        record = retained("history/desktop-b798631.json")
        record["jobs"] = record["jobs"][:2]
        with self.assertRaisesRegex(helper.BoundaryError, "all three"):
            self.validate(record, "desktop")

    def test_duplicate_native_target_cannot_replace_macos(self):
        record = retained("history/desktop-b798631.json")
        record["jobs"][2] = copy.deepcopy(record["jobs"][0])
        with self.assertRaisesRegex(helper.BoundaryError, "all three"):
            self.validate(record, "desktop")

    def test_desktop_skipped_link_or_missing_protocol_is_rejected(self):
        for defect in ("link", "protocol"):
            record = retained("history/desktop-b798631.json")
            if defect == "link":
                for row in record["jobs"][0]["gates"]:
                    if row["name"] == "Build and link the complete native host":
                        row["conclusion"] = "skipped"
            else:
                record["jobs"][0]["tests"] = [row for row in record["jobs"][0]["tests"]
                                               if "dbus-run-session" not in row["command"]]
            with self.subTest(defect=defect), self.assertRaises(helper.BoundaryError):
                self.validate(record, "desktop")

    def test_nested_pages_deploy_must_have_executed_successfully(self):
        for outcome in ("failure", "skipped", "cancelled"):
            record = retained("browser.json")
            record["publication"]["job"]["conclusion"] = outcome
            with self.subTest(outcome=outcome), self.assertRaises(helper.BoundaryError):
                self.validate(record, "browser")

    def test_browser_rendered_step_cannot_be_skipped_in_green_job(self):
        record = retained("browser.json")
        for step in record["job"]["steps"]:
            if step["name"] == "Verify rendered desktop and touch views":
                step["conclusion"] = "skipped"
        with self.assertRaisesRegex(helper.BoundaryError, "did not execute"):
            self.validate(record, "browser")

    def test_artifact_wrong_head_or_run_is_rejected(self):
        for field in ("head_sha", "id"):
            record = retained("android-package.json")
            record["artifacts"][0]["workflow_run"][field] = SHA if field == "head_sha" else 123
            with self.subTest(field=field), self.assertRaisesRegex(helper.BoundaryError, "artifact workflow"):
                self.validate(record, "android_arm64_package")

    def test_browser_normalized_artifacts_still_require_exact_run_links(self):
        for defect in ("missing", "wrong_run", "wrong_artifact"):
            record = retained("browser.json")
            artifact = record["artifacts"][0]
            if defect == "missing":
                artifact.pop("url")
            elif defect == "wrong_run":
                artifact["url"] = artifact["url"].replace(str(record["run_id"]), "123")
            else:
                artifact["url"] = artifact["url"].replace(str(artifact["id"]), "123")
            with self.subTest(defect=defect), self.assertRaises(helper.BoundaryError):
                self.validate(record, "browser")

    def test_source_commit_alias_cannot_disagree(self):
        for field, value in (("source_commit", SHA), ("source", SHA), ("source", {"commit": SHA})):
            record = retained("browser.json")
            record[field] = value
            with self.subTest(field=field), self.assertRaisesRegex(helper.BoundaryError, "conflicting"):
                self.validate(record, "browser")

    def test_arm_package_requires_both_abis_and_native_alignment(self):
        for defect in ("missing", "abi", "alignment"):
            record = retained("android-package.json")
            packages = record["artifact_verification"]["packages"]
            if defect == "missing":
                packages.pop()
            elif defect == "abi":
                packages[0]["abi"] = "x86_64"
            else:
                packages[0]["load_alignment"]["lib/arm64-v8a/libphone_android.so"] = [4096]
            with self.subTest(defect=defect), self.assertRaises(helper.BoundaryError):
                self.validate(record, "android_arm64_package")

    def test_synthetic_mobile_schemas_are_admitted_without_claiming_actual_success(self):
        self.validate(android_ui_fixture(), "android_x86_64_ui")
        self.validate(ios_fixture(), "ios")

    def test_android_old_success_lacks_new_cold_start_and_notification_proof(self):
        record = retained("history/android-ui-83e0c00.json")
        record["steps"] = [{"name": name, "conclusion": "success"} for name in helper.ANDROID_UI_STEPS]
        with self.assertRaisesRegex(helper.BoundaryError, "cold_start_unread"):
            self.validate(record, "android_x86_64_ui")

    def test_android_successful_job_does_not_cover_skipped_runtime_step(self):
        record = android_ui_fixture()
        for row in record["steps"]:
            if row["name"].startswith("Verify native geometry"):
                row["conclusion"] = "skipped"
        with self.assertRaisesRegex(helper.BoundaryError, "did not execute"):
            self.validate(record, "android_x86_64_ui")

    def test_android_cold_start_requires_actual_visible_unread_badge(self):
        for key, value in (("fully_visible", False), ("text", "0")):
            record = android_ui_fixture()
            record["runtime"]["cold_start_unread"]["badge"][key] = value
            with self.subTest(key=key), self.assertRaisesRegex(helper.BoundaryError, "cold Start"):
                self.validate(record, "android_x86_64_ui")

    def test_android_revocation_restoration_and_owned_sets_are_checked(self):
        for defect in ("private_rows", "unrelated_change", "restoration"):
            record = android_ui_fixture()
            notice = record["runtime"]["notification_bootstrap"]
            if defect == "private_rows":
                notice["revocation"]["store_count"] = 1
            elif defect == "unrelated_change":
                notice["granted"].append("unrelated/.Listener")
            else:
                notice["restored"] = []
            with self.subTest(defect=defect), self.assertRaises(helper.BoundaryError):
                self.validate(record, "android_x86_64_ui")

    def test_android_original_off_does_not_require_fictional_regrant(self):
        record = android_ui_fixture()
        self.assertNotIn("regrant", record["runtime"]["notification_bootstrap"])
        self.validate(record, "android_x86_64_ui")

    def test_android_partial_mounts_call_foreground_and_role_restoration_fail(self):
        for defect in ("mount", "foreground", "role"):
            record = android_ui_fixture()
            if defect == "mount":
                record["mounted_observations"].pop()
            elif defect == "foreground":
                record["native_call_observations"]["active"]["call_presentation"]["service_type"] = 0
            else:
                record["runtime"]["roles_restored"]["sms"]["restored"] = "wrong.owner"
            with self.subTest(defect=defect), self.assertRaises(helper.BoundaryError):
                self.validate(record, "android_x86_64_ui")

    def test_ios_each_build_provider_and_url_phase_is_required(self):
        for defect in ("standard", "optional", "provider", "route", "order", "outcome", "foreground"):
            record = ios_fixture()
            if defect in {"standard", "optional"}:
                record["gates"][defect + ("_uikit" if defect == "standard" else "_carrier") + "_full_build_link_bundle"] = "skipped"
            elif defect == "provider":
                record["runtime"]["provider_roundtrips"]["checks"].pop()
            elif defect == "route":
                record["runtime"]["warm_activations"]["routes"].pop()
            elif defect == "order":
                record["runtime"]["warm_activations"]["routes"].reverse()
            elif defect == "outcome":
                record["runtime"]["warm_activations"]["routes"][0]["outcome"] = "pending"
            else:
                record["runtime"]["invariants"]["foreground_active"] = False
            with self.subTest(defect=defect), self.assertRaises(helper.BoundaryError):
                self.validate(record, "ios")

    def test_ios_successful_job_does_not_cover_skipped_runtime_step(self):
        record = ios_fixture()
        for row in record["steps"]:
            if row["name"] == "Verify native UI, device roundtrips and activation":
                row["conclusion"] = "skipped"
        with self.assertRaisesRegex(helper.BoundaryError, "did not execute"):
            self.validate(record, "ios")

    def test_json_duplicate_keys_and_nonfinite_numbers_fail_closed(self):
        for raw in (b'{"head_sha":"a","head_sha":"b"}', b'{"a":NaN}', b'{"a":1e400}'):
            with self.subTest(raw=raw), self.assertRaises(helper.BoundaryError):
                helper.strict_json(raw, "fixture")


class WorkflowTests(unittest.TestCase):
    def project(self, text, jobs=None):
        with patch.object(helper, "git", return_value=text.encode()):
            return helper.workflow_projection(Path("."), SHA, ".github/workflows/test.yml", jobs or ["build"])

    def sample(self, scalar):
        return f"on:\n  workflow_dispatch:\n    inputs:\n      flag:\n        default: {scalar}\njobs:\n  build:\n    runs-on: ubuntu-latest\n    steps:\n      - run: echo ok\n"

    def test_plain_and_quoted_null_bool_and_numeric_scalars_are_distinct(self):
        for plain, quoted in (("null", '"null"'), ("false", '"false"'), ("1", '"1"'), ("1.0", "1"), ("!!str null", "null")):
            with self.subTest(plain=plain, quoted=quoted):
                self.assertNotEqual(self.project(self.sample(plain))[0], self.project(self.sample(quoted))[0])

    def test_on_dispatch_inputs_are_retained_without_yaml11_boolean_key(self):
        first, detail = self.project(self.sample("true"))
        second, _ = self.project(self.sample("false"))
        self.assertNotEqual(first, second)
        self.assertIn("workflow_dispatch", json.dumps(first))
        self.assertEqual(detail["normalization"], helper.WORKFLOW_NORMALIZATION)

    def test_scalar_mapping_key_style_is_preserved(self):
        first = 'jobs:\n  build:\n    env:\n      false: value\n'
        second = first.replace("false:", '"false":')
        self.assertNotEqual(self.project(first)[0], self.project(second)[0])

    def test_comments_mapping_order_and_event_filters_are_nonexecution(self):
        first = 'on: push\nname: test\njobs:\n  build:\n    runs-on: ubuntu-latest\n    steps: [{run: "echo ok"}]\n'
        second = '# comment\nname: test\non: pull_request\njobs:\n  build:\n    steps: [{run: "echo ok"}]\n    runs-on: ubuntu-latest\n'
        self.assertEqual(self.project(first)[0], self.project(second)[0])

    def test_job_admission_is_omitted_but_step_if_is_material(self):
        first = 'jobs:\n  build:\n    if: true\n    steps:\n      - if: false\n        run: echo ok\n'
        admission = first.replace("    if: true", "    if: false")
        execution = first.replace("      - if: false", "      - if: true")
        self.assertEqual(self.project(first)[0], self.project(admission)[0])
        self.assertNotEqual(self.project(first)[0], self.project(execution)[0])

    def test_selected_job_cannot_depend_on_omitted_job(self):
        text = 'jobs:\n  build:\n    runs-on: ubuntu-latest\n  deploy:\n    needs: build\n    runs-on: ubuntu-latest\n'
        with self.assertRaisesRegex(helper.BoundaryError, "omitted jobs"):
            self.project(text, ["deploy"])
        self.project(text, ["build", "deploy"])

    def test_local_action_and_reusable_workflow_inputs_fail_closed(self):
        for text in ('jobs:\n  build:\n    uses: ./.github/workflows/local.yml\n',
                     'jobs:\n  build:\n    steps:\n      - uses: ./actions/local\n'):
            with self.subTest(text=text), self.assertRaisesRegex(helper.BoundaryError, "source-input closure"):
                self.project(text)

    def test_duplicate_keys_and_recursive_aliases_fail_closed(self):
        for text in ('jobs:\n  build: {}\n  build: {}\n', 'jobs: &loop\n  build: *loop\n'):
            with self.subTest(text=text), self.assertRaises(helper.BoundaryError):
                self.project(text)


class GitProofTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.git("init", "--quiet")
        self.scope = scopes()[0]
        required = helper.COMMON_REQUIRED | self.scope["required"] | {self.scope["workflow"]}
        for name in required - {"FerroUI"}:
            target = self.root / name
            target.parent.mkdir(parents=True, exist_ok=True)
            target.write_text("fixture\n")
        (self.root / self.scope["workflow"]).write_text(
            'name: Build and publish\non: push\njobs:\n  build:\n    steps: [{run: "echo fixture"}]\n  deploy:\n    needs: build\n    steps: [{run: "echo fixture"}]\n')
        (self.root / "crates/phone-browser/src").mkdir(parents=True)
        (self.root / "crates/phone-browser/src/lib.rs").write_text("// fixture source\n")
        self.git("add", ".")
        self.git("update-index", "--add", "--cacheinfo", "160000," + SHA + ",FerroUI")
        self.commit("test fixture")
        self.tested = self.git("rev-parse", "HEAD").strip()
        self.scope["tested_ref"] = self.tested
        self.record_path = self.root / helper.EVIDENCE_DIR / "browser.json"
        self.record_path.parent.mkdir(parents=True)
        record = retained("browser.json")
        record["head_sha"] = self.tested
        self.record_path.write_text(json.dumps(record))
        self.generator = self.root / "fixture-generator.py"
        self.generator.write_text("# isolated generator fixture\n")
        self.output = self.root / helper.EVIDENCE_DIR / "source-boundaries.json"

    def git(self, *args):
        result = subprocess.run(["git", *args], cwd=self.root, text=True, capture_output=True, check=True)
        return result.stdout

    def commit(self, message):
        self.git("-c", "user.name=Boundary Tests", "-c", "user.email=boundary-tests@example.invalid",
                 "commit", "--quiet", "-m", message)

    def proof(self, ref=None):
        return helper.make_proof(self.root, ref or self.tested, [self.scope], generator=self.generator)

    def save(self):
        proof = self.proof()
        self.output.write_text(json.dumps(proof))
        return proof

    def check(self, current=None):
        return helper.check_saved_proof(self.root, self.output, current or self.proof(), [self.scope], generator=self.generator)

    def test_check_only_requires_existing_saved_proof(self):
        with self.assertRaisesRegex(helper.BoundaryError, "Missing saved"):
            self.check()
        self.assertFalse(self.output.exists())

    def test_corrupt_saved_proof_is_not_replaced_or_ignored(self):
        self.output.write_bytes(b'{"schema":')
        with self.assertRaisesRegex(helper.BoundaryError, "invalid JSON"):
            self.check()
        self.assertEqual(self.output.read_bytes(), b'{"schema":')

    def test_valid_saved_proof_is_not_rewritten(self):
        original = self.save()
        before = self.output.read_bytes()
        self.assertEqual(self.check(), original)
        self.assertEqual(self.output.read_bytes(), before)

    def test_docs_only_head_reconstructs_the_recorded_implementation(self):
        original = self.save()
        (self.root / "docs/note.md").write_text("A documentation-only final commit.\n")
        self.git("add", "docs/note.md")
        self.commit("docs only")
        new_head = self.git("rev-parse", "HEAD").strip()
        self.assertNotEqual(new_head, original["implementation_head"])
        validated = self.check(self.proof(new_head))
        self.assertEqual(validated["implementation_head"], self.tested)

    def test_stale_evidence_bytes_fail_even_when_outcomes_are_unchanged(self):
        self.save()
        record = json.loads(self.record_path.read_text())
        record["note"] = "Changed recorded evidence bytes"
        self.record_path.write_text(json.dumps(record))
        with self.assertRaisesRegex(helper.BoundaryError, "reconstructed"):
            self.check()

    def test_changed_generator_fails_saved_proof_validation(self):
        self.save()
        self.generator.write_text("# modified generator fixture\n")
        with self.assertRaisesRegex(helper.BoundaryError, "reconstructed"):
            self.check()

    def test_tampered_fingerprint_scope_or_missing_target_fails(self):
        for defect in ("fingerprint", "scope", "target", "evidence"):
            saved = self.save()
            if defect == "fingerprint":
                saved["targets"][0]["source_configuration_sha256"] = HASH
            elif defect == "scope":
                saved["targets"][0]["scope"]["extra_files"] = ["unreviewed-file"]
            elif defect == "target":
                saved["targets"] = []
            else:
                saved["targets"][0]["verification_records"][0]["sha256"] = HASH
            self.output.write_text(json.dumps(saved))
            with self.subTest(defect=defect), self.assertRaisesRegex(helper.BoundaryError, "reconstructed"):
                self.check()

    def test_saved_implementation_commit_is_independently_fingerprinted(self):
        saved = self.save()
        source = self.root / "crates/phone-browser/src/lib.rs"
        source.write_text("// material untested change\n")
        self.git("add", str(source.relative_to(self.root)))
        self.commit("untested source")
        different = self.git("rev-parse", "HEAD").strip()
        saved["implementation_head"] = different
        saved["targets"][0]["implementation_head"] = different
        self.output.write_text(json.dumps(saved))
        with self.assertRaisesRegex(helper.BoundaryError, "source/configuration changed"):
            self.check()

    def test_material_current_source_difference_fails_before_recording(self):
        source = self.root / "crates/phone-browser/src/lib.rs"
        source.write_text("// new current code\n")
        self.git("add", str(source.relative_to(self.root)))
        self.commit("material current source")
        with self.assertRaisesRegex(helper.BoundaryError, "source/configuration changed"):
            self.proof(self.git("rev-parse", "HEAD").strip())

    def test_ios_shared_icon_survives_excluded_browser_crate_scope(self):
        ios = scopes()[4]
        path = "crates/phone-browser/wwwroot/icon-512.png"
        self.assertIn(path, helper.IOS_REQUIRED)
        self.assertTrue(helper.includes(path, ios))
        self.assertFalse(helper.includes("crates/phone-browser/src/lib.rs", ios))
        for name in helper.COMMON_REQUIRED | ios["required"] | {ios["workflow"]}:
            if name == "FerroUI":
                continue
            target = self.root / name
            target.parent.mkdir(parents=True, exist_ok=True)
            target.write_bytes(b"fixture icon or source")
        (self.root / ios["workflow"]).write_text("jobs:\n  ios:\n    steps: [{run: fixture}]\n")
        self.git("add", ".")
        self.git("update-index", "--add", "--cacheinfo", "160000," + SHA + ",FerroUI")
        self.commit("ios fixture")
        first = self.git("rev-parse", "HEAD").strip()
        before, _ = helper.fingerprint(self.root, first, ios)
        (self.root / path).write_bytes(b"changed icon")
        self.git("add", path)
        self.commit("changed consumed UIKit icon")
        after, _ = helper.fingerprint(self.root, self.git("rev-parse", "HEAD").strip(), ios)
        with self.assertRaisesRegex(helper.BoundaryError, "icon-512.png"):
            helper.assert_identical(before, after, "ios", first)
        (self.root / path).unlink()
        self.git("add", path)
        self.commit("missing consumed UIKit icon")
        with self.assertRaisesRegex(helper.BoundaryError, "missing required files"):
            helper.fingerprint(self.root, self.git("rev-parse", "HEAD").strip(), ios)


if __name__ == "__main__":
    unittest.main()
