#!/usr/bin/env bash
# Owns one ephemeral API 36 x86_64 AVD. Temporary HOME/carrier roles exist only inside that AVD and are restored.
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
verify_private_avd() {
    local actual
    [ "$emulator_started" = 1 ] || return 1
    actual="$(with_timeout 5 "$ADB" -s "$FERROUI_SERIAL" emu avd name 2>/dev/null | tr -d '\r')" || return 1
    [ "${actual%%$'\n'*}" = "$FERROUI_AVD" ] || return 1
}
adb_do() {
    verify_private_avd || return 1
    with_timeout "${ADB_TIMEOUT:-60}" "$ADB" -s "$FERROUI_SERIAL" "$@"
}
phone_package=""
phone_carrier_roles=0
phone_home_role=0
phone_notification_access=0
phone_call_diagnostics=0
phone_call_logcat_pid=""
phone_phase="Preparing the ephemeral emulator"
phase() {
    phone_phase="$1"
    printf '%s\n' "$phone_phase" | tee "$phone_out/phase.txt"
}
start_call_diagnostics() {
    phone_call_diagnostics=1
    printf 'time_limit_seconds=300\nfile_limit_kib=32768\nbuffers=main,system,radio\n' > "$phone_out/call-logcat-limits.txt"
    (
        # Keep the beginning of this phase: a busy system can overwrite the
        # device ring buffer long before wait_device's failure is captured.
        # Own the reader PID directly so cleanup can stop it without touching
        # any other adb client. Both time and output size are bounded.
        trap - EXIT INT TERM
        ulimit -f 32768
        verify_private_avd || exit 1
        exec perl -e 'alarm shift; exec @ARGV' 300 "$ADB" -s "$FERROUI_SERIAL" \
            logcat -b main -b system -b radio -v threadtime -T 1
    ) > "$phone_out/call-logcat.txt" 2>&1 &
    phone_call_logcat_pid=$!
}
stop_call_diagnostics() {
    local status=0
    if [ -n "$phone_call_logcat_pid" ]; then
        kill "$phone_call_logcat_pid" 2>/dev/null || true
        wait "$phone_call_logcat_pid" || status=$?
        printf '%s\n' "$status" > "$phone_out/call-logcat-exit.txt"
        phone_call_logcat_pid=""
    fi
}
restore_home_role() {
    local holder
    adb_shell cmd role remove-role-holder --user "$phone_user" android.app.role.HOME org.ferroui.windowsphone.launcher > "$phone_out/home-restore-remove.txt" 2>&1 || true
    while IFS= read -r holder; do
        [[ "$holder" =~ ^[A-Za-z0-9_]+(\.[A-Za-z0-9_]+)+$ ]] || continue
        adb_shell cmd role add-role-holder --user "$phone_user" android.app.role.HOME "$holder" >> "$phone_out/home-restore-add.txt" 2>&1 || return 1
    done < "$phone_out/home-before.txt"
    adb_shell cmd role get-role-holders --user "$phone_user" android.app.role.HOME | tr -d '\r' | sort > "$phone_out/home-restored.txt"
    cmp "$phone_out/home-before.txt" "$phone_out/home-restored.txt" || return 1
    phone_home_role=0
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
            if [ "$phone_call_diagnostics" = 1 ]; then
                # Capture the real call before role restoration cancels the
                # fixture; diagnostic failures never replace the gate result.
                ADB_TIMEOUT=10 adb_do emu gsm list > "$phone_out/modem-failure-calls.txt" 2>&1
                ADB_TIMEOUT=10 emu_read_file "$phone_package" windows-phone/phone-device-smoke.json > "$phone_out/device-call-failure.json"
                ADB_TIMEOUT=20 adb_shell dumpsys telecom > "$phone_out/call-failure-telecom.txt" 2>&1
            fi
            emu_screencap "$phone_out/failure.png"
            if [ -n "$phone_package" ]; then
                emu_read_file "$phone_package" windows-phone/phone-android-smoke.json > "$phone_out/failure.json"
            fi
        fi
        ADB_TIMEOUT=30 adb_do logcat -d -t 12000 -v threadtime > "$phone_out/logcat.txt" 2>&1
        ADB_TIMEOUT=30 adb_do logcat -d -b crash -v threadtime > "$phone_out/crash.txt" 2>&1
        ADB_TIMEOUT=20 adb_shell dumpsys activity activities > "$phone_out/activities.txt" 2>&1
        ADB_TIMEOUT=20 adb_shell dumpsys window > "$phone_out/windows.txt" 2>&1
    fi
    stop_call_diagnostics
    if [ "$phone_notification_access" = 1 ]; then ADB_TIMEOUT=10 restore_notification_access || result=1; fi
    if [ "$phone_carrier_roles" = 1 ]; then ADB_TIMEOUT=10 restore_carrier_roles || result=1; fi
    if [ "$phone_home_role" = 1 ]; then ADB_TIMEOUT=10 restore_home_role || result=1; fi
    write_result "$result"
    emu_stop
    # This directory came from this invocation's mktemp, never from an existing AVD.
    rm -rf -- "$phone_avd_home"
    trap - EXIT
    exit "$result"
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
phase 'Waiting for completed provisioning and a stable eligible HOME baseline'
# sys.boot_completed can precede the setup app disabling its temporary HOME
# component. Preserve raw read-only observations and never restore that transient
# holder merely because RoleManager reported it once during first boot.
phone_home_deadline=$((SECONDS + 120))
home_readiness_query() {
    local name="$1" status=0 remaining=$((phone_home_deadline - SECONDS - 5))
    shift
    # Reserve the private-AVD identity check's five-second bound as well.
    if [ "$remaining" -le 0 ]; then return 1; fi
    if [ "$remaining" -gt 5 ]; then remaining=5; fi
    ADB_TIMEOUT="$remaining" adb_shell "$@" > "$phone_home_sample/$name.txt" \
        2> "$phone_home_sample/$name.error.txt" || status=$?
    printf '%s\n' "$status" > "$phone_home_sample/$name.status"
    return "$status"
}
phone_home_attempt=0
phone_home_previous=""
phone_home_ready=0
while [ "$SECONDS" -lt "$phone_home_deadline" ]; do
    if [ "$((phone_home_deadline - SECONDS))" -le 5 ]; then break; fi
    phone_home_attempt=$((phone_home_attempt + 1))
    phone_home_sample="$phone_out/home-readiness/$phone_home_attempt"
    mkdir -p "$phone_home_sample"
    date -u +%Y-%m-%dT%H:%M:%SZ > "$phone_home_sample/observed-at.txt"
    if home_readiness_query user am get-current-user \
        && home_readiness_query device-provisioned settings get global device_provisioned \
        && home_readiness_query user-setup-complete settings --user "$phone_user" get secure user_setup_complete \
        && home_readiness_query holders cmd role get-role-holders --user "$phone_user" android.app.role.HOME \
        && home_readiness_query resolved cmd package resolve-activity --components --user "$phone_user" \
            -a android.intent.action.MAIN -c android.intent.category.HOME \
        && home_readiness_query activities cmd package query-activities --components --user "$phone_user" \
            -a android.intent.action.MAIN -c android.intent.category.HOME \
        && home_readiness_query holders-after cmd role get-role-holders --user "$phone_user" android.app.role.HOME \
        && home_readiness_query user-after am get-current-user; then
        if [ -n "$phone_home_previous" ] && python3 "$phone_repo/scripts/verify_android_home.py" \
            "$phone_home_previous" "$phone_home_sample" "$phone_user" \
            > "$phone_out/home-before.tmp" 2> "$phone_out/home-readiness-error.txt"; then
            mv "$phone_out/home-before.tmp" "$phone_out/home-before.txt"
            printf '%s\n%s\n' "${phone_home_previous##*/}" "$phone_home_attempt" \
                > "$phone_out/home-readiness-accepted.txt"
            phone_home_ready=1
            break
        fi
    fi
    phone_home_previous="$phone_home_sample"
    if [ "$((SECONDS + 2))" -lt "$phone_home_deadline" ]; then sleep 2; else break; fi
done
if [ "$phone_home_ready" != 1 ]; then
    printf 'Timed out waiting for a provisioned, stable and eligible original HOME; see home-readiness observations\n' >&2
    exit 1
fi
adb_do logcat -c

wait_for_view() {
    local nonce="$1" uri="$2" after="$3" candidate=0 current=0
    local -a role_args=()
    if [ "${4:-app}" = home ]; then role_args=(--expect-home-active); fi
    local deadline=$((SECONDS + 100))
    while [ "$SECONDS" -lt "$deadline" ]; do
        emu_read_file "$phone_package" windows-phone/phone-android-smoke.json > "$phone_out/current.tmp"
        mv "$phone_out/current.tmp" "$phone_out/current.json"
        if current="$(python3 "$phone_repo/scripts/verify_android_ui.py" "$phone_out/current.json" \
            "$nonce" "$uri" "$after" "${role_args[@]}" 2> "$phone_out/last-observation-error.txt")"; then
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
        if [ "$phone_page" = home-restored ]; then
            adb_shell am start -W -a android.intent.action.MAIN -c android.intent.category.HOME -n "$phone_component" > "$phone_out/$phone_variant-$phone_page-activation.txt" 2>&1
        else
            adb_shell am start -W -a android.intent.action.VIEW -d "$phone_uri" -n "$phone_component" > "$phone_out/$phone_variant-$phone_page-activation.txt" 2>&1
        fi
        wait_for_view "$phone_nonce" "$phone_uri" "$phone_sequence"
        phase "Checking the focused native window for $phone_variant / $phone_page"
        # Android 36's windows-only section omits the per-display mCurrentFocus.
        # Match the framework smoke helper's full dump without weakening focus checks.
        adb_shell dumpsys window > "$phone_out/$phone_variant-$phone_page-window.txt"
        if ! grep 'mCurrentFocus' "$phone_out/$phone_variant-$phone_page-window.txt" \
            | grep -F "$phone_component" >/dev/null; then
            printf 'The requested activity is not the focused native window: %s\n' "$phone_component" >&2
            exit 1
        fi
        cp "$phone_out/current.json" "$phone_out/$phone_variant-$phone_page.json"
        emu_screencap "$phone_out/$phone_variant-$phone_page.png"
        if [ "$phone_page" = wallet ]; then
            phase "Checking real Android Back from $phone_variant Wallet to Start"
            adb_shell input keyevent KEYCODE_BACK
            wait_for_view "$phone_nonce" "back:$phone_uri" "$phone_sequence"
            cp "$phone_out/current.json" "$phone_out/$phone_variant-back.json"
            emu_screencap "$phone_out/$phone_variant-back.png"
            # Reopen Wallet so the next actual HOME intent must navigate back to Start.
            adb_shell am start -W -a android.intent.action.VIEW -d "$phone_uri" -n "$phone_component" > "$phone_out/$phone_variant-wallet-reopen.txt" 2>&1
            wait_for_view "$phone_nonce" "$phone_uri" "$phone_sequence"
        fi
    done
    adb_shell am force-stop "$phone_package"
done

source "$phone_repo/scripts/smoke-android-home.sh"
source "$phone_repo/scripts/smoke-android-device.sh"

phase 'Verifying the current user is unchanged and the original HOME role holder is restored'
adb_shell am get-current-user | tr -d '\r' > "$phone_out/user-after.txt"
adb_shell cmd role get-role-holders --user "$phone_user" android.app.role.HOME \
    | tr -d '\r' | sort > "$phone_out/home-after.txt"
cmp "$phone_out/user-before.txt" "$phone_out/user-after.txt"
cmp "$phone_out/home-before.txt" "$phone_out/home-after.txt"
phase 'Native geometry, Back, active HOME, cold-Start SMS badge, rendered notification bootstrap/revocation, restored access, incoming call ownership and MMS codec passed'
