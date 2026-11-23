#!/usr/bin/env python3
"""Add only capabilities implemented by the UIKit host to FerroUI's bundle."""
import pathlib
import plistlib
import sys

bundle = pathlib.Path(sys.argv[1]).resolve()
path = bundle / "Info.plist"
with path.open("rb") as stream:
    info = plistlib.load(stream)
if info.get("CFBundleIdentifier") != "org.ferroui.windowsphone":
    raise SystemExit("Refusing to modify a bundle with a different application identity.")
info.update({
    "NSLocationWhenInUseUsageDescription": "Use your location for an explicit Maps or Weather request.",
    "NSContactsUsageDescription": "Show the device contacts you choose in People and use their numbers for an explicit call or message.",
    "CFBundleURLTypes": [{
        "CFBundleURLName": "org.ferroui.windowsphone.activation",
        "CFBundleTypeRole": "Viewer",
        "CFBundleURLSchemes": ["ferroui-phone"],
    }],
    "LSApplicationQueriesSchemes": ["tel", "sms", "mailto", "maps"],
    "CFBundleIcons": {"CFBundlePrimaryIcon": {"CFBundleIconFiles": ["AppIcon60x60"]}},
    "CFBundleIcons~ipad": {"CFBundlePrimaryIcon": {
        "CFBundleIconFiles": ["AppIcon60x60", "AppIcon76x76", "AppIcon83.5x83.5"],
    }},
})
# No background modes, private settings URLs, home-screen privileges, associated
# domains, or managed default-app entitlements are silently provisioned.
with path.open("wb") as stream:
    plistlib.dump(info, stream, fmt=plistlib.FMT_BINARY)
