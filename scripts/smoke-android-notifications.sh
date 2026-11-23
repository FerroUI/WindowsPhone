#!/usr/bin/env bash
# Sourced only inside the exact private AVD after carrier roles and cold-Start unread proof.
[ "${emulator_started:-0}" = 1 ] && [ "${FERROUI_EMU_REUSE:-1}" = 0 ]
[[ "${FERROUI_AVD:-}" == phone_ui_* ]]
phone_listener=org.ferroui.windowsphone.launcher/org.ferroui.phone.PhoneNotificationListener

listener_state() {
    adb_shell settings --user "$phone_user" get secure enabled_notification_listeners > "$phone_out/listener-current-raw.txt"
    python3 - "$phone_repo/scripts" "$phone_out/listener-current-raw.txt" <<'PY'
import pathlib,sys
sys.path.insert(0,sys.argv[1])
from verify_android_bootstrap import listener_set
for component in listener_set(pathlib.Path(sys.argv[2]).read_text()): print(component)
PY
}

wait_listener_state() {
    local operation="$1" output="$2" status=0 stop=$((SECONDS + 30))
    while [ "$SECONDS" -lt "$stop" ]; do
        listener_state > "$phone_out/listener-observed.txt" || return 1
        if python3 - "$phone_repo/scripts" "$phone_out/listener-before.txt" "$phone_out/listener-observed.txt" "$phone_listener" "$operation" \
            >> "$phone_out/listener-$operation-observations.jsonl" <<'PY'
import json,pathlib,sys,time
sys.path.insert(0,sys.argv[1])
from verify_android_bootstrap import listener_transition
before=pathlib.Path(sys.argv[2]).read_text().splitlines()
observed=pathlib.Path(sys.argv[3]).read_text().splitlines()
try:
    matched=listener_transition(before,observed,sys.argv[4],sys.argv[5])
except ValueError as error:
    print(json.dumps({'observed_at_unix_ms':round(time.time()*1000),'components':observed,'error':str(error)}))
    print(str(error),file=sys.stderr)
    raise SystemExit(1)
print(json.dumps({'observed_at_unix_ms':round(time.time()*1000),'components':observed,'matched':matched}))
raise SystemExit(0 if matched else 2)
PY
        then
            cp "$phone_out/listener-observed.txt" "$phone_out/$output"
            cp "$phone_out/listener-current-raw.txt" "$phone_out/listener-$operation-raw.txt"
            return 0
        else status=$?; fi
        [ "$status" = 2 ] || return "$status"
        sleep 0.5
    done
    printf 'Android did not publish the exact listener %s set within 30 seconds\n' "$operation" >&2
    return 1
}

restore_notification_access() {
    local command=disallow_listener
    if grep -Fx "$phone_listener" "$phone_out/listener-before.txt" >/dev/null; then command=allow_listener; fi
    adb_shell cmd notification "$command" "$phone_listener" "$phone_user" > "$phone_out/listener-restore.txt" 2>&1 || return 1
    wait_listener_state restore listener-after.txt || return 1
    phone_notification_access=0
}

phase 'Checking real Android notification shell syntax and original listener access'
phone_notification_help_status=0
adb_shell cmd notification help > "$phone_out/notification-help.txt" 2>&1 || phone_notification_help_status=$?
printf '%s\n' "$phone_notification_help_status" > "$phone_out/notification-help-exit.txt"
# Android 36 prints complete ShellCommand help with exit 255. Validate its actual
# supported commands below; a usage exit is not a failed listener operation.
case "$phone_notification_help_status" in 0|255) ;; *) exit "$phone_notification_help_status" ;; esac
adb_shell cmd notification post --help > "$phone_out/notification-post-help.txt" 2>&1 || true
python3 - "$phone_repo/scripts" "$phone_out" <<'PY'
import pathlib,sys
sys.path.insert(0,sys.argv[1])
from verify_android_bootstrap import verify_help
root=pathlib.Path(sys.argv[2])
verify_help((root/'notification-help.txt').read_text(),(root/'notification-post-help.txt').read_text())
PY
listener_state > "$phone_out/listener-before.txt"
phone_notification_access=1
phase 'Granting only the app listener and posting a unique native notification in the private emulator'
adb_shell cmd notification allow_listener "$phone_listener" "$phone_user" > "$phone_out/listener-grant.txt" 2>&1
wait_listener_state grant listener-granted.txt
adb_shell cmd notification post -t "FerroUI-bootstrap-$phone_device_nonce" "ferroui-smoke-$phone_device_nonce" "Native-listener-$phone_device_nonce" > "$phone_out/notification-post.txt" 2>&1
wait_bootstrap notifications-admitted

phase 'Opening the mounted Action Center with a real native tap and checking the rendered fixture'
read -r phone_notice_x phone_notice_y < <(python3 - "$phone_out/device-notifications-admitted.json" <<'PY'
import json,sys
p=json.load(open(sys.argv[1])); r=p['status_bar']; s=p['render_scaling']
assert r['fully_visible'] and r['width']>0 and r['height']>0 and s>0
print(round((r['x']+r['width']/2)*s),round((r['y']+r['height']/2)*s))
PY
)
[[ "$phone_notice_x" =~ ^[0-9]+$ ]] && [[ "$phone_notice_y" =~ ^[0-9]+$ ]]
adb_shell input tap "$phone_notice_x" "$phone_notice_y"
phone_notice_visible=0
for phone_notice_scroll in 0 1 2 3; do
    if wait_bootstrap notifications-visible 10; then phone_notice_visible=1; break; fi
    # Only scroll the observed Action Center; never navigate or inject a response through debug code.
    read -r phone_notice_x phone_notice_from phone_notice_to < <(python3 - "$phone_out/bootstrap-current.json" <<'PY'
import json,sys
p=json.load(open(sys.argv[1])); r=p['action_center']; s=p['render_scaling']
assert r['fully_visible'] and r['height']>300 and s>0
print(round((r['x']+r['width']/2)*s),round((r['y']+r['height']-65)*s),round((r['y']+r['height']*0.45)*s))
PY
)
    [[ "$phone_notice_x" =~ ^[0-9]+$ ]] && [[ "$phone_notice_from" =~ ^[0-9]+$ ]] && [[ "$phone_notice_to" =~ ^[0-9]+$ ]]
    adb_shell input swipe "$phone_notice_x" "$phone_notice_from" "$phone_notice_x" "$phone_notice_to" 500
done
[ "$phone_notice_visible" = 1 ]
emu_screencap "$phone_out/device-action-center.png"

phase 'Revoking the listener and requiring shared/private rendered rows to clear'
adb_shell cmd notification disallow_listener "$phone_listener" "$phone_user" > "$phone_out/listener-revoke.txt" 2>&1
wait_listener_state revoke listener-revoked.txt
wait_bootstrap notifications-revoked
emu_screencap "$phone_out/device-action-center-revoked.png"
adb_shell input keyevent KEYCODE_BACK
wait_bootstrap start-restored
restore_notification_access
