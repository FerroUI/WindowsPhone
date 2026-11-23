#!/usr/bin/env python3
"""Check the actual APK boundary, without installing it or selecting a HOME app."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import re
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


def android_enum(value, symbol, number):
    """APK XML printers may preserve enum names or render their encoded integers."""
    if value == symbol:
        return True
    if value is None or re.fullmatch(r"(?:0[xX][0-9a-fA-F]+|[0-9]+)", value) is None:
        return False
    return int(value, 16 if value.lower().startswith("0x") else 10) == number


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
    launch_mode = activity.get(A + "launchMode") if activity is not None else None
    # ActivityInfo.LAUNCH_SINGLE_TASK == 2. Reject the other launch modes.
    require(activity is not None and android_enum(launch_mode, "singleTask", 2),
            f"Missing single-task activity (launchMode={launch_mode!r})")
    categories = {c.get(A + "name") for f in activity.findall("intent-filter") for c in f.findall("category")}
    require("android.intent.category.LAUNCHER" in categories, "The package needs an application entry")
    require(("android.intent.category.HOME" in categories) == launcher, "HOME role leaked across package flavors")
    service = next((v for v in app.findall("service")
                    if v.get(A + "name") == "org.ferroui.phone.PhoneMediaService"), None)
    service_type = service.get(A + "foregroundServiceType") if service is not None else None
    # ServiceInfo.FOREGROUND_SERVICE_TYPE_MEDIA_PLAYBACK == 2, with no added type bits.
    require(service is not None and service.get(A + "exported") == "false"
            and android_enum(service_type, "mediaPlayback", 2),
            f"Invalid media service boundary (foregroundServiceType={service_type!r})")
    provider = next((v for v in app.findall("provider")
                     if v.get(A + "name") == "org.ferroui.phone.PhoneAssetProvider"), None)
    require(provider is not None and provider.get(A + "exported") == "false"
            and provider.get(A + "grantUriPermissions") == "true"
            and provider.get(A + "authorities") == package + ".assets", "Invalid leased asset provider boundary")
    def component(tag, name):
        return next((value for value in app.findall(tag) if value.get(A + "name") == "org.ferroui.phone." + name), None)

    def protected(tag, name, permission):
        value = component(tag, name)
        require(value is not None and value.get(A + "permission") == "android.permission." + permission,
                f"Missing protected native component: {name}")
        return value

    call = protected("service", "PhoneInCallService", "BIND_INCALL_SERVICE")
    require("android.permission.FOREGROUND_SERVICE_PHONE_CALL" in permissions,
            "Phone-call foreground service permission is missing")
    require(android_enum(call.get(A + "foregroundServiceType"), "phoneCall", 4),
            "Invalid phone-call foreground service type")
    require(call.get(A + "exported") == "true" and any(value.get(A + "name") == "android.telecom.IN_CALL_SERVICE_UI"
            and value.get(A + "value") == "true" for value in call.findall("meta-data")), "The default dialer needs the real exported Telecom UI service")
    require(not any(value.get(A + "name") == "android.telecom.IN_CALL_SERVICE_RINGING" for value in call.findall("meta-data")),
            "Android Telecom must retain ringtone ownership")
    protected("service", "PhoneNotificationListener", "BIND_NOTIFICATION_LISTENER_SERVICE")
    protected("service", "PhoneRespondViaMessageService", "SEND_RESPOND_VIA_MESSAGE")
    protected("service", "PhoneMessageJobService", "BIND_JOB_SERVICE")
    protected("receiver", "PhoneSmsReceiver", "BROADCAST_SMS")
    protected("receiver", "PhoneMmsReceiver", "BROADCAST_WAP_PUSH")
    for tag, name in [("activity", "PhoneCallActivity"), ("receiver", "PhoneCallActionReceiver"),
                      ("receiver", "PhoneSmsResultReceiver"), ("receiver", "PhoneMmsResultReceiver")]:
        value = component(tag, name)
        require(value is not None and value.get(A + "exported") == "false", f"Native callback must remain private: {name}")
    transfer = component("provider", "PhoneMessageFileProvider")
    require(transfer is not None and transfer.get(A + "exported") == "false"
            and transfer.get(A + "grantUriPermissions") == "true"
            and transfer.get(A + "authorities") == package + ".message-files", "Invalid native message lease provider")
    filters = activity.findall("intent-filter")
    dial = [value for value in filters if any(v.get(A + "name") == "android.intent.action.DIAL" for v in value.findall("action"))]
    require(any(not v.findall("data") for v in dial) and any(any(d.get(A + "scheme") == "tel" for d in v.findall("data")) for v in dial),
            "The native dialer must handle both empty and tel DIAL intents")
    sendto = [value for value in filters if any(v.get(A + "name") == "android.intent.action.SENDTO" for v in value.findall("action"))]
    require({"sms", "smsto", "mms", "mmsto"} <= {d.get(A + "scheme") for value in sendto for d in value.findall("data")},
            "The default SMS application must handle all four standard composition schemes")
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
    parser.add_argument("--manifest-out", type=Path, help="Save the decoded APK manifest before validation")
    args = parser.parse_args()
    if args.apk.is_dir():
        candidates = list(args.apk.glob("*.apk"))
        require(len(candidates) == 1, "Expected exactly one APK in the selected variant directory")
        args.apk = candidates[0]
    sdk = Path(os.environ.get("ANDROID_HOME", os.environ.get("ANDROID_SDK_ROOT", "")))
    analyzer = shutil.which("apkanalyzer") or str(sdk / "cmdline-tools/latest/bin/apkanalyzer")
    manifest = subprocess.run([analyzer, "manifest", "print", str(args.apk)],
                              capture_output=True, text=True, check=True, timeout=60).stdout
    if args.manifest_out:
        args.manifest_out.parent.mkdir(parents=True, exist_ok=True)
        args.manifest_out.write_text(manifest, encoding="utf-8")
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
