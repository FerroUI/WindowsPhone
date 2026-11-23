#!/usr/bin/env python3
"""Keep the clean, pinned framework fresh against restored Cargo fingerprints."""
import os
from pathlib import Path
import stat
import subprocess
import sys

root = Path(sys.argv[1] if len(sys.argv) > 1 else Path(__file__).resolve().parents[1] / "FerroUI").resolve()


def git(*args):
    return subprocess.check_output(["git", "-C", str(root), *args])


if subprocess.run(["git", "-C", str(root), "diff", "--quiet", "HEAD", "--"]).returncode:
    raise SystemExit("Refusing to normalize modified framework sources; use the pinned clean checkout.")

timestamp = int(git("show", "-s", "--format=%ct", "HEAD")) * 1_000_000_000
directories = {root}
count = 0
skipped_links = 0
can_touch_link = os.utime in os.supports_follow_symlinks
for entry in git("ls-files", "--stage", "-z").split(b"\0"):
    if not entry:
        continue
    metadata, name = entry.split(b"\t", 1)
    if metadata.startswith(b"160000 "):
        continue  # A gitlink belongs to a separate checkout.
    file = root / os.fsdecode(name)
    mode = file.lstat().st_mode
    if not (stat.S_ISREG(mode) or stat.S_ISLNK(mode)):
        continue
    if stat.S_ISLNK(mode):
        # Windows Python cannot change a link's timestamps without following it.
        # Leave that entry alone instead of touching a target outside the checkout.
        if not can_touch_link:
            skipped_links += 1
            continue
        os.utime(file, ns=(timestamp, timestamp), follow_symlinks=False)
    else:
        os.utime(file, ns=(timestamp, timestamp))
    count += 1
    parent = file.parent
    while parent != root:
        directories.add(parent)
        parent = parent.parent

# Theme build scripts also watch directories, so their mtimes must be stable.
for directory in directories:
    os.utime(directory, ns=(timestamp, timestamp))
print(f"Normalized {count} pinned framework files and {len(directories)} directories.")
if skipped_links:
    print(f"Skipped {skipped_links} symbolic links: this platform cannot timestamp links safely.")
