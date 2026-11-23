#!/usr/bin/env bash
# Build the real UIKit host with FerroUI's external-workspace bundle support.
set -euo pipefail
root="$(cd "$(dirname "$0")/.." && pwd)"
target=aarch64-apple-ios-sim
profile=dev
build_args=()
while [ "$#" -gt 0 ]; do
  case "$1" in
    --target) target="$2"; shift 2 ;;
    --profile) profile="$2"; shift 2 ;;
    --no-build) build_args+=(--no-build); shift ;;
    *) echo "Usage: scripts/build-ios.sh [--target aarch64-apple-ios-sim|aarch64-apple-ios|x86_64-apple-ios] [--profile dev|release] [--no-build]" >&2; exit 2 ;;
  esac
done
if [ "$(uname -s)" != Darwin ]; then
  echo "The iOS build requires macOS and Xcode's iOS SDK." >&2
  exit 2
fi
app="$(FERROUI_IOS_WORKSPACE="$root" bash "$root/FerroUI/scripts/ios/bundle.sh" \
  --package phone-ios --bin phone-ios --target "$target" --profile "$profile" \
  --bundle-id org.ferroui.windowsphone --name "Windows Phone" --high-refresh-rate \
  --camera-usage-description "Take a photo and add it to your phone's local Camera Roll." \
  ${build_args[@]+"${build_args[@]}"})"
python3 "$root/scripts/ios-bundle-metadata.py" "$app"
for spec in "120 AppIcon60x60@2x" "180 AppIcon60x60@3x" "152 AppIcon76x76@2x" "167 AppIcon83.5x83.5@2x"; do
  read -r pixels name <<< "$spec"
  sips -z "$pixels" "$pixels" "$root/crates/phone-browser/wwwroot/icon-512.png" --out "$app/$name.png" >/dev/null
done
plutil -lint "$app/Info.plist" >&2
case "$target" in
  aarch64-apple-ios-sim|x86_64-apple-ios) codesign --force --sign - --timestamp=none "$app" >&2 ;;
  aarch64-apple-ios) echo "Device bundle is unsigned; use your development signing identity and provisioning profile." >&2 ;;
esac
printf '%s\n' "$app"
