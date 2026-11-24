#!/usr/bin/env python3
"""Require a provisioned, stable, resolvable HOME before taking a role baseline."""
import argparse
from pathlib import Path
import re
import sys


FIELDS = ("user", "device-provisioned", "user-setup-complete", "holders",
          "resolved", "activities", "holders-after", "user-after")
PACKAGE = r"[A-Za-z0-9_]+(?:\.[A-Za-z0-9_]+)+"
COMPONENT = re.compile(rf"({PACKAGE})/(\.?[A-Za-z0-9_$]+(?:\.[A-Za-z0-9_$]+)*)")


def require(condition, message):
    if not condition:
        raise ValueError(message)


def nonempty_lines(value):
    return [line.strip() for line in value.splitlines() if line.strip()]


def component(value):
    match = COMPONENT.fullmatch(value)
    require(match is not None, "HOME component is missing or malformed")
    package, activity = match.groups()
    return package, package + activity if activity.startswith(".") else activity


def identity(sample, user):
    require(user.isdecimal() and sample["user"].strip() == user
            and sample["user-after"].strip() == user, "Android user changed during HOME sampling")
    require(sample["device-provisioned"].strip() == "1"
            and sample["user-setup-complete"].strip() == "1", "Android provisioning is incomplete")
    holders = nonempty_lines(sample["holders"])
    require(len(holders) == 1 and re.fullmatch(PACKAGE, holders[0]) is not None,
            "HOME must have one authoritative package holder")
    require(nonempty_lines(sample["holders-after"]) == holders,
            "HOME holder changed during sampling")
    resolved = nonempty_lines(sample["resolved"])
    require(len(resolved) == 1, "HOME did not resolve to exactly one component")
    selected = component(resolved[0])
    require(selected[0] == holders[0], "Resolved HOME does not belong to its role holder")
    # query-activities --components uses ordinary flags, so disabled components
    # are excluded. Do not infer eligibility from a package name or a role alone.
    activities = {component(line) for line in nonempty_lines(sample["activities"])}
    require(selected in activities, "HOME holder has no enabled matching MAIN/HOME activity")
    return user, holders[0], selected[1]


def verify_stable(previous, current, user):
    before, after = identity(previous, user), identity(current, user)
    require(before == after, "HOME holder or resolved activity is still changing")
    return after[1]


def read_sample(directory):
    result = {}
    for name in FIELDS:
        require((directory / (name + ".status")).read_text().strip() == "0",
                f"Android HOME query failed: {name}")
        result[name] = (directory / (name + ".txt")).read_text()
    return result


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("previous", type=Path)
    parser.add_argument("current", type=Path)
    parser.add_argument("user")
    args = parser.parse_args()
    try:
        require(args.previous.resolve() != args.current.resolve(), "Two distinct HOME samples are required")
        print(verify_stable(read_sample(args.previous), read_sample(args.current), args.user))
    except (OSError, ValueError, KeyError, TypeError) as error:
        print(str(error), file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
