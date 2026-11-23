import {notificationPermission} from './system-notifications.js';
import {readStorageEnvironment} from './storage-environment.js';

let current;
export function refreshEnvironment() { current?.refresh(); }

// Observations are ephemeral. Reading these facts never requests permission.
// Return an owned cleanup so a replaced runtime cannot publish stale promises.
export function observeEnvironment(runtime, platform = globalThis) {
  current?.dispose();
  const navigator = platform.navigator || {};
  let battery, previous = '', disposed = false, inFlight = false, pending = false;
  let storage = {};
  const listeners = [];
  const listen = (target, type, handler) => {
    target?.addEventListener?.(type, handler);
    listeners.push(() => target?.removeEventListener?.(type, handler));
  };
  const publish = () => {
    if (disposed) return;
    const next = JSON.stringify({
      platform: 'browser',
      online: typeof navigator.onLine === 'boolean' ? navigator.onLine : null,
      battery_percent: battery && Number.isFinite(battery.level)
        ? Math.round(Math.min(1, Math.max(0, battery.level)) * 100) : null,
      charging: typeof battery?.charging === 'boolean' ? battery.charging : null,
      notification_permission: notificationPermission(platform),
      media_session_supported: Boolean(navigator.mediaSession),
      persistent_storage_supported: Boolean(navigator.storage?.persist),
      ...storage,
    });
    if (next !== previous) { previous = next; runtime.setEnvironment(next); }
  };
  const refresh = () => {
    publish();
    if (disposed) return;
    if (inFlight) { pending = true; return; }
    inFlight = true;
    void readStorageEnvironment(navigator.storage).then(value => {
      if (!disposed) { storage = value; publish(); }
    }).finally(() => {
      inFlight = false;
      if (pending && !disposed) { pending = false; refresh(); }
    });
  };
  const dispose = () => {
    if (disposed) return;
    disposed = true;
    for (const remove of listeners) remove();
    if (current?.dispose === dispose) current = undefined;
  };
  current = {refresh, dispose};
  listen(platform, 'online', publish);
  listen(platform, 'offline', publish);
  listen(platform, 'focus', refresh);
  listen(platform, 'pageshow', refresh);
  listen(platform.document, 'visibilitychange', () => {
    if (!platform.document.hidden) refresh();
  });
  refresh();
  if (navigator.getBattery) {
    Promise.resolve().then(() => navigator.getBattery()).then(value => {
      if (disposed) return;
      battery = value;
      listen(battery, 'levelchange', publish);
      listen(battery, 'chargingchange', publish);
      publish();
    }).catch(() => {}); // Unsupported/blocked battery access remains unknown.
  }
  if (navigator.permissions?.query && platform.Notification) {
    Promise.resolve().then(() => navigator.permissions.query({name: 'notifications'})).then(permission => {
      if (!disposed) listen(permission, 'change', publish);
    }).catch(() => {}); // Not all browsers implement this permission descriptor.
  }
  return dispose;
}
