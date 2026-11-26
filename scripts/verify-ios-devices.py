#!/usr/bin/env python3
"""Require evidence from real provider operations on the disposable simulator."""
import json
import pathlib
import sys


def verify(data):
    if (data.get("schema") != 1 or data.get("platform") != "ios"
            or data.get("status") != "passed" or data.get("simulator_only") is not True):
        raise ValueError("The isolated native provider roundtrip has not passed.")
    if data.get("error") is not None or data.get("cleanup_errors") != []:
        raise ValueError("The roundtrip or its restoration failed.")
    if data.get("sent_calls") != 0 or data.get("sent_messages") != 0:
        raise ValueError("Provider qualification must never place calls or send messages.")
    checks = data.get("checks", [])
    expected = {"observed_simulator_capabilities", "brightness_read_only", "contact_create_and_query",
                "contact_update_and_query", "contact_delete_and_query", "location_authorization_observed",
                "network_path_observed"}
    if len(checks) != len(expected) or {check.get("name") for check in checks} != expected:
        raise ValueError("The native provider evidence is incomplete.")
    if not all(check.get("passed") is True for check in checks):
        raise ValueError("A native provider assertion failed.")
    authorization = next(check for check in checks if check["name"] == "location_authorization_observed")
    if authorization.get("status") != "prompt":
        raise ValueError("The fresh simulator must observe unrequested location authorization.")
    network = next(check for check in checks if check["name"] == "network_path_observed")
    if type(network.get("online")) is not bool:
        raise ValueError("The native network path must be observed, without requiring Internet reachability.")


if __name__ == "__main__":
    try:
        verify(json.loads(pathlib.Path(sys.argv[1]).read_text()))
    except (OSError, ValueError, KeyError, TypeError):
        raise SystemExit(1)
