#!/usr/bin/env python3
"""Validate an observation of the mounted UIKit phone; never drive its state."""
import json
import math
import pathlib
import sys


def verify(data, expected, after_sequence, uri=""):
    if data.get("schema") != 1 or data.get("platform") != "ios":
        raise ValueError("Expected an iOS observation with schema 1.")
    if not data.get("ready") or data.get("closed"):
        raise ValueError("The phone must remain attached with valid measure and arrangement.")
    if data.get("sequence", 0) <= after_sequence:
        raise ValueError("The observation must be newer than the URL request.")
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
    pages = {
        "home": {"StartTiles"},
        "org.ferroui.wallet": {"WalletSearch"},
        "phone.phone": {"DevicePhoneNumber"},
        "phone.people": {"DevicePeopleSearch"},
        "phone.messaging": {"DeviceMessageRecipients", "DeviceMessageBody"},
        "phone.settings": {"DeviceSettingsRoot"},
    }
    if expected not in pages:
        raise ValueError("No mounted-control contract exists for this route.")
    required = {"PhoneScreen", "PhonePage", *pages[expected]}
    if not required <= controls:
        raise ValueError(f"Expected visible mounted controls: {sorted(required)}.")
    route = data["route"]
    if expected == "home":
        matches = route == "Start"
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


if __name__ == "__main__":
    try:
        verify(json.loads(pathlib.Path(sys.argv[1]).read_text()), sys.argv[2], int(sys.argv[3]), sys.argv[4])
    except (OSError, ValueError, KeyError, TypeError) as error:
        if "--explain" in sys.argv[5:]:
            print(error, file=sys.stderr)
        raise SystemExit(1)
