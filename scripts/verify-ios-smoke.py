#!/usr/bin/env python3
"""Validate an observation of the mounted UIKit phone; never drive its state."""
import argparse
import json
import math
import pathlib
import sys
import uuid


def rectangle(value, label):
    if not isinstance(value, dict) or not all(
        isinstance(value.get(key), (int, float)) and math.isfinite(value[key])
        for key in ("x", "y", "width", "height")
    ):
        raise ValueError(f"{label} must have finite top-level bounds.")
    return value


def contains(outer, inner, tolerance=1):
    return (inner["x"] >= outer["x"] - tolerance and inner["y"] >= outer["y"] - tolerance
            and inner["x"] + inner["width"] <= outer["x"] + outer["width"] + tolerance
            and inner["y"] + inner["height"] <= outer["y"] + outer["height"] + tolerance)


def verify_navigation_controls(data):
    names = ("HardwareBack", "HardwareStart", "HardwareSearch")
    controls = data.get("navigation_controls")
    if (not isinstance(controls, list) or len(controls) != 3 or not all(isinstance(item, dict) for item in controls)
            or {item.get("name") for item in controls} != set(names)):
        raise ValueError("The mounted navigation row must contain Back, Start and Search exactly once.")
    screen = rectangle(data["screen"], "Phone screen")
    previous_right = screen["x"]
    for name in names:
        control = next(item for item in controls if item["name"] == name)
        if any(control.get(key) is not True for key in ("visible", "enabled", "hit")):
            raise ValueError(f"{name} must be visible, enabled and hit-testable at its center.")
        opacity = control.get("opacity", 0)
        if not isinstance(opacity, (int, float)) or not math.isfinite(opacity) or opacity < 0.99:
            raise ValueError(f"{name} must retain its visible navigation ink.")
        bounds = rectangle(control.get("bounds"), name)
        if (bounds["width"] < 44 or abs(bounds["height"] - 54) > 1
                or not contains(screen, bounds) or abs(bounds["x"] - previous_right) > 1
                or abs(bounds["y"] + bounds["height"] - screen["y"] - screen["height"]) > 1):
            raise ValueError(f"{name} must occupy its usable bottom navigation cell inside the safe area.")
        previous_right = bounds["x"] + bounds["width"]
        glyph = control.get("glyph")
        if not isinstance(glyph, dict) or glyph.get("visible") is not True or glyph.get("has_geometry") is not True:
            raise ValueError(f"{name} must contain its visible vector glyph.")
        ink = rectangle(glyph.get("bounds"), f"{name} glyph")
        opacity = glyph.get("opacity", 0)
        if (ink["width"] < 10 or ink["height"] < 10 or not contains(bounds, ink)
                or not isinstance(opacity, (int, float)) or not math.isfinite(opacity) or opacity < 0.99):
            raise ValueError(f"{name} glyph must remain opaque and inside its navigation button.")
    if abs(previous_right - screen["x"] - screen["width"]) > 1:
        raise ValueError("The navigation row must span the safe phone width.")


def verify(data, expected, after_sequence, uri="", baseline=None):
    if data.get("schema") != 1 or data.get("platform") != "ios":
        raise ValueError("Expected an iOS observation with schema 1.")
    if not data.get("ready") or data.get("closed"):
        raise ValueError("The phone must remain attached with valid measure and arrangement.")
    if data.get("sequence", 0) <= after_sequence:
        raise ValueError("The observation must be newer than the URL request.")
    native = data.get("native", {})
    if not isinstance(native, dict):
        raise ValueError("The observation must include read-only native identity diagnostics.")
    if native.get("bundle_id") != "org.ferroui.windowsphone":
        raise ValueError("The observed native process must be the installed phone application.")
    if type(native.get("pid")) is not int or native["pid"] <= 0:
        raise ValueError("The observation must contain the actual native process ID.")
    launch_id = native.get("launch_id")
    if not isinstance(launch_id, str) or len(launch_id) != 36 or str(uuid.UUID(launch_id)).lower() != launch_id.lower():
        raise ValueError("The observation must contain its native per-launch UUID.")
    # UIKit's UIApplicationStateActive and UISceneActivationStateForegroundActive
    # are both zero. Attached controls alone can also exist in a background scene.
    if native.get("application_state") != 0 or not any(
        scene.get("activation_state") == 0 for scene in native.get("scenes", [])
    ):
        raise ValueError("The phone application and a connected scene must be foreground-active.")
    if uri and baseline is None:
        raise ValueError("Warm activation requires the original Start process observation.")
    if baseline is not None:
        if any(native[key] != baseline.get("native", {}).get(key) for key in ("pid", "launch_id", "bundle_id")):
            raise ValueError("Warm activation must preserve the original phone process and launch.")
        if data.get("observed_at_unix_ms", 0) <= baseline.get("observed_at_unix_ms", 0):
            raise ValueError("Warm activation must have a fresh observation from the same process.")
        def active_sessions(value):
            return {scene.get("session_id") for scene in value.get("native", {}).get("scenes", [])
                    if scene.get("activation_state") == 0 and isinstance(scene.get("session_id"), str) and scene["session_id"]}
        if not active_sessions(data) & active_sessions(baseline):
            raise ValueError("Native navigation must preserve the original foreground scene session.")
    client, inset = data["client"], data["safe_area"]
    values = [client["width"], client["height"], *inset.values()]
    if not all(isinstance(value, (int, float)) and math.isfinite(value) and value >= 0 for value in values):
        raise ValueError("Client and safe-area values must be finite and nonnegative.")
    safe = {"x": inset["left"], "y": inset["top"],
            "width": client["width"] - inset["left"] - inset["right"],
            "height": client["height"] - inset["top"] - inset["bottom"]}
    for name in ("inner", "screen"):
        rect = data[name]
        if not all(isinstance(rect[key], (int, float)) and math.isfinite(rect[key]) for key in ("x", "y", "width", "height")):
            raise ValueError(f"{name} must have finite top-level bounds.")
        if rect["width"] < 300 or rect["height"] < 400:
            raise ValueError(f"{name} must have usable mounted dimensions.")
        if (rect["x"] < safe["x"] - 1 or rect["y"] < safe["y"] - 1
                or rect["x"] + rect["width"] > safe["x"] + safe["width"] + 1
                or rect["y"] + rect["height"] > safe["y"] + safe["height"] + 1):
            raise ValueError(f"{name} exceeds the UIKit safe area by more than 1 point.")
    controls = set(data["mounted_controls"])
    verify_navigation_controls(data)
    pages = {
        "home": {"StartTiles"},
        "org.ferroui.wallet": {"WalletSearch"},
        "phone.phone": {"DevicePhoneNumber"},
        "phone.people": {"PeoplePanorama", "DevicePeoplePane"},
        "phone.messaging": {"DeviceMessageRecipients", "DeviceMessageBody"},
        "phone.settings": {"DeviceSettingsRoot"},
        "search": {"CortanaInput"},
    }
    if expected not in pages:
        raise ValueError("No mounted-control contract exists for this route.")
    required = {"PhoneScreen", "PhonePage", *pages[expected]}
    if not required <= controls:
        raise ValueError(f"Expected visible mounted controls: {sorted(required)}.")
    if expected == "phone.people":
        panorama = data.get("people_panorama")
        if (not isinstance(panorama, dict) or panorama.get("selected_index") != 2
                or panorama.get("swiping") is not False
                or panorama.get("device_visible") is not True
                or panorama.get("device_enabled") is not True):
            raise ValueError("People activation must settle on the real enabled device-contact section.")
        pane = rectangle(panorama.get("device_bounds"), "Device contact pane")
        if pane["width"] < 200 or pane["height"] < 100 or not contains(data["screen"], pane):
            raise ValueError("The selected device-contact pane must fit inside the safe phone screen.")
    route = data["route"]
    if expected == "home":
        matches = route == "Start"
    elif expected == "search":
        matches = route == "Search"
    elif expected == "phone.settings":
        matches = route == {"Setting": "device services"}
    elif expected in ("phone.phone", "phone.people", "phone.messaging"):
        matches = route == {"App": expected.split(".")[1].capitalize()}
    else:
        matches = isinstance(route, dict) and route.get("Extension", {}).get("app") == expected
    if not matches:
        raise ValueError(f"The mounted route is not {expected}.")
    if uri and data.get("activation") != {"uri": uri, "outcome": "accepted"}:
        raise ValueError("The real UIKit delegate must accept the exact requested URI.")
    if expected == "phone.phone" and data.get("device_fields", {}).get("DevicePhoneNumber") != "+1 202 555 0147":
        raise ValueError("The native activation must populate the actual dialer field without placing a call.")


def verify_navigation(data, expected, request, driver, baseline):
    if baseline is None:
        raise ValueError("Navigation taps require the retained restored-Start baseline.")
    sequence = request.get("sequence")
    allowed = ("HardwareSearch", "HardwareBack", "HardwareSearch", "HardwareStart")
    if type(sequence) is not int or not 1 <= sequence <= len(allowed) or request.get("control") != allowed[sequence - 1]:
        raise ValueError("The driver must perform the fixed Search, Back, Search, Start sequence.")
    if expected != ("search" if request["control"] == "HardwareSearch" else "home"):
        raise ValueError("The expected route must match the navigation button action.")
    verify(data, expected, request["after_sequence"], baseline=baseline)
    if data.get("activation") != baseline.get("activation"):
        raise ValueError("A navigation tap must not be replaced by another native URL activation.")
    if data["observed_at_unix_ms"] <= request["issued_at_unix_ms"]:
        raise ValueError("The navigation observation must follow the real tap request.")
    taps = driver.get("navigation", [])
    if (driver.get("navigation_driver") != "XCUIElement.tap" or len(taps) != sequence
            or [tap.get("control") for tap in taps] != list(allowed[:sequence])
            or taps[-1].get("sequence") != sequence):
        raise ValueError("XCTest must acknowledge the exact real navigation tap.")
    tap = taps[-1]
    if tap.get("tap_completed_at_unix_ms", 0) < request["issued_at_unix_ms"]:
        raise ValueError("The acknowledged navigation tap must follow its request.")
    if tap.get("bright_opaque_pixels", 0) < 24:
        raise ValueError("The XCTest button screenshot must retain visible navigation ink.")


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("observation", type=pathlib.Path)
    parser.add_argument("route")
    parser.add_argument("after_sequence", type=int)
    parser.add_argument("uri")
    parser.add_argument("--baseline", default="")
    parser.add_argument("--navigation-request", default="")
    parser.add_argument("--driver", default="")
    parser.add_argument("--explain", action="store_true")
    arguments = parser.parse_args()
    try:
        baseline = json.loads(pathlib.Path(arguments.baseline).read_text()) if arguments.baseline else None
        data = json.loads(arguments.observation.read_text())
        if arguments.navigation_request:
            request = json.loads(pathlib.Path(arguments.navigation_request).read_text())
            driver = json.loads(pathlib.Path(arguments.driver).read_text())
            verify_navigation(data, arguments.route, request, driver, baseline)
        else:
            verify(data, arguments.route, arguments.after_sequence, arguments.uri, baseline)
    except (OSError, ValueError, KeyError, TypeError) as error:
        if arguments.explain:
            print(error, file=sys.stderr)
        raise SystemExit(1)
