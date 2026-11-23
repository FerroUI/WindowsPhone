#!/usr/bin/env python3
"""Generate a per-account Shell Launcher v2 policy; never apply it automatically."""
from __future__ import annotations

import argparse
from pathlib import Path, PureWindowsPath
import shutil
import uuid
import xml.etree.ElementTree as ET

NS = "http://schemas.microsoft.com/ShellLauncher/2018/Configuration"
V2 = "http://schemas.microsoft.com/ShellLauncher/2019/Configuration"
ET.register_namespace("", NS)
ET.register_namespace("V2", V2)


def configuration(executable: str, account: str) -> bytes:
    path = PureWindowsPath(executable)
    if (not path.is_absolute() or len(path.drive) != 2 or path.drive[1] != ":"
            or path.suffix.lower() != ".exe" or ".." in path.parts
            or any(":" in part or part.endswith((".", " ")) or PureWindowsPath(part).is_reserved() for part in path.parts[1:])
            or any(character in executable for character in '\"<>|?*%\r\n\0')
            or any(ord(character) < 32 or ord(character) == 127 for character in executable)
            or len(executable) > 1000):
        raise ValueError("Provide a literal absolute local .exe path, without variables, quotes, wildcards or traversal.")
    if (not account.strip() or len(account) > 256
            or any(ord(character) < 32 or ord(character) == 127 for character in account)):
        raise ValueError("Provide the existing dedicated account name (for example .\\PhoneUser).")
    profile_id = "{" + str(uuid.uuid5(uuid.NAMESPACE_URL, "https://github.com/FerroUI/WindowsPhone/shell/" + account)) + "}"
    root = ET.Element(f"{{{NS}}}ShellLauncherConfiguration")
    profiles = ET.SubElement(root, f"{{{NS}}}Profiles")
    fallback = ET.SubElement(profiles, f"{{{NS}}}DefaultProfile")
    ET.SubElement(fallback, f"{{{NS}}}Shell", {"Shell": "%SystemRoot%\\explorer.exe"})
    profile = ET.SubElement(profiles, f"{{{NS}}}Profile", {"Id": profile_id, "Name": "Windows Phone"})
    shell = ET.SubElement(profile, f"{{{NS}}}Shell", {
        "Shell": f'"{path}" --mode shell',
        f"{{{V2}}}AppType": "Desktop", f"{{{V2}}}AllAppsFullScreen": "false",
    })
    actions = ET.SubElement(shell, f"{{{NS}}}ReturnCodeActions")
    ET.SubElement(actions, f"{{{NS}}}ReturnCodeAction", {"ReturnCode": "0", "Action": "RestartShell"})
    # A failed capability/startup check must not become an unlimited restart loop.
    ET.SubElement(shell, f"{{{NS}}}DefaultAction", {"Action": "DoNothing"})
    config = ET.SubElement(ET.SubElement(root, f"{{{NS}}}Configs"), f"{{{NS}}}Config")
    ET.SubElement(config, f"{{{NS}}}Account", {"Name": account})
    ET.SubElement(config, f"{{{NS}}}Profile", {"Id": profile_id})
    ET.indent(root, space="  ")
    return ET.tostring(root, encoding="utf-8", xml_declaration=True) + b"\n"


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--executable", required=True)
    parser.add_argument("--account", required=True)
    parser.add_argument("--output", type=Path, required=True, help="New directory for the XML and explicit deployment script")
    args = parser.parse_args()
    try:
        xml = configuration(args.executable, args.account)
        args.output.mkdir(parents=True, exist_ok=False)
        (args.output / "ShellLauncher.xml").write_bytes(xml)
        shutil.copyfile(Path(__file__).with_name("ShellLauncher.ps1"), args.output / "ShellLauncher.ps1")
    except (OSError, ValueError) as error:
        parser.error(str(error))
    print(f"Generated {args.output}. Inspect the XML and deployment script; generation changes no Windows policy.")


if __name__ == "__main__":
    main()
