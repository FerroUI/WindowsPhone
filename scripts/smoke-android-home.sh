#!/usr/bin/env bash
# Sourced only after smoke-android.sh creates and guards its exact ephemeral AVD.
[ "${emulator_started:-0}" = 1 ] && [ "${FERROUI_EMU_REUSE:-1}" = 0 ]
[[ "${FERROUI_AVD:-}" == phone_ui_* ]]
phone_package=org.ferroui.windowsphone.launcher
phone_component="$phone_package/org.ferroui.phone.PhoneActivity"
phone_nonce="$(python3 -c 'import secrets; print(secrets.token_hex(16))')"
phone_sequence=0
emu_write_file "$phone_package" windows-phone/phone-android-smoke.enabled "$phone_nonce"

home_capture() {
    local stage="$1"
    adb_shell dumpsys window > "$phone_out/home-role-$stage-window.txt"
    if ! grep 'mCurrentFocus' "$phone_out/home-role-$stage-window.txt" | grep -F "$phone_component" >/dev/null; then
        printf 'The selected HOME is not the focused native window: %s\n' "$phone_component" >&2
        return 1
    fi
    cp "$phone_out/current.json" "$phone_out/home-role-$stage.json"
    emu_screencap "$phone_out/home-role-$stage.png"
}

phase 'Qualifying and selecting the actual HOME role only inside the private emulator'
phone_home_role=1
adb_shell cmd role add-role-holder --user "$phone_user" android.app.role.HOME "$phone_package" > "$phone_out/home-select.txt" 2>&1
adb_shell cmd role get-role-holders --user "$phone_user" android.app.role.HOME | tr -d '\r' | sort > "$phone_out/home-selected.txt"
[ "$(cat "$phone_out/home-selected.txt")" = "$phone_package" ]
# Resolve HOME through Android's selected default, without an explicit component.
adb_shell am start -W -a android.intent.action.MAIN -c android.intent.category.HOME > "$phone_out/home-role-start-activation.txt" 2>&1
wait_for_view "$phone_nonce" ferroui-phone://home "$phone_sequence" home
home_capture start

phase 'Opening Wallet while the real HOME role is active'
phone_uri=ferroui-phone://app/org.ferroui.wallet
adb_shell am start -W -a android.intent.action.VIEW -d "$phone_uri" -n "$phone_component" > "$phone_out/home-role-wallet-activation.txt" 2>&1
wait_for_view "$phone_nonce" "$phone_uri" "$phone_sequence" home
home_capture wallet

phase 'Returning the actual selected HOME to Start through the Android HOME key'
adb_shell input keyevent KEYCODE_HOME
wait_for_view "$phone_nonce" ferroui-phone://home "$phone_sequence" home
home_capture returned

phase 'Keeping the active HOME focused after one real Back press at Start'
adb_shell input keyevent KEYCODE_BACK
wait_for_view "$phone_nonce" back:ferroui-phone://home "$phone_sequence" home
home_capture back-start

phase 'Restoring and verifying the original HOME holder inside the isolated emulator'
restore_home_role
adb_shell am force-stop "$phone_package"
