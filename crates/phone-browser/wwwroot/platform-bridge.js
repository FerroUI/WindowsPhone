import {notificationPermission, requestNotificationPermission, showSystemNotification} from './system-notifications.js';
import {webUrl} from './urls.js';

const MAX_PENDING = 8;
const validId = id => Number.isInteger(id) && id > 0 && id <= 0xffffffff;
const accepted = message => ({kind: 'accepted', value: {message}});
const text = (value, max, empty = true) => typeof value === 'string' && value.length <= max &&
  (empty || value.trim().length > 0) && !/\p{Cc}/u.test(value);

function policyAllows(platform, feature) {
  const policy = platform.document?.permissionsPolicy || platform.document?.featurePolicy;
  try { return !policy?.allowsFeature || policy.allowsFeature(feature); }
  catch { return false; }
}

export function describeBrowser(mode = 'app', platform = globalThis) {
  if (!['app', 'embedded'].includes(mode)) {
    throw new Error('The browser supports app and embedded modes; it cannot replace a device launcher or desktop shell.');
  }
  const navigator = platform.navigator || {};
  const secure = platform.isSecureContext === true;
  const features = [];
  if (secure && typeof navigator.share === 'function' && policyAllows(platform, 'web-share')) features.push('nativeSharing');
  if (typeof navigator.vibrate === 'function') features.push('haptics');
  if (secure && platform.Notification && typeof platform.Notification.requestPermission === 'function') features.push('notifications');
  if (secure && typeof navigator.geolocation?.watchPosition === 'function' &&
      typeof navigator.geolocation?.clearWatch === 'function' && policyAllows(platform, 'geolocation')) features.push('location');
  return {platform: 'browser', mode, launcher: 'unsupported', shell: 'unsupported', features,
    detail: 'Available services depend on your browser, site permissions and device support. Installed apps and the device Home screen are unavailable here.'};
}

export function browserEnvironment(platform = globalThis) {
  return {platform: 'browser',
    online: typeof platform.navigator?.onLine === 'boolean' ? platform.navigator.onLine : null,
    notification_permission: platform.isSecureContext === true ? notificationPermission(platform) : 'unsupported',
  };
}

function validate(request) {
  if (!request || typeof request !== 'object' || Array.isArray(request)) throw new Error('Invalid platform request.');
  switch (request.kind) {
    case 'refresh': case 'requestNotifications': case 'locate': return;
    case 'haptic':
      if (!['selection', 'light', 'medium', 'heavy'].includes(request.feedback)) throw new Error('Invalid haptic feedback.');
      return;
    case 'shareText':
      if (!text(request.title, 512) || typeof request.text !== 'string' || request.text.length > 65536 || request.text.includes('\0') || !text(request.url, 4096)) throw new Error('Invalid shared content.');
      if (!request.title.trim() && !request.text.trim() && !request.url.trim()) throw new Error('Choose text or a link to share.');
      return;
    case 'notify':
      if (!text(request.title, 512, false) || typeof request.body !== 'string' || request.body.length > 4096 || request.body.includes('\0')) throw new Error('Invalid notification content.');
      return;
    default: throw new Error('This integration is unavailable in the browser host.');
  }
}
const requestFeature = kind => ({shareText: 'nativeSharing', haptic: 'haptics',
  requestNotifications: 'notifications', notify: 'notifications', locate: 'location'})[kind];

// Browser side effects start within start(), before it returns to WebAssembly.
// Awaiting the Rust future is deliberately not what initiates a gesture API.
export function createBrowserPlatformBridge(runtime, platform = globalThis, options = {}) {
  let api = runtime, closed = false, sharePending = false, permissionPending;
  let vibrating = false;
  const pending = new Map();
  const schedule = options.setTimeout || platform.setTimeout?.bind(platform) || setTimeout;
  const unschedule = options.clearTimeout || platform.clearTimeout?.bind(platform) || clearTimeout;
  const settings = options.settings || (() => JSON.parse(api.hostState()).settings);
  const preference = name => {
    if (settings()?.[name] !== true) throw new Error(`Turn on ${name === 'vibration' ? 'vibration' : name} in the phone's Settings first.`);
  };
  const live = entry => !closed && !entry.cancelled && pending.get(entry.id) === entry;
  const finish = (entry, result, error) => {
    if (!live(entry)) return false;
    pending.delete(entry.id);
    entry.cancelled = true;
    unschedule(entry.timer);
    entry.cleanup?.();
    if (error) return api?.failPlatformTask(entry.id, String(error?.message || error).slice(0, 2048));
    return api?.completePlatformTask(entry.id, JSON.stringify(result));
  };
  const cancel = id => {
    const entry = pending.get(id);
    if (!entry) return false;
    pending.delete(id);
    entry.cancelled = true;
    unschedule(entry.timer);
    entry.cleanup?.();
    return true;
  };
  function locate(entry) {
    preference('location');
    let watch, settled = false;
    const geolocation = platform.navigator.geolocation;
    const clear = () => { if (watch !== undefined) { geolocation.clearWatch(watch); watch = undefined; } };
    entry.cleanup = clear;
    return new Promise((resolve, reject) => {
      const success = position => {
        if (settled || !live(entry)) return;
        settled = true;
        clear();
        const {latitude, longitude} = position?.coords || {};
        if (!Number.isFinite(latitude) || !Number.isFinite(longitude) || Math.abs(latitude) > 90 || Math.abs(longitude) > 180) {
          reject(new Error('The browser returned an invalid location.')); return;
        }
        resolve({kind: 'location', value: {latitude, longitude}});
      };
      const failure = error => {
        if (settled || !live(entry)) return;
        settled = true; clear();
        reject(new Error(error?.code === 1 ? 'Location permission was denied by the browser or device.' : error?.code === 3 ? 'The location request timed out.' : 'The browser could not obtain a location.'));
      };
      watch = geolocation.watchPosition(success, failure, {enableHighAccuracy: false, maximumAge: 300000, timeout: 15000});
      // Browser callbacks are asynchronous, but a synchronous injected/native
      // adapter must still have its returned watch cancelled after completion.
      if (settled || !live(entry)) clear();
    });
  }
  function perform(entry, request, mode) {
    switch (request.kind) {
      case 'refresh': return {kind: 'state', value: describeBrowser(mode, platform)};
      case 'locate': return locate(entry);
      case 'haptic': {
        preference('vibration');
        if (platform.document?.hidden) throw new Error('Vibration requires a visible page.');
        const duration = {selection: 8, light: 15, medium: 25, heavy: 40}[request.feedback];
        if (platform.navigator.vibrate(duration) !== true) throw new Error('The browser declined the vibration request.');
        vibrating = true;
        return accepted('Vibration requested.');
      }
      case 'shareText': {
        if (sharePending) throw new Error('A browser share operation is already open.');
        const payload = {};
        if (request.title) payload.title = request.title;
        if (request.text) payload.text = request.text;
        if (request.url) payload.url = webUrl(request.url);
        if (platform.navigator.canShare && !platform.navigator.canShare(payload)) throw new Error('The browser cannot share this content.');
        // No preceding await: this must consume the original user activation.
        const sharing = platform.navigator.share(payload);
        sharePending = true;
        return Promise.resolve(sharing).then(() => accepted('The browser accepted the share.'), error => {
          if (error?.name === 'AbortError') return accepted('Sharing cancelled.');
          throw error;
        }).finally(() => { sharePending = false; });
      }
      case 'requestNotifications': {
        // A permission UI cannot be programmatically dismissed. Keep one
        // actual browser request even if a caller cancels its result future.
        if (!permissionPending) {
          permissionPending = requestNotificationPermission(platform).finally(() => { permissionPending = undefined; });
        }
        return permissionPending.then(permission => accepted(permission === 'granted' ? 'Device notifications are allowed.' :
          permission === 'denied' ? 'Device notifications are blocked in browser settings.' : 'Device notifications were not enabled.'));
      }
      case 'notify':
        preference('notifications');
        return showSystemNotification(request.title, request.body, platform, () => live(entry)).then(shown => {
          if (!shown) throw new Error('Device notifications are not allowed. Use the explicit notification permission command first.');
          return accepted('The browser accepted the notification.');
        });
    }
  }
  const start = (id, mode, request) => {
    if (closed) throw new Error('The browser platform host is closed.');
    if (!validId(id) || pending.has(id)) throw new Error('Invalid or duplicate platform request identity.');
    if (pending.size >= MAX_PENDING) throw new Error('Too many browser platform requests are in progress.');
    validate(request);
    const state = describeBrowser(mode, platform);
    const feature = requestFeature(request.kind);
    if (feature && !state.features.includes(feature)) throw new Error('This service is unavailable or blocked by this browser context.');
    const entry = {id, kind: request.kind, cancelled: false};
    pending.set(id, entry);
    entry.timer = schedule(() => finish(entry, undefined, new Error('The browser platform request timed out.')), request.kind === 'locate' ? 15000 : 30000);
    try {
      const operation = perform(entry, request, mode);
      Promise.resolve(operation).then(result => finish(entry, result), error => finish(entry, undefined, error));
    } catch (error) {
      // Rejections are delivered through the same task boundary. Do not invoke
      // Rust exports reentrantly while Rust is calling this imported function.
      Promise.resolve().then(() => finish(entry, undefined, error));
    }
  };
  const suspend = () => {
    for (const entry of [...pending.values()]) {
      if (entry.kind === 'locate') finish(entry, undefined, new Error('Location was cancelled when the page was suspended.'));
    }
    if (vibrating) { vibrating = false; try { platform.navigator.vibrate(0); } catch {} }
  };
  const close = () => {
    if (closed) return;
    closed = true;
    for (const id of [...pending.keys()]) cancel(id);
    if (vibrating) { vibrating = false; try { platform.navigator.vibrate(0); } catch {} }
    // Late permission/share promises retain no WebAssembly runtime reference.
    api = undefined;
  };
  return {start, cancel, suspend, close, get pendingCount() { return pending.size; }};
}

let current;
export function attachPlatformRuntime(runtime) {
  current?.close();
  const bridge = createBrowserPlatformBridge(runtime);
  current = bridge;
  return () => { bridge.close(); if (current === bridge) current = undefined; };
}
export function platformMode() {
  const explicit = document.getElementById('out')?.dataset.phoneMode || new URLSearchParams(location.search).get('mode');
  if (explicit) return explicit;
  return window.top === window ? 'app' : 'embedded';
}
export const platformState = mode => JSON.stringify(describeBrowser(mode));
export const platformEnvironment = () => JSON.stringify(browserEnvironment());
export function startPlatformTask(id, mode, json) {
  if (!current) throw new Error('The browser platform bridge has not been attached.');
  if (typeof json !== 'string' || json.length > 512 * 1024) throw new Error('The platform request exceeds its encoding limit.');
  current.start(id, mode, JSON.parse(json));
}
export const cancelPlatformTask = id => current?.cancel(id);
export const suspendPlatform = () => current?.suspend();
export const closePlatform = () => current?.close();
