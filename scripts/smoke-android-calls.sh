#!/usr/bin/env bash
# Sourced after stable roles, before any modem traffic, on the owned fresh AVD.
set -euo pipefail
if [ "${emulator_started:-0}" != 1 ] || [ "${FERROUI_EMU_REUSE:-1}" != 0 ] \
    || [[ "${FERROUI_AVD:-}" != phone_ui_* ]] || ! [[ "${phone_device_nonce:-}" =~ ^[0-9a-f]{32}$ ]]; then
    printf '%s\n' 'Refusing a CallLog fixture outside the explicitly owned fresh AVD session.' >&2
    exit 1
fi
verify_private_avd
phase 'Verifying real bounded CallLog acknowledgment without initiating a call'
python3 - "$phone_out/call-fixture-request.json" "$phone_device_nonce" "$FERROUI_AVD" <<'PY'
import json,pathlib,sys,time
pathlib.Path(sys.argv[1]).write_text(json.dumps({'nonce':sys.argv[2], 'avd':sys.argv[3],
    'requested_at':time.time_ns()//1_000_000, 'phase':'after stable roles, before modem traffic'})+'\n')
PY
emu_write_file "$phone_package" windows-phone/phone-call-fixture.enabled "$phone_device_nonce"
phone_call_fixture_passed=0
phone_call_fixture_stop=$((SECONDS + 90))
while [ "$SECONDS" -lt "$phone_call_fixture_stop" ]; do
    emu_read_file "$phone_package" windows-phone/phone-call-fixture.json > "$phone_out/call-fixture-current.tmp"
    mv "$phone_out/call-fixture-current.tmp" "$phone_out/call-fixture-current.json"
    if [ -s "$phone_out/call-fixture-current.json" ]; then
        printf '%s\n' "$(<"$phone_out/call-fixture-current.json")" >> "$phone_out/call-fixture-observations.jsonl"
    fi
    if python3 "$phone_repo/scripts/verify_android_calls.py" "$phone_out/call-fixture-current.json" "$phone_device_nonce" \
        > "$phone_out/call-fixture-verification.txt" 2> "$phone_out/call-fixture-last-error.txt"; then
        cp "$phone_out/call-fixture-current.json" "$phone_out/call-fixture.json"
        phone_call_fixture_passed=1
        break
    fi
    if python3 - "$phone_out/call-fixture-current.json" "$phone_device_nonce" <<'PY'
import json,sys
try:
    proof=json.load(open(sys.argv[1]))
    failed=proof.get('nonce')==sys.argv[2] and proof.get('result') in ('failed','passed')
except (OSError,ValueError): failed=False
raise SystemExit(0 if failed else 1)
PY
    then
        # A completed failed assertion is final. Preserve it and stop; no replay.
        cp "$phone_out/call-fixture-current.json" "$phone_out/call-fixture.json"
        break
    fi
    sleep 0.5
done
adb_shell "run-as $phone_package rm -f files/windows-phone/phone-call-fixture.enabled"
if [ "$phone_call_fixture_passed" != 1 ]; then
    printf 'CallLog fixture failed: %s\n' "$(cat "$phone_out/call-fixture-last-error.txt")" >&2
    return 1
fi
cat "$phone_out/call-fixture-verification.txt"
