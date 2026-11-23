#!/usr/bin/env python3
"""Record target-specific source equality only after all final test refs are known.

This helper requires Python 3.10+ and PyYAML. Pass --repo when using an archived
copy from the evidence directory.
It does not fetch refs, run builds/tests, or infer a broader qualification from
source equality. No output file is written unless every comparison succeeds.

Example, after final evidence records exist:
    python record_launcher_source_boundaries.py --repo /path/to/WindowsPhone \
        --browser <tested-commit> --desktop <tested-commit> \
        --android-package <tested-commit> --android-ui <tested-commit> \
        --ios <tested-commit>

Do not substitute an unverified candidate ref for a tested ref merely to make a
comparison pass. A material difference requires new evidence or an explicit,
reviewed scope correction in this helper.
"""
from __future__ import annotations

import argparse
import copy
import datetime as dt
import hashlib
import json
import os
from pathlib import Path
import re
import subprocess
import sys
import tempfile
from typing import Any

EVIDENCE_DIR = Path("docs/verification/launcher-native-integration")
COMMON_FILES = {
    "Cargo.toml", "Cargo.lock", "rust-toolchain.toml", ".gitmodules", "FerroUI",
    "scripts/ci-framework-mtimes.py",
}
COMMON_REQUIRED = COMMON_FILES | {".cargo/config.toml"}
ANDROID_PACKAGE_SCRIPTS = {
    "scripts/build-android.sh", "scripts/test-android-contracts.sh",
    "scripts/verify-android-package.py", "scripts/verify_android_package_test.py",
    "scripts/verify_android_vendor.py",
}
ANDROID_UI_SCRIPTS = {
    "scripts/smoke-android.sh", "scripts/smoke-android-device.sh",
    "scripts/verify_android_ui.py", "scripts/verify_android_ui_test.py",
    "scripts/verify_android_device.py",
    "scripts/verify_android_device_test.py", "scripts/verify_android_bootstrap.py",
    "scripts/verify_android_bootstrap_test.py", "scripts/smoke-android-home.sh",
    "scripts/smoke-android-notifications.sh",
}
IOS_REQUIRED = {
    "scripts/build-ios.sh", "scripts/ios-bundle-metadata.py",
    "scripts/smoke-ios.sh", "scripts/verify-ios-smoke.py",
    "scripts/verify-ios-devices.py", "scripts/verify_ios_smoke_test.py",
    "scripts/collect-ios-failure.py", "scripts/collect_ios_failure_test.py",
    "scripts/ios-ui-tests/PhonePromptTests.swift",
    "scripts/ios-ui-tests/PhonePromptTests.xcodeproj/project.pbxproj",
    "scripts/ios-ui-tests/PhonePromptTests.xcodeproj/xcshareddata/xcschemes/PhonePromptTests.xcscheme",
    # build-ios.sh resizes this shared asset into the actual UIKit bundle.
    "crates/phone-browser/wwwroot/icon-512.png",
}
BROWSER_REQUIRED = {
    "package.json", "scripts/build-browser.sh", "scripts/prepare-site.mjs",
    "scripts/verify-site.mjs", "scripts/verify-reuse.mjs",
    "scripts/startup-smoke.mjs", "scripts/ui-smoke.mjs",
    "scripts/ui-map-fixture.mjs", "scripts/ui-maps-expanded.mjs",
    "scripts/ui-start-customization.mjs",
}
EXCLUDED_DEVELOPMENT_SCRIPTS = {"scripts/new-phone-app.py"}
DOC_SUFFIXES = (".md", ".markdown", ".rst")
SCHEMA = 3
REPOSITORY = "FerroUI/WindowsPhone"
WORKFLOW_NORMALIZATION = "yaml-node-explicit-tag-style-value-v1"
BROWSER_STEPS = {
    "Verify state", "Verify SDK and game contracts", "Verify host boundaries",
    "Build Windows Phone", "Verify rendered desktop and touch views",
}
DESKTOP_GATES = {
    "Verify portable host boundaries", "Verify session packaging boundaries",
    "Verify shared native services", "Build and link the complete native host",
}
ANDROID_PACKAGE_STEPS = {
    "Verify portable Android completion and media contracts",
    "Build native application and HOME launcher APKs",
    "Check the Android embedding library",
    "Verify merged manifests, JNI entry and native alignment",
}
ANDROID_UI_STEPS = {
    "Verify package and mounted-layout contracts",
    "Build x86_64 application and launcher APKs",
    "Verify the actual emulator packages",
    "Verify native geometry, HOME, cold-Start unread badge, notification bootstrap/revocation and carrier call ownership",
}
IOS_STEPS = {
    "Verify portable iOS completion contracts",
    "Type check the optional carrier provider for physical iPhone",
    "Check all native bridge files against the simulator SDK",
    "Build and bundle the standard UIKit application",
    "Build and bundle the optional carrier provider",
    "Verify native UI, device roundtrips and activation",
}
IOS_GATES = {
    "standard_uikit_full_build_link_bundle", "optional_carrier_full_build_link_bundle",
    "physical_iphone_optional_swift_typecheck", "simulator_objc_syntax_feature_0_and_1",
    "xctest_system_url_driver_build",
}
IOS_PROVIDER_CHECKS = {
    "observed_simulator_capabilities", "brightness_read_only", "contact_create_and_query",
    "contact_update_and_query", "contact_delete_and_query",
}
IOS_URIS = [
    "ferroui-phone://app/org.ferroui.wallet",
    "ferroui-phone://app/phone.phone?path=dial/KzEgMjAyIDU1NSAwMTQ3",
    "ferroui-phone://app/phone.people?path=device",
    "ferroui-phone://app/phone.messaging?path=compose",
    "ferroui-phone://app/phone.settings?path=device",
    "ferroui-phone://home",
]
IOS_INVARIANTS = {
    "same_process", "foreground_active", "exact_native_uris", "mounted_controls",
    "safe_area_containment",
}
ANDROID_MOUNTS = {
    "app-start", "app-wallet", "app-back", "app-home-restored",
    "launcher-start", "launcher-wallet", "launcher-back", "launcher-home-restored",
    "home-role-start", "home-role-wallet", "home-role-returned", "home-role-back-start",
}


class BoundaryError(Exception):
    pass


def compact(value: Any) -> bytes:
    return json.dumps(value, ensure_ascii=True, sort_keys=True,
                      separators=(",", ":"), allow_nan=False).encode("utf-8")


def sha256(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def git(root: Path, *args: str) -> bytes:
    result = subprocess.run(["git", *args], cwd=root, capture_output=True)
    if result.returncode:
        raise BoundaryError(result.stderr.decode("utf-8", errors="replace").strip())
    return result.stdout


def resolve_commit(root: Path, ref: str) -> str:
    value = git(root, "rev-parse", "--verify", "--end-of-options",
                ref + "^{commit}").decode().strip()
    if not re.fullmatch(r"[0-9a-f]{40}", value):
        raise BoundaryError(f"Expected a full SHA-1 commit for {ref!r}: {value!r}")
    return value


def tree(root: Path, ref: str) -> dict[str, tuple[str, str, str]]:
    entries = {}
    for row in git(root, "ls-tree", "-rz", "--full-tree", ref).split(b"\0"):
        if not row:
            continue
        info, raw_path = row.split(b"\t", 1)
        mode, kind, oid = info.decode("ascii").split()
        path = raw_path.decode("utf-8", errors="strict")
        entries[path] = (mode, kind, oid)
    return entries


def script_family(path: str) -> str | None:
    """Fail closed on a new, unclassified verification helper."""
    if not path.startswith("scripts/"):
        return None
    name = Path(path).name.lower()
    if path in EXCLUDED_DEVELOPMENT_SCRIPTS:
        return "scaffolding"
    if path == "scripts/ci-framework-mtimes.py":
        return "shared"
    if path.startswith("scripts/ios-ui-tests/") or "ios" in name:
        return "ios"
    if "android" in name:
        if path in ANDROID_UI_SCRIPTS or name.startswith(("verify_android_ui", "verify_android_device", "smoke-android")):
            return "android_ui"
        return "android_package"
    if path == "scripts/build-browser.sh" or Path(path).suffix in {".mjs", ".js", ".cjs"}:
        return "browser"
    if path.lower().endswith(DOC_SUFFIXES):
        return "documentation"
    raise BoundaryError(f"Unclassified script or fixture requires an explicit target scope: {path}")


def is_doc(path: str) -> bool:
    return path.startswith("docs/") or path.lower().endswith(DOC_SUFFIXES)


def includes(path: str, scope: dict[str, Any]) -> bool:
    if is_doc(path):
        return False
    if path in COMMON_FILES or path.startswith(".cargo/"):
        return True
    if path in scope.get("extra_files", set()):
        return True
    if path.startswith("crates/"):
        return not any(path.startswith("crates/" + name + "/")
                       for name in scope["excluded_crates"])
    if path.startswith("scripts/"):
        return script_family(path) in scope["script_families"]
    if scope["name"] == "browser" and path in {"package.json", "package-lock.json", "npm-shrinkwrap.json"}:
        return True
    return False


def workflow_projection(root: Path, ref: str, path: str,
                        selected_jobs: list[str]) -> tuple[list, dict]:
    """Retain node tags, scalar style and value, including mapping keys.

    BaseLoader avoids YAML 1.1's accidental boolean interpretation of `on`.
    No scalar is constructed into a Python value: null/"null", false/"false",
    explicit tags and lexical numbers remain distinguishable. Comments and
    mapping order are ignored; conservative quote/style differences are kept.
    """
    try:
        import yaml
    except ImportError as error:
        raise BoundaryError("PyYAML is required for strict workflow comparison; it is not installed automatically.") from error

    class LexicalLoader(yaml.BaseLoader):
        def compose_scalar_node(self, anchor):
            # BaseLoader intentionally does not infer types. Preserve whether
            # a tag was explicit so !!str null cannot collide with plain null.
            explicit_tag = self.peek_event().tag
            node = super().compose_scalar_node(anchor)
            node.explicit_tag = explicit_tag
            return node

    def members(node):
        if not isinstance(node, yaml.MappingNode):
            raise BoundaryError(f"Expected a workflow mapping in {path} at {ref}")
        result = {}
        for key, value in node.value:
            if (not isinstance(key, yaml.ScalarNode)
                    or key.tag != "tag:yaml.org,2002:str" or key.value in result):
                raise BoundaryError(f"Non-string or duplicate YAML key in {path} at {ref}")
            result[key.value] = (key, value)
        return result

    def normalize(node, ancestors=frozenset()):
        if id(node) in ancestors:
            raise BoundaryError(f"Recursive YAML aliases are unsupported: {path}")
        ancestors = ancestors | {id(node)}
        if isinstance(node, yaml.ScalarNode):
            return ["scalar", node.tag, node.explicit_tag, node.style, node.value]
        if isinstance(node, yaml.SequenceNode):
            return ["sequence", node.tag, [normalize(v, ancestors) for v in node.value]]
        if isinstance(node, yaml.MappingNode):
            items = members(node)
            return ["mapping", node.tag, [
                [normalize(k, ancestors), normalize(v, ancestors)]
                for _name, (k, v) in sorted(items.items())]]
        raise BoundaryError(f"Unsupported YAML node in {path}")

    def subset(node, names):
        items = members(node)
        return yaml.MappingNode(node.tag, [items[name] for name in names])

    def remove(node, name):
        items = members(node)
        value = items.pop(name, None)
        node.value = list(items.values())
        return value[1] if value is not None else None

    def text_value(node, label):
        if not isinstance(node, yaml.ScalarNode) or node.tag != "tag:yaml.org,2002:str":
            raise BoundaryError(f"{path}: {label} must be a literal string")
        return node.value

    def check_execution_dependencies(name, job):
        items = members(job)
        if "needs" in items:
            need_node = items["needs"][1]
            needs = need_node.value if isinstance(need_node, yaml.SequenceNode) else [need_node]
            dependencies = [text_value(value, f"jobs.{name}.needs") for value in needs]
            outside = set(dependencies) - set(selected_jobs)
            if outside:
                raise BoundaryError(f"{path}: selected job {name} needs omitted jobs {sorted(outside)}")
        # The current workflows use no local actions/reusable workflows. Fail
        # closed until a future scope explicitly covers their complete closure,
        # including action metadata, scripts, Dockerfiles and nested local uses.
        uses = []
        if "uses" in items:
            uses.append(items["uses"][1])
        if "steps" in items:
            steps = items["steps"][1]
            if not isinstance(steps, yaml.SequenceNode):
                raise BoundaryError(f"{path}: jobs.{name}.steps is not a sequence")
            for step in steps.value:
                step_items = members(step)
                if "uses" in step_items:
                    uses.append(step_items["uses"][1])
        for value in uses:
            action = text_value(value, f"jobs.{name}.uses")
            if action.startswith(("./", "../", "/")):
                raise BoundaryError(f"{path}: local action/reusable workflow {action!r} has no reviewed source-input closure")

    raw = git(root, "show", f"{ref}:{path}")
    try:
        document = yaml.compose(raw.decode("utf-8"), Loader=LexicalLoader)
    except (UnicodeError, yaml.YAMLError) as error:
        raise BoundaryError(f"Cannot parse {path} at {ref}: {error}") from error
    # Validate every node, even in excluded jobs, before selecting execution.
    normalize(document)
    top = members(document)
    if "jobs" not in top:
        raise BoundaryError(f"Workflow jobs are missing: {path} at {ref}")
    result = copy.deepcopy(document)
    omitted = {}
    trigger = remove(result, "on")
    if isinstance(trigger, yaml.MappingNode):
        trigger_names = members(trigger)
        runtime_names = [key for key in trigger_names if key in {"workflow_dispatch", "workflow_call"}]
        omitted["on.event_filters"] = normalize(subset(trigger, [key for key in trigger_names if key not in runtime_names]))
        if runtime_names:
            result.value.append((top["on"][0], subset(trigger, runtime_names)))
    elif trigger is not None:
        omitted["on.event_filters"] = normalize(trigger)
    concurrency = remove(result, "concurrency")
    if concurrency is not None:
        omitted["concurrency"] = normalize(concurrency)
    jobs_key, jobs_node = members(result)["jobs"]
    jobs = members(jobs_node)
    missing = set(selected_jobs) - jobs.keys()
    if missing or len(selected_jobs) != len(set(selected_jobs)):
        raise BoundaryError(f"Missing or duplicate selected jobs in {path} at {ref}: {sorted(missing)}")
    selected = []
    for name in selected_jobs:
        key, original = jobs[name]
        job = copy.deepcopy(original)
        check_execution_dependencies(name, job)
        condition = remove(job, "if")
        if condition is not None:
            omitted[f"jobs.{name}.if"] = normalize(condition)
        selected.append((key, job))
    remove(result, "jobs")
    result.value.append((jobs_key, yaml.MappingNode(jobs_node.tag, selected)))
    projection = normalize(result)
    return projection, {
        "file": path,
        "normalization": WORKFLOW_NORMALIZATION,
        "selected_jobs": selected_jobs,
        "unrelated_jobs_excluded": sorted(set(jobs) - set(selected_jobs)),
        "blob_sha256": sha256(raw),
        "execution_configuration_sha256": sha256(compact(projection)),
        "omitted_scheduling_fields": omitted,
    }


def require(condition: bool, label: str) -> None:
    if not condition:
        raise BoundaryError(label)


def object_at(data: dict, key: str, label: str) -> dict:
    value = data.get(key)
    require(isinstance(value, dict), f"{label}.{key}: expected an object")
    return value


def array_at(data: dict, key: str, label: str) -> list:
    value = data.get(key)
    require(isinstance(value, list) and bool(value), f"{label}.{key}: expected a nonempty array")
    return value


def positive(value: Any) -> bool:
    return type(value) is int and value > 0


def digest(value: Any) -> bool:
    return isinstance(value, str) and re.fullmatch(r"[0-9a-f]{64}", value) is not None


def strict_json(raw: bytes, label: str) -> dict:
    def pairs(items):
        result = {}
        for key, value in items:
            require(key not in result, f"{label}: duplicate JSON key {key!r}")
            result[key] = value
        return result

    def invalid_constant(value):
        raise BoundaryError(f"{label}: non-finite JSON value {value}")

    try:
        result = json.loads(raw, object_pairs_hook=pairs, parse_constant=invalid_constant)
    except (UnicodeError, json.JSONDecodeError) as error:
        raise BoundaryError(f"{label}: invalid JSON") from error
    require(isinstance(result, dict), f"{label}: expected a JSON object")
    # JSON numeric overflow (e.g. 1e400) must not bypass parse_constant.
    try:
        compact(result)
    except (TypeError, ValueError) as error:
        raise BoundaryError(f"{label}: invalid JSON values") from error
    return result


def named_success(rows: Any, names: set[str], label: str) -> None:
    require(isinstance(rows, list), f"{label}: expected recorded steps/gates")
    for name in sorted(names):
        matches = [row for row in rows if isinstance(row, dict) and row.get("name") == name]
        require(len(matches) == 1, f"{label}: missing or duplicate required gate {name!r}")
        row = matches[0]
        require(row.get("conclusion") == "success" and row.get("status", "completed") == "completed",
                f"{label}: required gate {name!r} did not execute successfully")


def completed_job(job: dict, label: str, name: str | None = None) -> None:
    require(job.get("status") == "completed" and job.get("conclusion") == "success",
            f"{label}: job must have completed successfully")
    require(positive(job.get("id", job.get("job_id"))), f"{label}: missing actual job ID")
    if name is not None:
        require(job.get("name") == name, f"{label}: expected job {name!r}")


def test_counts(row: dict, label: str) -> None:
    require(positive(row.get("passed")) and type(row.get("failed")) is int and row["failed"] == 0,
            f"{label}: expected positive passed count and zero failures")


def artifacts(data: dict, names: set[str], tested: str, label: str) -> None:
    rows = array_at(data, "artifacts", label)
    actual = [row.get("name") for row in rows if isinstance(row, dict)]
    require(len(actual) == len(rows) and len(set(actual)) == len(actual)
            and names <= set(actual), f"{label}: missing, duplicate or invalid target artifacts")
    ids = set()
    for row in rows:
        require(positive(row.get("id")) and row["id"] not in ids, f"{label}: invalid or repeated artifact ID")
        ids.add(row["id"])
        recorded_hash = row.get("sha256")
        api_hash = row.get("digest")
        require(digest(recorded_hash) or (isinstance(api_hash, str) and api_hash.startswith("sha256:")
                                        and digest(api_hash[7:])), f"{label}: missing artifact SHA-256")
        if recorded_hash is not None and api_hash is not None:
            require(api_hash == "sha256:" + recorded_hash, f"{label}: artifact digests disagree")
        for field in ("head_sha", "source_commit", "tested_commit"):
            if field in row:
                require(row[field] == tested, f"{label}: artifact {field} is not the tested commit")
        bound_to_run = False
        if "workflow_run" in row:
            run = object_at(row, "workflow_run", label + ".artifact")
            require(run.get("head_sha") == tested and run.get("id") == data["run_id"],
                    f"{label}: artifact workflow head/run differs from the verification record")
            bound_to_run = True
        if "run_id" in row:
            require(row["run_id"] == data["run_id"], f"{label}: artifact run differs")
            bound_to_run |= row.get("head_sha", row.get("source_commit", row.get("tested_commit"))) == tested
        # The already-retained browser summary uses GitHub's per-run artifact
        # links instead of repeating workflow_run metadata. Tie that exact run
        # and artifact ID to this record; a bare hash/ID alone is insufficient.
        artifact_url = row.get("url", "")
        if isinstance(artifact_url, str):
            match = re.fullmatch(r"https://github\.com/FerroUI/WindowsPhone/actions/runs/(\d+)/artifacts/(\d+)", artifact_url)
            if match is not None:
                require((int(match[1]), int(match[2])) == (data["run_id"], row["id"]),
                        f"{label}: artifact URL names a different workflow run or artifact")
                bound_to_run = True
        require(bound_to_run, f"{label}: artifact lacks tested-head/run provenance")
        for field in ("sha256_verified", "downloaded_and_hash_verified", "download_verified", "extracted_files_match_zip"):
            if field in row:
                require(row[field] is True, f"{label}: artifact {field} is not true")


def validate_browser(data: dict, tested: str, label: str) -> list[str]:
    job = object_at(data, "job", label)
    completed_job(job, label + ".job", "build")
    require(job.get("run_id") == data["run_id"], f"{label}: build run differs")
    named_success(job.get("steps"), BROWSER_STEPS, label + ".job.steps")
    build = object_at(data, "wasm_build", label)
    require(build.get("result") == "success" and build.get("compile_reused") is False,
            f"{label}: current evidence must record the full WASM build")
    rust = object_at(data, "rust_tests", label)
    test_counts(rust, label + ".rust_tests")
    groups = array_at(rust, "groups", label + ".rust_tests")
    for group in groups:
        require(isinstance(group, dict), f"{label}: invalid Rust test group")
        test_counts(group, label + ".rust_tests.group")
    require(sum(row["passed"] for row in groups) == rust["passed"], f"{label}: Rust test totals disagree")
    test_counts(object_at(data, "javascript_tests", label), label + ".javascript_tests")
    rendered = object_at(data, "rendered_ui", label)
    require(rendered.get("passed") is True and rendered.get("errors") == []
            and positive(rendered.get("checks")) and positive(rendered.get("captures"))
            and digest(rendered.get("results_sha256")) and bool(rendered.get("results_path")),
            f"{label}: rendered suite evidence is incomplete or failed")
    for check in rendered.get("focus_checks", []):
        require(isinstance(check, dict) and check.get("passed") is True, f"{label}: a rendered focus check failed")
    publication = object_at(data, "publication", label)
    deploy = object_at(publication, "job", label + ".publication")
    completed_job(deploy, label + ".publication.job", "deploy")
    require(deploy.get("run_id") == data["run_id"], f"{label}: deployment run differs")
    named_success(deploy.get("steps"), {"Run actions/deploy-pages@v4"}, label + ".publication.job.steps")
    artifacts(data, {"compiled-app", "ui-verification", "github-pages"}, tested, label)
    return sorted(BROWSER_STEPS | {"Pages deployment"})


def validate_desktop(data: dict, _tested: str, label: str) -> list[str]:
    jobs = array_at(data, "jobs", label)
    platforms = [job.get("platform") for job in jobs if isinstance(job, dict)]
    require(len(jobs) == 3 and set(platforms) == {"linux", "windows", "macos"},
            f"{label}: all three distinct native platforms must be qualified")
    for job in jobs:
        prefix = label + "." + job["platform"]
        completed_job(job, prefix)
        gates = set(DESKTOP_GATES)
        if job["platform"] == "linux":
            gates.add("Verify Linux media-session protocol")
        if job["platform"] == "windows":
            gates |= {"Verify Windows compiler and standard library compatibility", "Parse Windows shell configuration helper"}
        named_success(job.get("gates"), gates, prefix + ".gates")
        rows = array_at(job, "tests", prefix)
        for crate in ("phone-host", "phone-platform-desktop", "phone-native"):
            matches = [row for row in rows if isinstance(row, dict) and row.get("crate") == crate
                       and "dbus-run-session" not in row.get("command", "")]
            require(len(matches) == 1, f"{prefix}: missing or duplicate tests for {crate}")
            require(matches[0].get("result") == "ok", f"{prefix}: {crate} tests failed")
            test_counts(matches[0], prefix + "." + crate)
        if job["platform"] == "linux":
            matches = [row for row in rows if isinstance(row, dict)
                       and "dbus-run-session" in row.get("command", "")
                       and "private_bus_media_controls" in row.get("command", "")]
            require(len(matches) == 1 and matches[0].get("result") == "ok", f"{prefix}: missing MPRIS protocol execution")
            test_counts(matches[0], prefix + ".MPRIS")
        packaging = object_at(job, "packaging", prefix)
        test_counts(packaging, prefix + ".packaging")
        require(type(packaging.get("skipped")) is int and packaging["skipped"] >= 0
                and packaging.get("total") == packaging["passed"] + packaging["skipped"],
                f"{prefix}: packaging totals disagree")
        builds = array_at(job, "builds", prefix)
        full = [row for row in builds if isinstance(row, dict)
                and row.get("command") == "cargo build --locked -p phone-desktop"]
        require(len(full) == 1 and full[0].get("profile") == "dev"
                and bool(full[0].get("completed_at")) and bool(full[0].get("cargo_duration")),
                f"{prefix}: full native link evidence is absent")
        require(digest(object_at(job, "log", prefix).get("sha256")), f"{prefix}: missing native log hash")
    return ["linux/windows/macos: host, plugin, native services, packaging and full link",
            "linux: private-bus MPRIS protocol", "windows: compiler and shell script parsing"]


def android_packages(rows: Any, abi: str, label: str) -> None:
    require(isinstance(rows, list) and len(rows) == 2, f"{label}: both actual Android packages are required")
    packages = {row.get("package"): row for row in rows if isinstance(row, dict)}
    require(set(packages) == {"org.ferroui.windowsphone", "org.ferroui.windowsphone.launcher"},
            f"{label}: incorrect application/launcher package coverage")
    for package, row in packages.items():
        require(row.get("abi") == abi and row.get("launcher") is package.endswith(".launcher")
                and positive(row.get("native_bytes")) and digest(row.get("native_sha256"))
                and digest(row.get("apk_sha256")), f"{label}: invalid {package} binary evidence")
        alignments = object_at(row, "load_alignment", label)
        require(bool(alignments) and f"lib/{abi}/libphone_android.so" in alignments,
                f"{label}: missing native ELF alignment evidence")
        for library, values in alignments.items():
            require(library.startswith(f"lib/{abi}/") and isinstance(values, list) and bool(values)
                    and all(type(value) is int and value >= 16384 and value % 16384 == 0 for value in values),
                    f"{label}: invalid 16 KiB load alignment")


def validate_android_package(data: dict, tested: str, label: str) -> list[str]:
    require(data.get("conclusion") == "success" and data.get("target") == "aarch64-linux-android",
            f"{label}: not a successful Android ARM64 package record")
    named_success(data.get("steps"), ANDROID_PACKAGE_STEPS, label + ".steps")
    results = object_at(data, "results", label)
    for name in ("native_link", "app_apk_assembly", "launcher_apk_assembly", "embedding_android_target_check"):
        require(object_at(results, name, label + ".results").get("result") == "passed", f"{label}: {name} did not pass")
    require(results["embedding_android_target_check"].get("default_features") is False,
            f"{label}: Android embedding library configuration was not checked")
    for name in ("jvm_contract_assertions", "aosp_source_hashes", "python_package_contract_tests", "rust_android_contract_tests"):
        require(positive(results.get(name)), f"{label}: missing {name}")
    for name in ("native_16k_elf_alignment", "apk_16k_zip_alignment"):
        require(isinstance(results.get(name), str) and results[name].startswith("passed"), f"{label}: {name} did not pass")
    verified = object_at(data, "artifact_verification", label)
    require(digest(verified.get("archive_sha256")), f"{label}: missing verified package archive hash")
    android_packages(verified.get("packages"), "arm64-v8a", label)
    manifests = array_at(verified, "call_service_manifests", label)
    require(len(manifests) == 2 and {row.get("flavor") for row in manifests if isinstance(row, dict)} == {"app", "launcher"},
            f"{label}: missing merged call-service manifests")
    for manifest in manifests:
        require(manifest.get("phone_call_service_type") == "0x4"
                and manifest.get("protected_by") == "android.permission.BIND_INCALL_SERVICE"
                and manifest.get("foreground_phone_call_permission") is True
                and manifest.get("manage_own_calls_permission") is False,
                f"{label}: invalid call-service ownership manifest")
    artifacts(data, {"phone-android-arm64-debug"}, tested, label)
    return sorted(ANDROID_PACKAGE_STEPS)


def component_set(value: Any, label: str) -> set[str]:
    require(isinstance(value, list) and all(isinstance(item, str) and item for item in value),
            f"{label}: expected notification component array")
    require(len(set(value)) == len(value), f"{label}: duplicate notification component")
    return set(value)


def validate_android_ui(data: dict, tested: str, label: str) -> list[str]:
    target = object_at(data, "target", label)
    require(data.get("conclusion") == "success" and target.get("platform") == "android"
            and target.get("abi") == "x86_64", f"{label}: not an Android x86_64 runtime success record")
    named_success(data.get("steps"), ANDROID_UI_STEPS, label + ".steps")
    android_packages(data.get("package_verification"), "x86_64", label)
    runtime = object_at(data, "runtime", label)
    require(runtime.get("result") == "passed" and runtime.get("crash_buffer_bytes") == 0,
            f"{label}: runtime phase failed or recorded a crash")
    cold = object_at(runtime, "cold_start_unread", label + ".runtime")
    badge = object_at(cold, "badge", label + ".cold_start_unread")
    require(cold.get("result") == "passed" and cold.get("route") == "Start"
            and "selected_thread" in cold and cold["selected_thread"] is None
            and cold.get("unread") == 1 and bool(cold.get("native_thread_id"))
            and badge.get("text") == "1" and badge.get("fully_visible") is True
            and bool(badge.get("tile_id")) and bool(cold.get("observation")) and bool(cold.get("capture")),
            f"{label}: cold Start native unread/badge proof is missing")
    notices = object_at(runtime, "notification_bootstrap", label + ".runtime")
    require(notices.get("result") == "passed" and isinstance(notices.get("component"), str)
            and bool(notices["component"]), f"{label}: notification bootstrap did not pass")
    before, granted, revoked, restored = [component_set(notices.get(key), label + ".notifications." + key)
                                          for key in ("before", "granted", "revoked", "restored")]
    component = notices["component"]
    require(notices.get("grant_exact") is True and notices.get("restored_exact") is True
            and granted == before | {component} and revoked == before - {component} and restored == before,
            f"{label}: notification access ownership/restoration sets disagree")
    admitted = object_at(notices, "admitted", label)
    require(admitted.get("package_id") == "com.android.shell"
            and all(bool(admitted.get(key)) for key in ("observation", "key", "title", "body")),
            f"{label}: missing native notification admission proof")
    rendered = object_at(notices, "rendered", label)
    require(rendered.get("title_body_visible") is True and bool(rendered.get("observation"))
            and bool(rendered.get("capture")), f"{label}: notification rendering did not pass")
    revocation = object_at(notices, "revocation", label)
    require(revocation.get("read_operation") is False and revocation.get("store_count") == 0
            and revocation.get("fixture_labels") == [] and bool(revocation.get("observation"))
            and bool(revocation.get("capture")), f"{label}: notification revocation/cleanup proof failed")
    roles = object_at(runtime, "roles_restored", label)
    for role in ("home", "dialer", "sms"):
        observed = object_at(roles, role, label + ".roles_restored")
        require("original" in observed and "restored" in observed and observed.get("equal") is True
                and observed["original"] == observed["restored"]
                and observed.get("selected") == "org.ferroui.windowsphone.launcher",
                f"{label}: {role} ownership was not restored")
    user = object_at(runtime, "android_user", label)
    require("before" in user and "after" in user and user.get("equal") is True and user["before"] == user["after"],
            f"{label}: Android user ownership changed")
    mounts = array_at(data, "mounted_observations", label)
    by_name = {row.get("observation"): row for row in mounts if isinstance(row, dict)}
    require(len(mounts) == len(ANDROID_MOUNTS) and set(by_name) == {name + ".json" for name in ANDROID_MOUNTS},
            f"{label}: incomplete distinct mounted UI checkpoints")
    for name, row in by_name.items():
        controls = row.get("mounted_controls")
        require(isinstance(controls, list) and positive(row.get("sequence")), f"{label}: missing mounted control observation")
        if "wallet" in name:
            require(row.get("route") == {"Extension": {"app": "org.ferroui.wallet", "path": ""}}
                    and "WalletSearch" in controls, f"{label}: Wallet was not mounted")
        else:
            require(row.get("route") == "Start" and {"StartScroll", "StartTiles"} <= set(controls),
                    f"{label}: Start was not mounted")
        # A launcher-flavor installation remains App until HOME is actually
        # selected; the package identity alone does not prove launcher mode.
        require(row.get("mode") == ("launcher" if name.startswith("home-role-") else "app"), f"{label}: incorrect host mode")
        if name.startswith("home-role-"):
            require(row.get("launcher_role") == "active", f"{label}: HOME ownership was not active")
        inner, screen, viewport = [object_at(row, key, label) for key in ("inner", "screen", "viewport")]
        for key in ("width", "height", "x", "y"):
            require(type(inner.get(key)) in (int, float) and type(screen.get(key)) in (int, float)
                    and abs(inner[key] - screen[key]) <= 1, f"{label}: measured screen exceeded its inset wrapper")
        for key in ("width", "height"):
            require(type(viewport.get(key)) in (int, float) and inner[key] > 0
                    and abs(inner[key] - viewport[key]) <= 1, f"{label}: viewport geometry mismatch")
    calls = object_at(data, "native_call_observations", label)
    call_id = None
    last_sequence = 0
    for phase in ("ringing", "active", "ended"):
        row = object_at(calls, phase, label + ".native_call_observations")
        presentation = object_at(row, "call_presentation", label)
        require(positive(row.get("sequence")) and row["sequence"] > last_sequence, f"{label}: stale call observation")
        last_sequence = row["sequence"]
        if phase == "ended":
            require(row.get("calls") == [] and presentation.get("service_type") == 0
                    and presentation.get("foreground_notification") is False and presentation.get("call_style") is False,
                    f"{label}: call foreground ownership was not released")
        else:
            current = array_at(row, "calls", label)
            require(len(current) == 1 and isinstance(current[0], dict) and current[0].get("state") == phase
                    and bool(current[0].get("id")) and row.get("native_call_visible") is True
                    and presentation.get("service_type") == 4 and presentation.get("foreground_notification") is True
                    and presentation.get("call_style") is True, f"{label}: actual call foreground presentation failed")
            require(call_id is None or current[0]["id"] == call_id, f"{label}: native call identity changed")
            call_id = current[0]["id"]
    artifacts(data, {"phone-android-x86_64-ui", "phone-android-x86_64-debug"}, tested, label)
    return sorted(ANDROID_UI_STEPS) + ["cold Start unread", "notification grant/render/revoke/restore", "12 mounted checkpoints", "ringing/active/ended foreground call ownership"]


def validate_ios(data: dict, tested: str, label: str) -> list[str]:
    require(data.get("conclusion") == "success" and data.get("workflow") == "Verify iOS host",
            f"{label}: not a successful iOS host record")
    named_success(data.get("steps"), IOS_STEPS, label + ".steps")
    gates = object_at(data, "gates", label)
    require(all(gates.get(name) == "passed" for name in IOS_GATES), f"{label}: an iOS build/typecheck gate did not pass")
    runtime = object_at(data, "runtime", label)
    require(object_at(runtime, "initial_mounted_start", label).get("passed") is True,
            f"{label}: initial UIKit Start was not mounted")
    providers = object_at(runtime, "provider_roundtrips", label)
    checks = array_at(providers, "checks", label)
    require(providers.get("passed") is True and len(checks) == len(IOS_PROVIDER_CHECKS)
            and {row.get("name") for row in checks if isinstance(row, dict)} == IOS_PROVIDER_CHECKS
            and all(row.get("passed") is True for row in checks), f"{label}: incomplete five-provider runtime proof")
    warm = object_at(runtime, "warm_activations", label)
    routes = array_at(warm, "routes", label)
    require(warm.get("passed") is True and warm.get("count") == 6 and len(routes) == 6
            and all(isinstance(row, dict) for row in routes)
            and [row.get("uri") for row in routes] == IOS_URIS
            and all(row.get("outcome") == "accepted" for row in routes),
            f"{label}: all six actual warm native URL routes must pass in order")
    invariants = object_at(runtime, "invariants", label)
    require(all(invariants.get(name) is True for name in IOS_INVARIANTS), f"{label}: iOS runtime invariants failed")
    artifacts(data, {"ios-verification", "phone-ios-simulator"}, tested, label)
    return sorted(IOS_STEPS | IOS_GATES) + ["five native provider roundtrips", "six same-process native warm URLs"]


VALIDATORS = {
    "browser": validate_browser, "desktop": validate_desktop,
    "android_arm64_package": validate_android_package, "android_x86_64_ui": validate_android_ui,
    "ios": validate_ios,
}


def validate_record(data: dict, tested: str, target: str, label: str) -> tuple[dict, list[str]]:
    require(data.get("repository") == REPOSITORY, f"{label}: wrong repository")
    require(data.get("head_sha") == tested, f"{label}: head_sha does not identify tested commit {tested}")
    require(positive(data.get("run_id")), f"{label}: missing actual workflow run ID")
    require(target in VALIDATORS, f"{label}: unknown target {target}")
    commits = {"head_sha": data["head_sha"]}
    for field in ("source_head", "runtime_commit", "tested_commit", "commit", "source_commit"):
        if field in data:
            require(data[field] == tested, f"{label}: conflicting {field}")
            commits[field] = data[field]
    source = data.get("source")
    if isinstance(source, str) or isinstance(source, dict) and "commit" in source:
        value = source if isinstance(source, str) else source["commit"]
        require(value == tested, f"{label}: conflicting source commit")
        commits["source" if isinstance(source, str) else "source.commit"] = value
    expected_workflow = {"browser": "Build and publish", "desktop": "Build and publish",
                         "android_arm64_package": "Verify Android hosts",
                         "android_x86_64_ui": "Verify Android mounted UI", "ios": "Verify iOS host"}[target]
    if "workflow" in data:
        require(data["workflow"] == expected_workflow, f"{label}: wrong target workflow")
    primary = [data, data.get("run"), data.get("job")]
    primary += data.get("jobs", []) if isinstance(data.get("jobs"), list) else []
    for item in primary:
        if not isinstance(item, dict):
            continue
        if "conclusion" in item:
            require(item["conclusion"] == "success", f"{label}: primary conclusion is not success")
        if "status" in item:
            require(item["status"] == "completed", f"{label}: primary status is unfinished")
        require(item.get("passed") is not False, f"{label}: primary outcome explicitly failed")
        for field in ("head_sha", "source_commit", "tested_commit"):
            if field in item:
                require(item[field] == tested, f"{label}: nested primary {field} differs")
        if isinstance(item.get("result"), str):
            require(item["result"] not in {"failed", "failure", "error", "cancelled", "pending", "skipped"},
                    f"{label}: unsuccessful primary result")
    return commits, VALIDATORS[target](data, tested, label)


def record_info(root: Path, path: Path, tested: str, target: str) -> dict[str, Any]:
    resolved = path.resolve()
    try:
        relative = resolved.relative_to(root)
    except ValueError as error:
        raise BoundaryError(f"Verification record must be inside the repository: {resolved}") from error
    require(resolved.is_file(), f"Missing verification record: {relative}")
    raw = resolved.read_bytes()
    data = strict_json(raw, str(relative))
    commit_fields, gates = validate_record(data, tested, target, str(relative))
    return {"path": relative.as_posix(), "sha256": sha256(raw), "bytes": len(raw),
            "tested_commit_fields": commit_fields, "qualified_target": target,
            "validated_gates": gates}


def scope_table(args) -> list[dict[str, Any]]:
    return [
        dict(name="browser", tested_ref=args.browser, record=args.browser_record,
             excluded_crates=["phone-android", "phone-ios", "phone-desktop", "phone-platform-desktop", "phone-native"],
             script_families={"shared", "browser"}, required=BROWSER_REQUIRED,
             workflow=".github/workflows/pages.yml", jobs=["build", "deploy"]),
        dict(name="desktop", tested_ref=args.desktop, record=args.desktop_record,
             excluded_crates=["phone-android", "phone-ios", "phone-browser"],
             script_families={"shared"}, required={"crates/phone-desktop/platform/tests/test_packaging.py"},
             workflow=".github/workflows/pages.yml", jobs=["native"]),
        dict(name="android_arm64_package", tested_ref=args.android_package, record=args.android_package_record,
             excluded_crates=["phone-ios", "phone-browser", "phone-desktop", "phone-platform-desktop"],
             script_families={"shared", "android_package"},
             required=ANDROID_PACKAGE_SCRIPTS | {"crates/phone-android/tests/java/org/ferroui/phone/PhoneContractTest.java", "crates/phone-android/platform/vendor/aosp-mms/UPSTREAM.json"},
             workflow=".github/workflows/android.yml", jobs=["android"]),
        dict(name="android_x86_64_ui", tested_ref=args.android_ui, record=args.android_ui_record,
             excluded_crates=["phone-ios", "phone-browser", "phone-desktop", "phone-platform-desktop"],
             script_families={"shared", "android_package", "android_ui"},
             required=ANDROID_PACKAGE_SCRIPTS | ANDROID_UI_SCRIPTS | {"crates/phone-android/tests/java/org/ferroui/phone/PhoneContractTest.java", "crates/phone-android/platform/vendor/aosp-mms/UPSTREAM.json"},
             workflow=".github/workflows/android-ui.yml", jobs=["android-ui"]),
        dict(name="ios", tested_ref=args.ios, record=args.ios_record,
             excluded_crates=["phone-android", "phone-browser", "phone-desktop", "phone-platform-desktop"],
             extra_files={"crates/phone-browser/wwwroot/icon-512.png"},
             script_families={"shared", "ios"}, required=IOS_REQUIRED,
             workflow=".github/workflows/ios.yml", jobs=["ios"]),
    ]


def scope_descriptor(scope: dict) -> dict:
    return {
        "target": scope["name"],
        "common_files": sorted(COMMON_FILES),
        "required_files": sorted(COMMON_REQUIRED | scope["required"] | {scope["workflow"]}),
        "extra_files": sorted(scope.get("extra_files", set())),
        "shared_roots": [".cargo/", "crates/"],
        "excluded_crate_roots": sorted("crates/" + name + "/" for name in scope["excluded_crates"]),
        "script_families": sorted(scope["script_families"]),
        "excluded_documentation": {"root": "docs/", "suffixes": list(DOC_SUFFIXES)},
        "excluded_scaffolding_scripts": sorted(EXCLUDED_DEVELOPMENT_SCRIPTS),
        "browser_root_files": ["npm-shrinkwrap.json", "package-lock.json", "package.json"] if scope["name"] == "browser" else [],
        "workflow": scope["workflow"], "selected_jobs": scope["jobs"],
        "workflow_normalization": WORKFLOW_NORMALIZATION,
        "local_actions_and_reusable_workflows": "rejected until their complete source-input closure is reviewed",
    }


def fingerprint(root: Path, ref: str, scope: dict, entries: dict | None = None) -> tuple[dict, dict]:
    entries = tree(root, ref) if entries is None else entries
    for path in entries:
        if path.startswith("scripts/") and not is_doc(path):
            script_family(path)
    required = COMMON_REQUIRED | scope["required"] | set(scope.get("extra_files", set())) | {scope["workflow"]}
    missing = sorted(required - entries.keys())
    require(not missing, f"{scope['name']}: missing required files at {ref}: {missing}")
    require(entries["FerroUI"][:2] == ("160000", "commit"), f"FerroUI must remain a pinned gitlink at {ref}")
    snapshot = {path: entry for path, entry in entries.items() if includes(path, scope)}
    projection, details = workflow_projection(root, ref, scope["workflow"], scope["jobs"])
    mode, kind, _oid = entries[scope["workflow"]]
    require(kind == "blob", f"{scope['name']}: selected workflow is not a file at {ref}")
    snapshot[scope["workflow"]] = (mode, kind + "+execution-json-sha256", sha256(compact(projection)))
    return snapshot, details


def assert_identical(old: dict, current: dict, target: str, tested: str) -> None:
    differences = [path for path in sorted(old.keys() | current.keys()) if old.get(path) != current.get(path)]
    require(not differences, f"{target}: source/configuration changed since tested {tested}: {differences}")


def reject_dirty_source(root: Path, scopes: list[dict]) -> None:
    dirty = set(git(root, "diff", "HEAD", "--name-only", "-z", "--ignore-submodules=none").decode().split("\0"))
    dirty |= set(git(root, "ls-files", "--others", "--exclude-standard", "-z").decode().split("\0"))
    dirty.discard("")
    material_dirty = sorted(path for path in dirty if any(
        includes(path, scope) or path == scope["workflow"] for scope in scopes))
    require(not material_dirty, f"Uncommitted target source/configuration cannot be attributed to HEAD: {material_dirty}")


def make_proof(root: Path, head: str, scopes: list[dict], *,
               recorded_at: str | None = None, generator: Path | None = None) -> dict:
    """Reconstruct every field from committed trees and current evidence bytes."""
    generator = Path(__file__) if generator is None else generator
    tested_refs = {scope["name"]: resolve_commit(root, scope["tested_ref"]) for scope in scopes}
    refs = {head, *tested_refs.values()}
    trees = {ref: tree(root, ref) for ref in refs}
    records = []
    for scope in scopes:
        tested = tested_refs[scope["name"]]
        old, before = fingerprint(root, tested, scope, trees[tested])
        current, after = fingerprint(root, head, scope, trees[head])
        assert_identical(old, current, scope["name"], tested)
        record_path = Path(scope["record"])
        if not record_path.is_absolute():
            record_path = root / EVIDENCE_DIR / record_path
        evidence = record_info(root, record_path, tested, scope["name"])
        changed_scheduling = [key for key in sorted(before["omitted_scheduling_fields"].keys() | after["omitted_scheduling_fields"].keys())
                              if before["omitted_scheduling_fields"].get(key) != after["omitted_scheduling_fields"].get(key)]
        descriptor = scope_descriptor(scope)
        records.append({
            "target": scope["name"], "tested_commit": tested, "implementation_head": head,
            "verification_records": [evidence],
            "identical_to_implementation_head": True,
            "tracked_entries": len(current),
            "source_configuration_sha256": sha256(compact(sorted(current.items()))),
            "compared_paths": sorted(current),
            "scope": descriptor, "scope_sha256": sha256(compact(descriptor)),
            "workflow": {"tested": before, "implementation": after,
                         "changed_scheduling_fields_only": changed_scheduling},
        })
    return {
        "schema": SCHEMA, "repository": REPOSITORY, "implementation_head": head,
        "framework_commit": trees[head]["FerroUI"][2],
        "recorded_at": recorded_at or dt.datetime.now(dt.timezone.utc).isoformat(),
        "generator": {"file": generator.name, "sha256": sha256(generator.read_bytes())},
        "method": "Compare target-specific committed git entries (path, file mode, type and blob/gitlink identity). SHA-256 covers sorted entries encoded as canonical compact JSON. Shared crates/assets/tests and target host, packaging, build, validation and smoke sources are included; the pinned FerroUI gitlink covers its framework/tooling. Each target's explicit scope and verification record bytes are hashed. Target-specific required execution gates and artifact source heads are validated before recording equality.",
        "workflow_normalization": WORKFLOW_NORMALIZATION,
        "workflow_exclusions": "Only event admission filters, top-level concurrency and selected jobs' top-level if conditions are excluded as scheduling. Dispatch/call contracts, steps/conditions, tool/action versions, runners/matrices, timeouts, environments/defaults and permissions remain compared. YAML node tags plus scalar styles and decoded lexical values (including mapping keys) are retained; comments and mapping order are ignored. Conservative style/quoting changes may require a new reviewed proof. Selected-job dependencies cannot refer to omitted jobs; local actions/reusable workflows fail closed pending explicit input-closure review. Each evidence record separately proves which gates executed.",
        "targets": records,
        "limits": "Source equality carries only the exact scope of the referenced compile/package/runtime evidence. It neither runs new tests nor qualifies physical carrier delivery, MMS APNs, OEM behavior, permissions, native session installation, hardware, release signing, existing-profile service-worker upgrades or original Windows Phone fidelity. Gate validation checks recorded evidence and hashes; it does not independently contact GitHub or rerun archived binaries. Overlapping local test counts are not combined.",
    }


def check_saved_proof(root: Path, output: Path, current: dict, scopes: list[dict], *,
                      generator: Path | None = None) -> dict:
    """Validate the saved proof itself, then compare its independently rebuilt head.

    A docs-only descendant may legitimately have a new HEAD. Never substitute
    that HEAD for the implementation commit written in the saved evidence.
    """
    require(output.is_file(), f"Missing saved source proof: {output}; --check-only never creates it")
    saved = strict_json(output.read_bytes(), str(output))
    require(saved.get("schema") == SCHEMA and saved.get("repository") == REPOSITORY,
            "Saved source proof has an unsupported schema or repository")
    saved_ref = saved.get("implementation_head")
    require(isinstance(saved_ref, str) and re.fullmatch(r"[0-9a-f]{40}", saved_ref) is not None,
            "Saved proof implementation_head is not a full commit")
    saved_head = resolve_commit(root, saved_ref)
    timestamp = saved.get("recorded_at")
    require(isinstance(timestamp, str), "Saved proof is missing its recording timestamp")
    try:
        parsed_time = dt.datetime.fromisoformat(timestamp)
    except ValueError as error:
        raise BoundaryError("Saved proof has an invalid recording timestamp") from error
    require(parsed_time.tzinfo is not None, "Saved proof timestamp must include its timezone")
    # This reconstructs scopes, paths, workflow projections, evidence bytes,
    # hashes, gate outcomes and the generator from their authoritative inputs.
    # Whole-document equality also rejects missing, duplicate or extra targets,
    # altered booleans, stale counts and unexpected proof fields.
    reconstructed = make_proof(root, saved_head, scopes, recorded_at=timestamp, generator=generator)
    require(saved == reconstructed,
            "Saved proof differs from reconstructed source/scopes/evidence/generator; regenerate only after reviewing the changed input")
    current_targets = {row["target"]: row for row in current["targets"]}
    require(set(current_targets) == {row["target"] for row in saved["targets"]}, "Saved/current target coverage differs")
    for recorded in saved["targets"]:
        latest = current_targets[recorded["target"]]
        for field in ("tested_commit", "verification_records", "scope", "scope_sha256",
                      "source_configuration_sha256", "tracked_entries", "compared_paths"):
            require(recorded[field] == latest[field], f"{recorded['target']}: saved/current {field} differs")
    return saved


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--repo", type=Path, default=Path(__file__).resolve().parent / "WindowsPhone")
    parser.add_argument("--check-only", action="store_true",
                        help="Require and reconstruct saved proof, scopes, evidence and generator; verify current target equality without writing")
    parser.add_argument("--browser", required=True, help="Final tested browser build/rendered-suite commit")
    parser.add_argument("--desktop", required=True, help="Final tested desktop build/protocol commit")
    parser.add_argument("--android-package", required=True, help="Final tested Android ARM64 package commit")
    parser.add_argument("--android-ui", required=True, help="Final tested Android emulator/UI commit")
    parser.add_argument("--ios", required=True, help="Final tested iOS build/simulator commit")
    parser.add_argument("--browser-record", default="browser.json")
    parser.add_argument("--desktop-record", default="desktop.json")
    parser.add_argument("--android-package-record", default="android-package.json")
    parser.add_argument("--android-ui-record", default="android-ui.json")
    parser.add_argument("--ios-record", default="ios.json")
    args = parser.parse_args()
    root = args.repo.resolve()
    head = resolve_commit(root, "HEAD")
    output = root / EVIDENCE_DIR / "source-boundaries.json"
    scopes = scope_table(args)
    reject_dirty_source(root, scopes)
    proof = make_proof(root, head, scopes)
    if args.check_only:
        saved = check_saved_proof(root, output, proof, scopes)
        print(json.dumps({"current_head": head, "recorded_implementation_head": saved["implementation_head"],
                          "mode": "read-only verification", "saved_proof_valid": True,
                          "source_equality": {row["target"]: True for row in proof["targets"]}}, indent=2))
        return
    output.parent.mkdir(parents=True, exist_ok=True)
    encoded = json.dumps(proof, indent=2, ensure_ascii=True, allow_nan=False) + "\n"
    temp_path = None
    try:
        with tempfile.NamedTemporaryFile(mode="w", encoding="utf-8", dir=output.parent,
                                         prefix=".source-boundaries-", suffix=".tmp", delete=False) as handle:
            temp_path = Path(handle.name)
            handle.write(encoded)
            handle.flush()
            os.fsync(handle.fileno())
        os.replace(temp_path, output)
    finally:
        if temp_path is not None and temp_path.exists():
            temp_path.unlink()
    print(json.dumps({"implementation_head": head, "output": str(output),
                      "source_equality": {row["target"]: True for row in proof["targets"]}}, indent=2))


if __name__ == "__main__":
    try:
        main()
    except (BoundaryError, OSError, ValueError, RecursionError) as error:
        print(f"Source boundary verification failed: {error}", file=sys.stderr)
        raise SystemExit(1)
