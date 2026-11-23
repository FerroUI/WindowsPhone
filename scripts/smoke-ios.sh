#!/usr/bin/env bash
# Creates its own simulator; never installs into or erases a user's device.
set -euo pipefail
app="${1:?Pass the built phone-ios.app bundle}"
root="$(cd "$(dirname "$0")/.." && pwd)"
out="${2:-${RUNNER_TEMP:-/tmp}/phone-ios-ui}"
mkdir -p "$out"
out="$(cd "$out" && pwd)"
bundle_id=org.ferroui.windowsphone
phase() {
  printf '%s\n' "$1" | tee "$out/phase.txt"
}
phase 'Building the public XCTest system URL driver'
prompt_build="$(mktemp -d "${TMPDIR:-/tmp}/phone-ios-prompt.XXXXXX")"
if ! xcodebuild build-for-testing \
  -project "$root/scripts/ios-ui-tests/PhonePromptTests.xcodeproj" \
  -scheme PhonePromptTests -destination 'generic/platform=iOS Simulator' \
  -derivedDataPath "$prompt_build" CODE_SIGNING_ALLOWED=NO \
  > "$out/xctest-build.log" 2>&1; then
  tail -80 "$out/xctest-build.log" >&2
  exit 1
fi
test_run="$(python3 - "$prompt_build/Build/Products" <<'PY'
import pathlib,sys
files=list(pathlib.Path(sys.argv[1]).glob('*.xctestrun'))
if len(files)!=1: raise SystemExit('Expected one built XCTest runner.')
print(files[0])
PY
)"
phase 'Selecting the simulator runtime'
spec="$(xcrun simctl list --json | python3 -c '
import json,sys
d=json.load(sys.stdin)
r=[x for x in d["runtimes"] if x.get("isAvailable") and x["identifier"].startswith("com.apple.CoreSimulator.SimRuntime.iOS-")]
if not r: raise SystemExit("No available iOS simulator runtime")
r.sort(key=lambda x:tuple(int(p) for p in x["version"].split(".")),reverse=True)
version=tuple(int(p) for p in r[0]["version"].split("."))
number=sum(value << shift for value,shift in zip((*version,0,0)[:3],(16,8,0)))
devices=[x for x in d["devicetypes"] if x["name"].startswith("iPhone")
    and x.get("minRuntimeVersion",0)<=number<=x.get("maxRuntimeVersion",0xffffffff)]
if not devices: raise SystemExit("No iPhone simulator supports the selected runtime")
print(r[0]["identifier"],devices[-1]["identifier"])
')"
read -r runtime device_type <<< "$spec"
printf '%s\n' "$runtime" "$device_type" > "$out/simulator.txt"
udid="$(xcrun simctl create "WindowsPhone-CI-${GITHUB_RUN_ID:-$$}" "$device_type" "$runtime")"
save_observations() {
  if [ -n "${status:-}" ] && [ -f "$status" ]; then
    cp "$status" "$out/last-observation.json" || true
  fi
  if [ -n "${ui_status:-}" ] && [ -f "$ui_status" ]; then
    cp "$ui_status" "$out/prompt-handler.json" || true
  fi
  if [ -n "${device_status:-}" ] && [ -f "$device_status" ]; then
    cp "$device_status" "$out/device-tests.json" || true
  fi
}
cleanup() {
  local result=$?
  save_observations
  if [ "$result" != 0 ]; then
    printf 'iOS verification failed during: %s\n' "$(cat "$out/phase.txt")" >&2
    # Diagnostic work begins only after failure. Each subprocess is bounded,
    # and the native process must match this simulator's installed executable.
    python3 "$root/scripts/collect-ios-failure.py" "$udid" "$bundle_id" "$out" || true
    if [ -n "${status:-}" ] && [ -f "$status" ]; then
      cp "$status" "$out/failure.json" || true
      cat "$out/failure.json" >&2 || true
    fi
    xcrun simctl io "$udid" screenshot "$out/failure.png" >/dev/null 2>&1 || true
  fi
  if [ -n "${ui_done:-}" ]; then touch "$ui_done" || true; fi
  if [ -n "${ui_status:-}" ] && [ -f "$ui_status" ]; then
    cp "$ui_status" "$out/prompt-handler.json" || true
  fi
  if [ -n "${ui_test:-}" ] && kill -0 "$ui_test" 2>/dev/null; then
    for attempt in $(seq 1 20); do
      if ! kill -0 "$ui_test" 2>/dev/null; then break; fi
      sleep 0.5
    done
    kill "$ui_test" 2>/dev/null || true
    wait "$ui_test" 2>/dev/null || true
  fi
  xcrun simctl terminate "$udid" "$bundle_id" >/dev/null 2>&1 || true
  xcrun simctl shutdown "$udid" >/dev/null 2>&1 || true
  xcrun simctl delete "$udid" >/dev/null 2>&1 || true
  return "$result"
}
trap cleanup EXIT
trap 'exit 130' INT
trap 'exit 143' TERM
phase 'Booting the simulator'
xcrun simctl boot "$udid"
xcrun simctl bootstatus "$udid" -b
phase 'Installing and launching the UIKit application'
xcrun simctl install "$udid" "$app"
xcrun simctl privacy "$udid" grant contacts "$bundle_id"
# Retain exact installation identity before launch: CoreSimulator may stop
# answering IPC precisely when the later timeout diagnostic needs a stack.
python3 "$root/scripts/collect-ios-failure.py" --prepare "$udid" "$bundle_id" "$out"
SIMCTL_CHILD_PHONE_IOS_SMOKE=1 SIMCTL_CHILD_PHONE_IOS_DEVICE_TEST=1 SIMCTL_CHILD_RUST_BACKTRACE=1 \
  xcrun simctl launch --console-pty --terminate-running-process "$udid" "$bundle_id" \
  > "$out/console.log" 2>&1 &
launch=$!
container="$(xcrun simctl get_app_container "$udid" "$bundle_id" data)"
status="$container/Documents/phone-ios-smoke.json"
device_status="$container/Documents/phone-ios-device-tests.json"
wait_for_route() {
  local expected="$1"
  local after_sequence="${2:-0}"
  local expected_uri="${3:-}"
  local baseline=
  if [ -n "$expected_uri" ]; then baseline="$out/start.json"; fi
  # A first boot on a hosted runner can spend over a minute scheduling UIKit.
  # Use a wall-clock bound; never treat a received URL alone as route success.
  local deadline=$((SECONDS + 90))
  while [ "$SECONDS" -lt "$deadline" ]; do
    save_observations
    if ! kill -0 "$launch" 2>/dev/null; then
      if wait "$launch"; then code=0; else code=$?; fi
      printf 'Simulator launch console exited with code %s while waiting for %s.\n' "$code" "$expected" >&2
      cat "$out/console.log" >&2
      return 1
    fi
    if python3 "$root/scripts/verify-ios-smoke.py" "$status" "$expected" "$after_sequence" "$expected_uri" --baseline "$baseline"; then
      if [ -n "$expected_uri" ]; then acknowledge_url; fi
      return 0
    fi
    sleep 0.5
  done
  printf 'Timed out waiting for the mounted route %s.\n' "$expected" >&2
  python3 "$root/scripts/verify-ios-smoke.py" "$status" "$expected" "$after_sequence" "$expected_uri" --baseline "$baseline" --explain || true
  cat "$out/console.log" >&2
  return 1
}
phase 'Waiting for the Start route'
wait_for_route home
cp "$status" "$out/start.json"
xcrun simctl io "$udid" screenshot "$out/start.png"
phase 'Verifying real Contacts CRUD and simulator hardware limits'
deadline=$((SECONDS + 120))
while [ "$SECONDS" -lt "$deadline" ]; do
  save_observations
  if python3 "$root/scripts/verify-ios-devices.py" "$device_status"; then break; fi
  if python3 - "$device_status" <<'PY'
import json,pathlib,sys
try: failed=json.loads(pathlib.Path(sys.argv[1]).read_text()).get('status')=='failed'
except (OSError,ValueError): failed=False
raise SystemExit(0 if failed else 1)
PY
  then cat "$device_status" >&2; exit 1; fi
  sleep 0.5
done
if [ "$SECONDS" -ge "$deadline" ]; then
  echo 'The native Contacts/hardware qualification did not finish.' >&2
  exit 1
fi
cp "$device_status" "$out/device-tests.json"
phase 'Starting the fixed-URL XCTest system driver'
xcodebuild test-without-building -xctestrun "$test_run" \
  -destination "platform=iOS Simulator,id=$udid" -parallel-testing-enabled NO \
  -only-testing:PhonePromptTests/PhonePromptTests/testOpenWindowsPhoneURLs \
  -resultBundlePath "$out/prompt-ui.xcresult" \
  > "$out/xctest.log" 2>&1 &
ui_test=$!
deadline=$((SECONDS + 120))
ui_container=
while [ "$SECONDS" -lt "$deadline" ]; do
  save_observations
  if ! kill -0 "$ui_test" 2>/dev/null; then cat "$out/xctest.log" >&2; exit 1; fi
  if [ -z "$ui_container" ]; then
    ui_container="$(xcrun simctl get_app_container "$udid" org.ferroui.phoneprompttests.xctrunner data 2>/dev/null || true)"
  fi
  if [ -n "$ui_container" ]; then
    ui_status="$ui_container/Documents/phone-prompt-status.json"
    ui_done="$ui_container/Documents/phone-prompt-finished"
    if python3 - "$ui_status" <<'PY'
import json,pathlib,sys
try: ready=json.loads(pathlib.Path(sys.argv[1]).read_text()).get('phase')=='ready'
except (OSError,ValueError): ready=False
raise SystemExit(0 if ready else 1)
PY
    then break; fi
  fi
  sleep 0.5
done
if [ "$SECONDS" -ge "$deadline" ]; then
  echo 'The XCTest system URL driver did not become ready.' >&2
  cat "$out/xctest.log" >&2
  exit 1
fi
url_request_sequence=0
acknowledge_url() {
  python3 - "$ui_container/Documents/phone-url-verified.json" "$url_request_sequence" <<'PY'
import json,pathlib,sys
path=pathlib.Path(sys.argv[1])
temporary=path.with_suffix('.tmp')
temporary.write_text(json.dumps({'sequence':int(sys.argv[2])}))
temporary.replace(path)
PY
}
request_url() {
  url_request_sequence=$((url_request_sequence + 1))
  python3 - "$ui_container/Documents/phone-url-request.json" "$out/request-$url_request_sequence.json" "$url_request_sequence" "$1" <<'PY'
import json,pathlib,sys,time
path=pathlib.Path(sys.argv[1])
request=json.dumps({'sequence':int(sys.argv[3]), 'uri':sys.argv[4], 'issued_at_unix_ms':time.time_ns()//1_000_000})
pathlib.Path(sys.argv[2]).write_text(request)
temporary=path.with_suffix('.tmp')
temporary.write_text(request)
temporary.replace(path)
PY
}
sequence="$(python3 -c 'import json,sys; print(json.load(open(sys.argv[1]))["sequence"])' "$status")"
phase 'Delivering the Wallet URL through the system XCTest API'
request_url 'ferroui-phone://app/org.ferroui.wallet'
phase 'Waiting for the Wallet route'
wait_for_route org.ferroui.wallet "$sequence" 'ferroui-phone://app/org.ferroui.wallet'
cp "$status" "$out/wallet.json"
xcrun simctl io "$udid" screenshot "$out/wallet.png"
for spec in \
  'phone|phone.phone|ferroui-phone://app/phone.phone?path=dial/KzEgMjAyIDU1NSAwMTQ3' \
  'people|phone.people|ferroui-phone://app/phone.people?path=device' \
  'messages|phone.messaging|ferroui-phone://app/phone.messaging?path=compose' \
  'settings|phone.settings|ferroui-phone://app/phone.settings?path=device'; do
  IFS='|' read -r capture route uri <<< "$spec"
  sequence="$(python3 -c 'import json,sys; print(json.load(open(sys.argv[1]))["sequence"])' "$status")"
  phase "Delivering the $capture URL through the system XCTest API"
  request_url "$uri"
  wait_for_route "$route" "$sequence" "$uri"
  cp "$status" "$out/$capture.json"
  xcrun simctl io "$udid" screenshot "$out/$capture.png"
done
sequence="$(python3 -c 'import json,sys; print(json.load(open(sys.argv[1]))["sequence"])' "$status")"
phase 'Delivering the Home URL through the system XCTest API'
request_url 'ferroui-phone://home'
phase 'Waiting for the restored Start route'
wait_for_route home "$sequence" 'ferroui-phone://home'
cp "$status" "$out/home-restored.json"
xcrun simctl io "$udid" screenshot "$out/home-restored.png"
touch "$ui_done"
if ! wait "$ui_test"; then cat "$out/xctest.log" >&2; exit 1; fi
cp "$ui_status" "$out/prompt-handler.json"
phase 'UIKit mounting, six native activations, Contacts CRUD and simulator hardware limits passed.'
