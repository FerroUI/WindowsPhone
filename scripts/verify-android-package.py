#!/usr/bin/env python3
"""Check the actual APK boundary, without installing it or selecting a HOME app."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import shutil
import struct
import subprocess
import tempfile
import xml.etree.ElementTree as ET
import zipfile

A = "{http://schemas.android.com/apk/res/android}"


def require(condition, message):
    if not condition:
        raise ValueError(message)


def verify_manifest(root, launcher):
    package = "org.ferroui.windowsphone" + (".launcher" if launcher else "")
    require(root.get("package") == package, "Unexpected application ID")
    sdk = root.find("uses-sdk")
    require(sdk is not None and sdk.get(A + "minSdkVersion") == "26"
            and sdk.get(A + "targetSdkVersion") == "36", "SDK levels do not match the framework pins")
    permissions = {p.get(A + "name") for p in root.findall("uses-permission")}
    require("android.permission.QUERY_ALL_PACKAGES" not in permissions, "Unscoped package visibility is forbidden")
    require({"android.permission.FOREGROUND_SERVICE", "android.permission.FOREGROUND_SERVICE_MEDIA_PLAYBACK"}
            <= permissions, "Media foreground service permissions are missing")
    app = root.find("application")
    require(app is not None and app.get(A + "name") == "org.ferroui.android.FerroApplication",
            "The APK must use the FerroUI application lifetime")
    require(app.get(A + "usesCleartextTraffic") == "false", "Unexpected cleartext network opt-in")
    require(any(v.get(A + "name") == "org.ferroui.android.library" and v.get(A + "value") == "phone_android"
                for v in app.findall("meta-data")), "The FerroUI native library metadata is missing")
    activity = next((v for v in app.findall("activity")
                     if v.get(A + "name") == "org.ferroui.phone.PhoneActivity"), None)
    require(activity is not None and activity.get(A + "launchMode") == "singleTask", "Missing single-task activity")
    categories = {c.get(A + "name") for f in activity.findall("intent-filter") for c in f.findall("category")}
    require("android.intent.category.LAUNCHER" in categories, "The package needs an application entry")
    require(("android.intent.category.HOME" in categories) == launcher, "HOME role leaked across package flavors")
    service = next((v for v in app.findall("service")
                    if v.get(A + "name") == "org.ferroui.phone.PhoneMediaService"), None)
    require(service is not None and service.get(A + "exported") == "false"
            and service.get(A + "foregroundServiceType") == "mediaPlayback", "Invalid media service boundary")
    provider = next((v for v in app.findall("provider")
                     if v.get(A + "name") == "org.ferroui.phone.PhoneAssetProvider"), None)
    require(provider is not None and provider.get(A + "exported") == "false"
            and provider.get(A + "grantUriPermissions") == "true"
            and provider.get(A + "authorities") == package + ".assets", "Invalid leased asset provider boundary")
    return package


def verify_elf(data, abi):
    require(len(data) >= 64 and data[:6] == b"\x7fELF\x02\x01", "Native entry must be a 64-bit little-endian ELF")
    elf_type, machine = struct.unpack_from("<HH", data, 16)
    require(elf_type == 3 and machine == {"arm64-v8a": 183, "x86_64": 62}[abi], "Wrong native library ABI/type")
    phoff = struct.unpack_from("<Q", data, 32)[0]
    phsize, phcount = struct.unpack_from("<HH", data, 54)
    require(phsize >= 56 and 0 < phcount <= 1024 and phoff + phsize * phcount <= len(data), "Invalid ELF program headers")
    loads = []
    for index in range(phcount):
        kind, _, offset, address, _, _, _, alignment = struct.unpack_from("<IIQQQQQQ", data, phoff + phsize * index)
        if kind == 1:
            require(alignment >= 16384 and offset % 16384 == address % 16384,
                    "Native LOAD segments must support Android 16 KiB pages")
            loads.append(alignment)
    require(bool(loads), "Native entry has no LOAD segment")
    return loads


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("apk", type=Path)
    parser.add_argument("--launcher", action="store_true")
    parser.add_argument("--abi", choices=["arm64-v8a", "x86_64"], default="arm64-v8a")
    args = parser.parse_args()
    if args.apk.is_dir():
        candidates = list(args.apk.glob("*.apk"))
        require(len(candidates) == 1, "Expected exactly one APK in the selected variant directory")
        args.apk = candidates[0]
    sdk = Path(os.environ.get("ANDROID_HOME", os.environ.get("ANDROID_SDK_ROOT", "")))
    analyzer = shutil.which("apkanalyzer") or str(sdk / "cmdline-tools/latest/bin/apkanalyzer")
    manifest = subprocess.run([analyzer, "manifest", "print", str(args.apk)],
                              capture_output=True, text=True, check=True, timeout=60).stdout
    package = verify_manifest(ET.fromstring(manifest), args.launcher)
    with zipfile.ZipFile(args.apk) as archive:
        libraries = [name for name in archive.namelist() if name.startswith("lib/") and name.endswith(".so")]
        require({name.split("/")[1] for name in libraries} == {args.abi}, "APK contains unexpected native ABIs")
        native_alignment = {name: verify_elf(archive.read(name), args.abi) for name in libraries}
        data = archive.read(f"lib/{args.abi}/libphone_android.so")
    readelf = shutil.which("llvm-readelf") or shutil.which("readelf")
    require(readelf is not None, "Install readelf to verify the JNI export")
    with tempfile.NamedTemporaryFile(suffix=".so") as library:
        library.write(data)
        library.flush()
        symbols = subprocess.run([readelf, "--dyn-syms", "--wide", library.name],
                                 capture_output=True, text=True, check=True, timeout=30).stdout
    require(any(line.split()[-1:] == ["JNI_OnLoad"] and " GLOBAL " in line
                and " UND " not in line for line in symbols.splitlines()), "Missing exported JNI_OnLoad")
    print(json.dumps({"apk": args.apk.name, "package": package, "launcher": args.launcher,
                      "abi": args.abi, "native_bytes": len(data), "load_alignment": native_alignment,
                      "native_sha256": hashlib.sha256(data).hexdigest(),
                      "apk_sha256": hashlib.sha256(args.apk.read_bytes()).hexdigest()}, indent=2))


if __name__ == "__main__":
    main()
