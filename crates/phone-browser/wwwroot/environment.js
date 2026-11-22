// Optional platform observations. No permission prompt is requested here.
export function observeEnvironment(runtime) {
  let battery;
  let previous = '';
  const publish = () => {
    const next = JSON.stringify({
      online: navigator.onLine,
      battery_percent: battery && Number.isFinite(battery.level)
        ? Math.round(Math.min(1, Math.max(0, battery.level)) * 100) : null,
      charging: battery ? battery.charging : null,
    });
    if (next !== previous) {
      previous = next;
      runtime.setEnvironment(next);
    }
  };
  addEventListener('online', publish);
  addEventListener('offline', publish);
  publish();
  if (navigator.getBattery) {
    navigator.getBattery().then(value => {
      battery = value;
      battery.addEventListener('levelchange', publish);
      battery.addEventListener('chargingchange', publish);
      publish();
    }).catch(() => {}); // Unsupported/blocked battery access remains unknown.
  }
}
