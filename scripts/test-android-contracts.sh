#!/usr/bin/env bash
set -euo pipefail
phone_repo="$(cd "$(dirname "$0")/.." && pwd)"
phone_test_out="$(mktemp -d "${TMPDIR:-/tmp}/phone-android-contracts.XXXXXX")"
trap 'rm -rf -- "$phone_test_out"' EXIT
phone_sdk="${ANDROID_HOME:-${ANDROID_SDK_ROOT:-}}"
phone_sdk_jar="${PHONE_ANDROID_JAR:-$phone_sdk/platforms/android-36/android.jar}"
[ -f "$phone_sdk_jar" ]
phone_sources="$phone_repo/crates/phone-android/platform/java/org/ferroui/phone"
phone_test="$phone_repo/crates/phone-android/tests/java/org/ferroui/phone/PhoneContractTest.java"
if [ -n "${PHONE_ECJ_JAR:-}" ]; then
    java -jar "$PHONE_ECJ_JAR" -source 8 -target 8 -proc:none -nowarn -classpath "$phone_sdk_jar" -d "$phone_test_out" \
        "$phone_sources/PhoneBackKey.java" "$phone_sources/PhoneDeviceAccess.java" "$phone_sources/PhoneDeliveryStatus.java" "$phone_sources/PhonePartProgress.java" "$phone_sources/PhoneNumbers.java" "$phone_sources/PhoneJson.java" "$phone_test"
else
    javac --release 8 -classpath "$phone_sdk_jar" -d "$phone_test_out" \
        "$phone_sources/PhoneBackKey.java" "$phone_sources/PhoneDeviceAccess.java" "$phone_sources/PhoneDeliveryStatus.java" "$phone_sources/PhonePartProgress.java" "$phone_sources/PhoneNumbers.java" "$phone_sources/PhoneJson.java" "$phone_test"
fi
java -classpath "$phone_test_out:$phone_sdk_jar" org.ferroui.phone.PhoneContractTest
python3 "$phone_repo/scripts/verify_android_vendor.py"
