#!/usr/bin/env bash
# Owns one ephemeral API 36 x86_64 AVD. Never reuses a device or selects a HOME app.
set -euo pipefail
phone_repo="$(cd "$(dirname "$0")/.." && pwd)"
phone_apks="${1:-$phone_repo/target/android/phone-x86_64-debug/outputs/apk}"
phone_out="${2:-$phone_repo/target/android-ui}"
mkdir -p "$phone_out"
phone_out="$(cd "$phone_out" && pwd)"
source "$phone_repo/FerroUI/scripts/android/env.sh"
phone_avd_home="$(mktemp -d "${RUNNER_TEMP:-${TMPDIR:-/tmp}}/phone-android-avd.XXXXXX")"
export ANDROID_AVD_HOME="$phone_avd_home"
export FERROUI_AVD="phone_ui_${GITHUB_RUN_ID:-local}_$$"
export FERROUI_EMU_REUSE=0
export FERROUI_EMU_BOOT_TIMEOUT=240
# Choose an unused emulator console/adb pair; every call still names its exact serial.
FERROUI_EMU_PORT="$(python3 - <<'PY'
import socket
for port in range(5584, 5682, 2):
    first, second = socket.socket(), socket.socket()
    try:
        first.bind(('127.0.0.1', port))
        second.bind(('127.0.0.1', port+1))
        print(port)
        break
    except OSError:
        pass
    finally:
        first.close()
        second.close()
else:
    raise SystemExit('No unused emulator console/adb port pair')
PY
)"
export FERROUI_EMU_PORT
source "$phone_repo/FerroUI/scripts/android/emu-common.sh"
# Recheck the AVD identity before every device operation, including helper cleanup.
# A concurrent process winning the selected port can never become our test device.
adb_do() {
    local actual
    [ "$emulator_started" = 1 ] || return 1
    actual="$(with_timeout 5 "$ADB" -s "$FERROUI_SERIAL" emu avd name 2>/dev/null | tr -d '\r')" || return 1
    [ "${actual%%$'\n'*}" = "$FERROUI_AVD" ] || return 1
    with_timeout "${ADB_TIMEOUT:-60}" "$ADB" -s "$FERROUI_SERIAL" "$@"
}
phone_package=""
phone_phase="Preparing the ephemeral emulator"
phase() {
    phone_phase="$1"
    printf '%s\n' "$phone_phase" | tee "$phone_out/phase.txt"
}
write_result() {
    python3 - "$phone_out" "$1" "$phone_phase" <<'PY'
import json, pathlib, sys
out, code, phase = pathlib.Path(sys.argv[1]), int(sys.argv[2]), sys.argv[3]
evidence={'result':'passed' if code == 0 else 'failed','exit_code':code,'phase':phase,
          'target':{'platform':'android','api':36,'abi':'x86_64','gpu':'software'},
          'expected_routes':['Start','org.ferroui.wallet','Start'], 'expected_variants':['app','launcher']}
evidence['captures']=sorted(p.name for p in out.glob('*.png'))
evidence['observations']=sorted(p.name for p in out.glob('*-*.json'))
(out/'result.json').write_text(json.dumps(evidence,indent=2)+'\n')
PY
}
cleanup() {
    local result=$?
    set +e
    # Write the outcome before bounded adb cleanup, including a forced timeout.
    write_result "$result"
    if [ "$emulator_started" = 1 ]; then
        if [ "$result" != 0 ]; then
            emu_screencap "$phone_out/failure.png"
            if [ -n "$phone_package" ]; then
                emu_read_file "$phone_package" windows-phone/phone-android-smoke.json > "$phone_out/failure.json"
            fi
        fi
        ADB_TIMEOUT=30 adb_do logcat -d -t 12000 -v threadtime > "$phone_out/logcat.txt" 2>&1
        ADB_TIMEOUT=30 adb_do logcat -d -b crash -v threadtime > "$phone_out/crash.txt" 2>&1
        ADB_TIMEOUT=20 adb_shell dumpsys activity activities > "$phone_out/activities.txt" 2>&1
        ADB_TIMEOUT=20 adb_shell dumpsys window windows > "$phone_out/windows.txt" 2>&1
    fi
    write_result "$result"
    emu_stop
    # This directory came from this invocation's mktemp, never from an existing AVD.
    rm -rf -- "$phone_avd_home"
    return "$result"
}
trap cleanup EXIT
trap 'exit 130' INT
trap 'exit 143' TERM

phase 'Creating a private Android 36 x86_64 AVD'
"$EMULATOR" -version > "$phone_out/emulator-version.txt" 2>&1 || true
"$EMULATOR" -accel-check > "$phone_out/acceleration.txt" 2>&1
printf 'no\n' | "$ANDROID_HOME/cmdline-tools/latest/bin/avdmanager" create avd \
    --name "$FERROUI_AVD" --package 'system-images;android-36;google_apis;x86_64' \
    --device pixel_7 --path "$phone_avd_home/device.avd" > "$phone_out/avd-create.txt" 2>&1
# Keep software GPU work bounded while retaining real system-bar insets.
python3 - "$phone_avd_home/device.avd/config.ini" <<'PY'
from pathlib import Path
import sys
p=Path(sys.argv[1])
settings={'hw.lcd.width':'720','hw.lcd.height':'1440','hw.lcd.density':'320',
          'hw.ramSize':'2048','hw.cpu.ncore':'2','disk.dataPartition.size':'2048M',
          'hw.keyboard':'yes','showDeviceFrame':'no'}
lines=[line for line in p.read_text().splitlines() if line.partition('=')[0].strip() not in settings]
p.write_text('\n'.join(lines+[f'{key}={value}' for key,value in settings.items()])+'\n')
PY
cp "$phone_avd_home/device.avd/config.ini" "$phone_out/avd-config.ini"
phase 'Booting the isolated emulator with software graphics'
emu_start software "$phone_out/emulator.log"
phone_api="$(adb_shell getprop ro.build.version.sdk | tr -d '\r\n')"
phone_abi="$(adb_shell getprop ro.product.cpu.abi | tr -d '\r\n')"
python3 - "$phone_out/device.json" "$phone_api" "$phone_abi" "$FERROUI_SERIAL" <<'PY'
import json, pathlib, sys
pathlib.Path(sys.argv[1]).write_text(json.dumps({'api':sys.argv[2],'abi':sys.argv[3],'serial':sys.argv[4]},indent=2)+'\n')
PY
[ "$phone_api" = 36 ] && [ "$phone_abi" = x86_64 ]
adb_shell wm size > "$phone_out/display-size.txt"
adb_shell wm density > "$phone_out/display-density.txt"
phone_user="$(adb_shell am get-current-user | tr -d '\r\n')"
[[ "$phone_user" =~ ^[0-9]+$ ]]
printf '%s\n' "$phone_user" > "$phone_out/user-before.txt"
adb_shell cmd role get-role-holders --user "$phone_user" android.app.role.HOME \
    | tr -d '\r' | sort > "$phone_out/home-before.txt"
adb_do logcat -c

wait_for_view() {
    local nonce="$1" uri="$2" after="$3" candidate=0 current=0
    local deadline=$((SECONDS + 100))
    while [ "$SECONDS" -lt "$deadline" ]; do
        emu_read_file "$phone_package" windows-phone/phone-android-smoke.json > "$phone_out/current.tmp"
        mv "$phone_out/current.tmp" "$phone_out/current.json"
        if current="$(python3 "$phone_repo/scripts/verify_android_ui.py" "$phone_out/current.json" \
            "$nonce" "$uri" "$after" 2> "$phone_out/last-observation-error.txt")"; then
            # Two distinct valid observations rule out a frozen or transient layout.
            if [ "$candidate" -gt 0 ] && [ "$current" -gt "$candidate" ]; then
                phone_sequence="$current"
                return 0
            fi
            candidate="$current"
        else
            candidate=0
        fi
        sleep 0.5
    done
    printf 'Timed out waiting for %s: %s\n' "$uri" "$(cat "$phone_out/last-observation-error.txt")" >&2
    return 1
}

for phone_variant in app launcher; do
    phone_package=org.ferroui.windowsphone
    if [ "$phone_variant" = launcher ]; then phone_package+=.launcher; fi
    phone_component="$phone_package/org.ferroui.phone.PhoneActivity"
    phone_apk="$phone_apks/$phone_variant/debug/WindowsPhoneAndroid-$phone_variant-debug.apk"
    [ -f "$phone_apk" ]
    phase "Installing the $phone_variant package into the private emulator"
    emu_install "$phone_apk" "$phone_package"
    phone_nonce="$(python3 -c 'import secrets; print(secrets.token_hex(16))')"
    adb_shell run-as "$phone_package" mkdir -p files/windows-phone
    emu_write_file "$phone_package" windows-phone/phone-android-smoke.enabled "$phone_nonce"
    phone_sequence=0
    for phone_page in start wallet home-restored; do
        phone_uri=ferroui-phone://home
        if [ "$phone_page" = wallet ]; then phone_uri=ferroui-phone://app/org.ferroui.wallet; fi
        phase "Activating $phone_variant / $phone_page through the real Android activity"
        adb_shell am start -W -a android.intent.action.VIEW -d "$phone_uri" -n "$phone_component" \
            > "$phone_out/$phone_variant-$phone_page-activation.txt" 2>&1
        wait_for_view "$phone_nonce" "$phone_uri" "$phone_sequence"
        adb_shell dumpsys window windows > "$phone_out/$phone_variant-$phone_page-window.txt"
        grep 'mCurrentFocus' "$phone_out/$phone_variant-$phone_page-window.txt" | grep -F "$phone_package" >/dev/null
        cp "$phone_out/current.json" "$phone_out/$phone_variant-$phone_page.json"
        emu_screencap "$phone_out/$phone_variant-$phone_page.png"
    done
    adb_shell am force-stop "$phone_package"
done

phase 'Verifying the current user and HOME role holder were preserved'
adb_shell am get-current-user | tr -d '\r' > "$phone_out/user-after.txt"
adb_shell cmd role get-role-holders --user "$phone_user" android.app.role.HOME \
    | tr -d '\r' | sort > "$phone_out/home-after.txt"
cmp "$phone_out/user-before.txt" "$phone_out/user-after.txt"
cmp "$phone_out/home-before.txt" "$phone_out/home-after.txt"
phase 'Both Android variants mounted inside system insets and navigated Start, Wallet and Home'
