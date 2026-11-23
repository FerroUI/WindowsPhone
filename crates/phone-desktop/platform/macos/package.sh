#!/usr/bin/env bash
set -euo pipefail
repo_root="$(cd "$(dirname "$0")/../../../.." && pwd)"
cd "$repo_root"
if [[ "$(uname -s)" != Darwin ]]; then
  echo "Build the macOS application bundle on macOS." >&2
  exit 1
fi
export MACOSX_DEPLOYMENT_TARGET="${MACOSX_DEPLOYMENT_TARGET:-11.0}"
cargo build --locked --release -p phone-desktop
phone_target_dir="${CARGO_TARGET_DIR:-$repo_root/target}"
bundle_path="$phone_target_dir/WindowsPhone.app"
mkdir -p "$bundle_path/Contents/MacOS" "$bundle_path/Contents/Resources"
cp "$phone_target_dir/release/phone-desktop" "$bundle_path/Contents/MacOS/phone-desktop"
cp crates/phone-desktop/platform/macos/Info.plist "$bundle_path/Contents/Info.plist"
# Ad-hoc signing gives a local bundle identity. Distribution signing and
# notarization remain the distributor's responsibility.
codesign --force --sign - "$bundle_path"
printf '%s\n' "$bundle_path"
