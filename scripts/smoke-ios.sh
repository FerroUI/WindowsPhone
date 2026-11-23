#!/usr/bin/env bash
# Creates its own simulator; never installs into or erases a user's device.
set -euo pipefail
app="${1:?Pass the built phone-ios.app bundle}"
out="${2:-${RUNNER_TEMP:-/tmp}/phone-ios-ui}"
mkdir -p "$out"
out="$(cd "$out" && pwd)"
bundle_id=org.ferroui.windowsphone
phase() {
  printf '%s\n' "$1" | tee "$out/phase.txt"
}
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
cleanup() {
  local result=$?
  if [ "$result" != 0 ]; then
    printf 'iOS verification failed during: %s\n' "$(cat "$out/phase.txt")" >&2
    if [ -n "${status:-}" ] && [ -f "$status" ]; then
      cp "$status" "$out/failure.json" || true
      cat "$out/failure.json" >&2 || true
    fi
    xcrun simctl io "$udid" screenshot "$out/failure.png" >/dev/null 2>&1 || true
  fi
  xcrun simctl terminate "$udid" "$bundle_id" >/dev/null 2>&1 || true
  xcrun simctl shutdown "$udid" >/dev/null 2>&1 || true
  xcrun simctl delete "$udid" >/dev/null 2>&1 || true
  return "$result"
}
trap cleanup EXIT
phase 'Booting the simulator'
xcrun simctl boot "$udid"
xcrun simctl bootstatus "$udid" -b
phase 'Installing and launching the UIKit application'
xcrun simctl install "$udid" "$app"
SIMCTL_CHILD_PHONE_IOS_SMOKE=1 SIMCTL_CHILD_RUST_BACKTRACE=1 \
  xcrun simctl launch --console-pty --terminate-running-process "$udid" "$bundle_id" \
  > "$out/console.log" 2>&1 &
launch=$!
container="$(xcrun simctl get_app_container "$udid" "$bundle_id" data)"
status="$container/Documents/phone-ios-smoke.json"
wait_for_route() {
  local expected="$1"
  local after_sequence="${2:-0}"
  # A first boot on a hosted runner can spend over a minute scheduling UIKit.
  # Use a wall-clock bound; never treat a received URL alone as route success.
  local deadline=$((SECONDS + 90))
  while [ "$SECONDS" -lt "$deadline" ]; do
    if ! kill -0 "$launch" 2>/dev/null; then
      if wait "$launch"; then code=0; else code=$?; fi
      printf 'Simulator launch console exited with code %s while waiting for %s.\n' "$code" "$expected" >&2
      cat "$out/console.log" >&2
      return 1
    fi
    if python3 - "$status" "$expected" "$after_sequence" <<'PY'
import json,pathlib,sys
p=pathlib.Path(sys.argv[1])
try:
    d=json.loads(p.read_text())
    mounted=d['ready'] and not d.get('closed',False) and d['platform']=='ios' and d['width']>=300 and d['height']>=400
    expected=sys.argv[2]
    route=d['route']
    matches=route=='Start' if expected=='home' else isinstance(route,dict) and route.get('Extension',{}).get('app')==expected
    fresh=d['sequence']>int(sys.argv[3])
except (OSError,ValueError,KeyError,TypeError): raise SystemExit(1)
raise SystemExit(0 if mounted and matches and fresh else 1)
PY
    then return 0; fi
    sleep 0.5
  done
  printf 'Timed out waiting for the mounted route %s.\n' "$expected" >&2
  cat "$out/console.log" >&2
  return 1
}
phase 'Waiting for the Start route'
wait_for_route home
cp "$status" "$out/start.json"
xcrun simctl io "$udid" screenshot "$out/start.png"
sequence="$(python3 -c 'import json,sys; print(json.load(open(sys.argv[1]))["sequence"])' "$status")"
phase 'Delivering the Wallet URL through UIKit'
xcrun simctl openurl "$udid" 'ferroui-phone://app/org.ferroui.wallet'
phase 'Waiting for the Wallet route'
wait_for_route org.ferroui.wallet "$sequence"
cp "$status" "$out/wallet.json"
xcrun simctl io "$udid" screenshot "$out/wallet.png"
sequence="$(python3 -c 'import json,sys; print(json.load(open(sys.argv[1]))["sequence"])' "$status")"
phase 'Delivering the Home URL through UIKit'
xcrun simctl openurl "$udid" 'ferroui-phone://home'
phase 'Waiting for the restored Start route'
wait_for_route home "$sequence"
cp "$status" "$out/home-restored.json"
xcrun simctl io "$udid" screenshot "$out/home-restored.png"
phase 'UIKit host mounted, app activation and return Home passed.'
