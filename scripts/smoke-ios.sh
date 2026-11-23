#!/usr/bin/env bash
# Creates its own simulator; never installs into or erases a user's device.
set -euo pipefail
app="${1:?Pass the built phone-ios.app bundle}"
out="${2:-${RUNNER_TEMP:-/tmp}/phone-ios-ui}"
mkdir -p "$out"
out="$(cd "$out" && pwd)"
bundle_id=org.ferroui.windowsphone
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
udid="$(xcrun simctl create "WindowsPhone-CI-${GITHUB_RUN_ID:-$$}" "$device_type" "$runtime")"
cleanup() {
  xcrun simctl terminate "$udid" "$bundle_id" >/dev/null 2>&1 || true
  xcrun simctl shutdown "$udid" >/dev/null 2>&1 || true
  xcrun simctl delete "$udid" >/dev/null 2>&1 || true
}
trap cleanup EXIT
xcrun simctl boot "$udid"
xcrun simctl bootstatus "$udid" -b
xcrun simctl install "$udid" "$app"
SIMCTL_CHILD_PHONE_IOS_SMOKE=1 SIMCTL_CHILD_RUST_BACKTRACE=1 \
  xcrun simctl launch --console-pty --terminate-running-process "$udid" "$bundle_id" \
  > "$out/console.log" 2>&1 &
launch=$!
container="$(xcrun simctl get_app_container "$udid" "$bundle_id" data)"
status="$container/Documents/phone-ios-smoke.json"
wait_for_route() {
  local expected="$1"
  for attempt in $(seq 1 60); do
    if ! kill -0 "$launch" 2>/dev/null; then cat "$out/console.log" >&2; return 1; fi
    if python3 - "$status" "$expected" <<'PY'
import json,pathlib,sys
p=pathlib.Path(sys.argv[1])
try:
    d=json.loads(p.read_text())
    mounted=d['ready'] and d['platform']=='ios' and d['width']>=300 and d['height']>=400
    expected=sys.argv[2]
    route=d['route']
    matches=route=='Start' if expected=='home' else isinstance(route,dict) and route.get('Extension',{}).get('app')==expected
except (OSError,ValueError,KeyError,TypeError): raise SystemExit(1)
raise SystemExit(0 if mounted and matches else 1)
PY
    then return 0; fi
    sleep 0.5
  done
  cat "$out/console.log" >&2
  return 1
}
wait_for_route home
cp "$status" "$out/start.json"
xcrun simctl io "$udid" screenshot "$out/start.png"
xcrun simctl openurl "$udid" 'ferroui-phone://app/org.ferroui.wallet'
wait_for_route org.ferroui.wallet
cp "$status" "$out/wallet.json"
xcrun simctl io "$udid" screenshot "$out/wallet.png"
xcrun simctl openurl "$udid" 'ferroui-phone://home'
wait_for_route home
cp "$status" "$out/home-restored.json"
xcrun simctl io "$udid" screenshot "$out/home-restored.png"
printf '%s\n' 'UIKit host mounted, app activation and return Home passed.'
