#!/usr/bin/env python3
"""Validate a fresh Android observation of a mounted, inset phone and real URI route."""
import argparse
import json
import math
from pathlib import Path
import sys


def require(condition, message):
    if not condition:
        raise ValueError(message)


def number(value):
    require(type(value) in (int, float) and math.isfinite(value), "Non-finite geometry")
    return value


def rectangle(value):
    result = tuple(number(value[key]) for key in ("x", "y", "width", "height"))
    require(result[2] > 0 and result[3] > 0, "A mounted control has empty bounds")
    return result


def near(actual, expected, message):
    require(len(actual) == len(expected) and all(abs(a-b) <= 1.0 for a, b in zip(actual, expected)), message)


def verify(proof, nonce, uri, after_sequence):
    require(proof["schema"] == 1 and proof["platform"] == "android", "Wrong observation schema/platform")
    require(proof["nonce"] == nonce and len(nonce) == 32, "Observation belongs to another smoke run")
    require(type(proof["sequence"]) is int and proof["sequence"] > after_sequence, "Observation is stale")
    require(type(proof["process_id"]) is int and proof["process_id"] > 0, "Missing application process")
    require(proof["observed_at_unix_ms"] > 0, "Missing observation timestamp")
    require(proof["ready"] is True and proof["closed"] is False, "Host is not mounted with valid layout")
    require(proof["host"]["mode"] == "app" and proof["host"]["launcher"] != "active"
            and proof["host"]["shell"] == "unsupported", "The smoke launch must not acquire a HOME or shell role")
    require(proof["activation"]["uri"] == uri and proof["activation"]["outcome"] == "accepted",
            "The real activity did not accept the requested URI")
    if uri == "ferroui-phone://home":
        require(proof["route"] == "Start", "The mounted route has not returned to Start")
        expected_control = "StartTiles"
    elif uri == "ferroui-phone://app/org.ferroui.wallet":
        route = proof["route"]
        require(isinstance(route, dict) and route.get("Extension", {}).get("app") == "org.ferroui.wallet"
                and route["Extension"].get("path") == "", "The mounted route is not the Wallet root")
        expected_control = "WalletSearch"
    else:
        raise ValueError("This gate only verifies Home and Wallet activations")
    require({"PhoneScreen", "PhonePage", expected_control} <= set(proof["mounted_controls"]),
            "The requested application's real controls are not mounted and visible")
    client = (number(proof["client"]["width"]), number(proof["client"]["height"]))
    require(client[0] >= 280 and client[1] >= 400, "Unexpected phone client size")
    require(number(proof["render_scaling"]) > 0, "Missing native render scale")
    inset = tuple(number(proof["safe_area"][key]) for key in ("left", "top", "right", "bottom"))
    require(all(v >= 0 for v in inset) and sum(inset) > 0, "The emulator did not exercise system safe-area insets")
    content = (inset[0], inset[1], client[0]-inset[0]-inset[2], client[1]-inset[1]-inset[3])
    require(content[2] >= 280 and content[3] >= 280, "System insets leave no usable phone surface")
    near(rectangle(proof["wrapper"]), (0, 0, *client), "The outer safe-area container does not fill the native client")
    near(rectangle(proof["inner"]), content, "The measured inner host does not match the inset content area")
    near(rectangle(proof["screen"]), content, "PhoneScreen overflows or undersizes the inset content area")
    viewport = (number(proof["viewport"]["width"]), number(proof["viewport"]["height"]))
    near(viewport, content[2:], "The VM viewport includes system padding")
    return proof["sequence"]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("observation", type=Path)
    parser.add_argument("nonce")
    parser.add_argument("uri")
    parser.add_argument("after_sequence", type=int)
    args = parser.parse_args()
    try:
        proof = json.loads(args.observation.read_text(encoding="utf-8"))
        print(verify(proof, args.nonce, args.uri, args.after_sequence))
    except (OSError, ValueError, KeyError, TypeError) as error:
        print(str(error), file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
