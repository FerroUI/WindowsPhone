#!/usr/bin/env bash
# Native FerroUI Android application/HOME launcher. Does not install an APK or change HOME.
set -euo pipefail
phone_repo="$(cd "$(dirname "$0")/.." && pwd)"
phone_abi=arm64-v8a
phone_flavor=App
phone_profile=debug
phone_build_type=Debug
phone_cargo=1
while [ "$#" -gt 0 ]; do
    case "$1" in
        --launcher) phone_flavor=Launcher ;;
        --release) phone_profile=release; phone_build_type=Release ;;
        --abi) phone_abi="${2:?--abi needs arm64-v8a or x86_64}"; shift ;;
        --no-cargo) phone_cargo=0 ;;
        --help) printf '%s\n' 'build-android.sh [--launcher] [--release] [--abi arm64-v8a|x86_64] [--no-cargo]'; exit 0 ;;
        *) printf 'Unknown option: %s\n' "$1" >&2; exit 2 ;;
    esac
    shift
done
case "$phone_abi" in
    arm64-v8a) phone_target=aarch64-linux-android ;;
    x86_64) phone_target=x86_64-linux-android ;;
    *) printf 'Unsupported ABI: %s\n' "$phone_abi" >&2; exit 2 ;;
esac
source "$phone_repo/FerroUI/scripts/android/env.sh"
command -v gradle >/dev/null || { printf '%s\n' 'Gradle 8.13 and JDK 17 are required for the pinned Android plugin.' >&2; exit 1; }
phone_target_dir="${CARGO_TARGET_DIR:-$phone_repo/target}"
mkdir -p "$phone_target_dir"
phone_target_dir="$(cd "$phone_target_dir" && pwd)"
phone_args=(build --locked -p phone-android --target "$phone_target")
if [ "$phone_profile" = release ]; then phone_args+=(--release); fi
if [ "$phone_cargo" = 1 ]; then
    (cd "$phone_repo" && CARGO_TARGET_DIR="$phone_target_dir" cargo "${phone_args[@]}")
fi
gradle --no-daemon --console plain -p "$phone_repo/crates/phone-android/platform" \
    "-PferroUiRoot=$phone_repo/FerroUI" "-PphoneAbi=$phone_abi" \
    "-PphoneRustTargetDir=$phone_target_dir" "-PphoneRustProfile=$phone_profile" \
    "assemble${phone_flavor}${phone_build_type}"
printf 'APK output: %s\n' "$phone_target_dir/android/phone-$phone_abi-$phone_profile/outputs/apk"
