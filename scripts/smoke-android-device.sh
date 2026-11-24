#!/usr/bin/env bash
# Sourced only by smoke-android.sh after its exact-AVD guard creates a private emulator.
[ "${emulator_started:-0}" = 1 ] && [ "${FERROUI_EMU_REUSE:-1}" = 0 ]
[[ "${FERROUI_AVD:-}" == phone_ui_* ]]
phone_role_package=org.ferroui.windowsphone.launcher
phone_package="$phone_role_package"
phone_component="$phone_package/org.ferroui.phone.PhoneActivity"

restore_carrier_roles() {
    local role holder
    adb_do emu gsm cancel 5550102 >/dev/null 2>&1 || true
    for role in DIALER SMS; do
        adb_shell cmd role remove-role-holder --user "$phone_user" "android.app.role.$role" "$phone_role_package" > "$phone_out/$role-restore-remove.txt" 2>&1 || true
        while IFS= read -r holder; do
            [[ "$holder" =~ ^[A-Za-z0-9_]+(\.[A-Za-z0-9_]+)+$ ]] || continue
            adb_shell cmd role add-role-holder --user "$phone_user" "android.app.role.$role" "$holder" >> "$phone_out/$role-restore-add.txt" 2>&1 || return 1
        done < "$phone_out/$role-before.txt"
        adb_shell cmd role get-role-holders --user "$phone_user" "android.app.role.$role" | tr -d '\r' | sort > "$phone_out/$role-after.txt"
        cmp "$phone_out/$role-before.txt" "$phone_out/$role-after.txt" || return 1
    done
    phone_carrier_roles=0
}

wait_device() {
    local expected="$1" candidate=0 current=0
    local stop=$((SECONDS + 90))
    while [ "$SECONDS" -lt "$stop" ]; do
        emu_read_file "$phone_package" windows-phone/phone-device-smoke.json > "$phone_out/device-current.tmp"
        mv "$phone_out/device-current.tmp" "$phone_out/device-current.json"
        case "$expected" in
            ringing|active|ended)
                # The 90-second deadline and existing 500ms polling interval
                # bound this history. Preserve transient real states as well
                # as the last sample without changing the acceptance test.
                printf '%s\n' "$(<"$phone_out/device-current.json")" >> "$phone_out/call-$expected-observations.jsonl"
                ;;
        esac
        if current="$(python3 "$phone_repo/scripts/verify_android_device.py" "$phone_out/device-current.json" "$phone_device_nonce" "$expected" "$phone_device_sequence" 2> "$phone_out/device-last-error.txt")"; then
            if [ "$candidate" -gt 0 ] && [ "$current" -gt "$candidate" ]; then
                phone_device_sequence="$current"
                cp "$phone_out/device-current.json" "$phone_out/device-$expected.json"
                return 0
            fi
            candidate="$current"
        else
            candidate=0
        fi
        sleep 0.5
    done
    printf 'Native %s observation failed: %s\n' "$expected" "$(cat "$phone_out/device-last-error.txt")" >&2
    return 1
}

wait_bootstrap() {
    local expected="$1" candidate=0 current=0 stop=$((SECONDS + ${2:-75}))
    while [ "$SECONDS" -lt "$stop" ]; do
        emu_read_file "$phone_package" windows-phone/phone-android-smoke.json > "$phone_out/bootstrap-current.tmp"
        mv "$phone_out/bootstrap-current.tmp" "$phone_out/bootstrap-current.json"
        if current="$(python3 "$phone_repo/scripts/verify_android_bootstrap.py" "$phone_out/bootstrap-current.json" \
            "$phone_device_nonce" "$expected" "$phone_bootstrap_sequence" --sms "$phone_out/device-sms.json" 2> "$phone_out/bootstrap-last-error.txt")"; then
            if [ "$candidate" -gt 0 ] && [ "$current" -gt "$candidate" ]; then
                phone_bootstrap_sequence="$current"
                cp "$phone_out/bootstrap-current.json" "$phone_out/device-$expected.json"
                return 0
            fi
            candidate="$current"
        else candidate=0; fi
        sleep 0.5
    done
    printf 'Shared %s observation failed: %s\n' "$expected" "$(cat "$phone_out/bootstrap-last-error.txt")" >&2
    return 1
}

for phone_role in DIALER SMS; do
    adb_shell cmd role get-role-holders --user "$phone_user" "android.app.role.$phone_role" | tr -d '\r' | sort > "$phone_out/$phone_role-before.txt"
done
phone_device_nonce="$(python3 -c 'import secrets; print(secrets.token_hex(16))')"
phone_device_sequence=0
phone_bootstrap_sequence=0
emu_write_file "$phone_package" windows-phone/phone-device-smoke.enabled "$phone_device_nonce"
emu_write_file "$phone_package" windows-phone/phone-android-smoke.enabled "$phone_device_nonce"
phase 'Checking native codec bounds and carrier permission boundaries before role selection'
adb_shell am start -W -a android.intent.action.VIEW -d ferroui-phone://home -n "$phone_component" > "$phone_out/device-baseline-activation.txt" 2>&1
wait_device baseline

phase 'Selecting qualified dialer and SMS roles only inside the newly created emulator'
phone_carrier_roles=1
for phone_role in DIALER SMS; do
    adb_shell cmd role add-role-holder --user "$phone_user" "android.app.role.$phone_role" "$phone_package" > "$phone_out/$phone_role-select.txt" 2>&1
    adb_shell cmd role get-role-holders --user "$phone_user" "android.app.role.$phone_role" | tr -d '\r' > "$phone_out/$phone_role-selected.txt"
    [ "$(cat "$phone_out/$phone_role-selected.txt")" = "$phone_package" ]
done
for phone_permission in READ_PHONE_STATE CALL_PHONE READ_CALL_LOG READ_SMS SEND_SMS RECEIVE_SMS RECEIVE_MMS RECEIVE_WAP_PUSH POST_NOTIFICATIONS; do
    adb_shell pm grant "$phone_package" "android.permission.$phone_permission" >> "$phone_out/device-permission-grants.txt" 2>&1
done
wait_device roles

phase 'Receiving a modem SMS through the protected receiver, native job and Android provider'
adb_do emu sms send 5550101 "FerroUI SMS $phone_device_nonce" > "$phone_out/modem-sms.txt" 2>&1
wait_device sms
phase 'Verifying native unread state and the mounted Messaging badge on Start before opening Messaging'
wait_bootstrap start-unread
emu_screencap "$phone_out/device-start-unread.png"
source "$phone_repo/scripts/smoke-android-notifications.sh"
phone_thread="$(python3 - "$phone_out/device-sms.json" <<'PY'
import base64,json,sys
value=json.load(open(sys.argv[1]))['received_sms']['thread_id']
print(base64.urlsafe_b64encode(value.encode()).decode().rstrip('='))
PY
)"
phone_sms_uri="ferroui-phone://app/phone.messaging?path=thread%2F$phone_thread"
adb_shell am start -W -a android.intent.action.VIEW -d "$phone_sms_uri" -n "$phone_component" > "$phone_out/device-sms-activation.txt" 2>&1
phone_stop=$((SECONDS + 75))
phone_sms_mounted=0
while [ "$SECONDS" -lt "$phone_stop" ]; do
    emu_read_file "$phone_package" windows-phone/phone-android-smoke.json > "$phone_out/device-sms-ui.json"
    if python3 - "$phone_out/device-sms-ui.json" "$phone_out/device-sms.json" "$phone_device_nonce" "$phone_sms_uri" <<'PY'
import json,sys
try:
    ui,sms=json.load(open(sys.argv[1])),json.load(open(sys.argv[2]))['received_sms']
    assert ui['nonce']==sys.argv[3] and ui['ready'] and not ui['closed']
    assert ui['activation']['uri']==sys.argv[4] and ui['activation']['outcome']=='accepted'
    assert ui['route']=={'App':'Messaging'}
    assert ui['device_message_thread']==sms['thread_id'] and sms['id'] in ui['device_message_ids']
    assert {'PhoneScreen','PhonePage'}<=set(ui['mounted_controls'])
except (ValueError,KeyError,AssertionError,TypeError):
    raise SystemExit(1)
PY
    then phone_sms_mounted=1; break; fi
    sleep 0.5
done
[ "$phone_sms_mounted" = 1 ]
emu_screencap "$phone_out/device-messaging.png"

phase 'Displaying and answering a real emulator Telecom incoming call'
start_call_diagnostics
ADB_TIMEOUT=10 adb_do emu gsm list > "$phone_out/modem-before-call.txt" 2>&1 || true
adb_do emu gsm call 5550102 > "$phone_out/modem-call.txt" 2>&1
wait_device ringing
ADB_TIMEOUT=10 adb_do emu gsm list > "$phone_out/modem-ringing-calls.txt" 2>&1 || true
ADB_TIMEOUT=10 adb_shell dumpsys window > "$phone_out/call-before-answer-window.txt" 2>&1 || true
emu_screencap "$phone_out/device-ringing.png"
adb_shell uiautomator dump /sdcard/phone-native-call.xml > "$phone_out/call-uiautomator.txt" 2>&1
adb_do pull /sdcard/phone-native-call.xml "$phone_out/call-tree.xml" >/dev/null
read -r phone_answer_x phone_answer_y < <(python3 - "$phone_out/call-tree.xml" <<'PY'
import re,sys,xml.etree.ElementTree as E
for node in E.parse(sys.argv[1]).iter('node'):
    if node.get('text','').strip().lower()=='answer' and node.get('clickable')=='true' and node.get('enabled')=='true':
        x1,y1,x2,y2=map(int,re.fullmatch(r'\[(\d+),(\d+)\]\[(\d+),(\d+)\]',node.get('bounds','')).groups())
        print((x1+x2)//2,(y1+y2)//2)
        break
else:
    raise SystemExit('The real native answer button is unavailable')
PY
)
[[ "$phone_answer_x" =~ ^[0-9]+$ ]] && [[ "$phone_answer_y" =~ ^[0-9]+$ ]]
printf '%s %s\n' "$phone_answer_x" "$phone_answer_y" > "$phone_out/call-answer-input.txt"
adb_shell input tap "$phone_answer_x" "$phone_answer_y"
ADB_TIMEOUT=10 adb_do emu gsm list > "$phone_out/modem-after-answer-calls.txt" 2>&1 || true
ADB_TIMEOUT=10 emu_read_file "$phone_package" windows-phone/phone-device-smoke.json > "$phone_out/device-after-answer.json" || true
wait_device active
emu_screencap "$phone_out/device-active-call.png"
adb_do emu gsm list > "$phone_out/modem-active-calls.txt" 2>&1
adb_do emu gsm cancel 5550102 > "$phone_out/modem-call-end.txt" 2>&1
wait_device ended
emu_screencap "$phone_out/device-ended-call.png"
stop_call_diagnostics
phase 'Restoring the original carrier-role holders inside the isolated AVD'
restore_carrier_roles
adb_shell am force-stop "$phone_package"
