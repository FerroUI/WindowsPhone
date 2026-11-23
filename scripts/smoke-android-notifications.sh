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

restore_notification_access() {
    local command=disallow_listener stop=$((SECONDS + 30))
    if grep -Fx "$phone_listener" "$phone_out/listener-before.txt" >/dev/null; then command=allow_listener; fi
    adb_shell cmd notification "$command" "$phone_listener" "$phone_user" > "$phone_out/listener-restore.txt" 2>&1 || return 1
    while [ "$SECONDS" -lt "$stop" ]; do
        listener_state > "$phone_out/listener-after.txt" || return 1
        if cmp -s "$phone_out/listener-before.txt" "$phone_out/listener-after.txt"; then
            phone_notification_access=0
            return 0
        fi
        sleep 0.5
    done
    printf 'Original notification-listener access was not restored\n' >&2
    return 1
}

phase 'Checking real Android notification shell syntax and original listener access'
adb_shell cmd notification help > "$phone_out/notification-help.txt" 2>&1
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
listener_state > "$phone_out/listener-granted.txt"
python3 - "$phone_out/listener-before.txt" "$phone_out/listener-granted.txt" "$phone_listener" <<'PY'
from pathlib import Path
import sys
before=set(Path(sys.argv[1]).read_text().splitlines())
after=set(Path(sys.argv[2]).read_text().splitlines())
assert after == before | {sys.argv[3]}, 'The command changed unrelated listener access'
PY
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
listener_state > "$phone_out/listener-revoked.txt"
python3 - "$phone_out/listener-before.txt" "$phone_out/listener-revoked.txt" "$phone_listener" <<'PY'
from pathlib import Path
import sys
before=set(Path(sys.argv[1]).read_text().splitlines())
after=set(Path(sys.argv[2]).read_text().splitlines())
assert after == before - {sys.argv[3]}, 'Listener revocation changed unrelated access or did not complete'
PY
wait_bootstrap notifications-revoked
emu_screencap "$phone_out/device-action-center-revoked.png"
adb_shell input keyevent KEYCODE_BACK
wait_bootstrap start-restored
restore_notification_access
