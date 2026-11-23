#!/usr/bin/env python3
"""Generate reviewable session files. Generation never installs or starts a session."""
from __future__ import annotations

import argparse
import hashlib
from pathlib import Path
import shlex

HERE = Path(__file__).resolve().parent
ASSETS = {
    "usr/local/libexec/windowsphone/windowsphone-session": ("windowsphone-session", 0o755),
    "usr/share/wayland-sessions/windowsphone.desktop": ("windowsphone.desktop", 0o644),
    "usr/local/share/windowsphone/labwc/rc.xml": ("labwc/rc.xml", 0o644),
    "usr/local/share/windowsphone/labwc/menu.xml": ("labwc/menu.xml", 0o644),
}


def stage(executable: Path, output: Path) -> None:
    executable = executable.resolve(strict=True)
    if not executable.is_file() or not executable.stat().st_mode & 0o111:
        raise ValueError("The installed phone-desktop path must be an executable file.")
    if any(ord(character) < 32 or ord(character) == 127 for character in str(executable)):
        raise ValueError("The executable path contains a control character.")
    if not output.is_absolute() or output == Path("/"):
        raise ValueError("Choose a new absolute staging directory.")
    # An exclusive new directory makes generation non-destructive and prevents
    # an existing/symlinked staged file from redirecting a write.
    output.mkdir(parents=True, exist_ok=False)
    entries = {}
    for destination, (source, mode) in ASSETS.items():
        entries[destination] = ((HERE / source).read_bytes(), mode)
    entries["usr/local/libexec/windowsphone/windowsphone-session-client"] = (
        ("#!/bin/sh\nset -eu\nexec " + shlex.quote(str(executable)) + " --mode shell --backend wayland\n").encode(),
        0o755,
    )
    for destination, (content, mode) in entries.items():
        target = output / destination
        target.parent.mkdir(parents=True, exist_ok=True)
        target.write_bytes(content)
        target.chmod(mode)
    hashes = "".join(f"{hashlib.sha256(content).hexdigest()}  /{destination}\n" for destination, (content, _) in entries.items())
    (output / "installed.sha256").write_text(hashes)
    install = [
        "#!/bin/sh", "set -eu", 'cd -- "$(dirname -- "$0")"',
        'if [ "${1-}" != "--install" ] || [ "$#" -ne 1 ]; then',
        "  printf '%s\\n' 'Review the staged files, then run this script as administrator with --install.'",
        "  printf '%s\\n' 'It adds a login-session choice; it does not change the default session or start it.'",
        "  exit 0", "fi",
        '[ "$(id -u)" -eq 0 ] || { printf "%s\\n" "Administrator privileges are required to install session files." >&2; exit 1; }',
    ]
    for destination in entries:
        path = shlex.quote("/" + destination)
        install.append(f"[ ! -e {path} ] && [ ! -L {path} ] || {{ printf '%s\\n' 'Refusing to replace an existing session file: /{destination}' >&2; exit 1; }}")
    for destination, (_, mode) in entries.items():
        install.append(f"install -D -m {mode:o} -- {shlex.quote(destination)} {shlex.quote('/' + destination)}")
    install.append("printf '%s\\n' 'Installed. Sign out and explicitly choose Windows Phone at the login screen. Keep this staging directory for removal.'")
    (output / "install.sh").write_text("\n".join(install) + "\n")
    (output / "install.sh").chmod(0o755)
    remove = [
        "#!/bin/sh", "set -eu", 'cd -- "$(dirname -- "$0")"',
        '[ "${1-}" = "--remove" ] && [ "$#" -eq 1 ] || { printf "%s\\n" "Run as administrator with --remove to remove these exact, unchanged session files."; exit 0; }',
        '[ "$(id -u)" -eq 0 ] || { printf "%s\\n" "Administrator privileges are required." >&2; exit 1; }',
        "# Refuse to remove files changed after this staging set was generated.",
        "sha256sum --check installed.sha256",
    ]
    remove.extend(f"rm -- {shlex.quote('/' + destination)}" for destination in entries)
    remove.append("printf '%s\\n' 'Removed the Windows Phone session choice. Existing desktop sessions were unchanged.'")
    (output / "remove.sh").write_text("\n".join(remove) + "\n")
    (output / "remove.sh").chmod(0o755)


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--executable", type=Path, required=True, help="Absolute installed phone-desktop executable path")
    parser.add_argument("--output", type=Path, required=True, help="New absolute staging directory")
    args = parser.parse_args()
    try:
        stage(args.executable, args.output)
    except (OSError, ValueError) as error:
        parser.error(str(error))
    print(f"Generated {args.output}. Review the files before invoking install.sh --install on the intended Linux device.")


if __name__ == "__main__":
    main()
