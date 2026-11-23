import assert from 'node:assert/strict';
import test from 'node:test';
import {createBrowserPlatformBridge, describeBrowser, browserEnvironment} from '../crates/phone-browser/wwwroot/platform-bridge.js';
import {bindRuntimeLifecycle, activationFromUrl} from '../crates/phone-browser/wwwroot/browser-lifecycle.js';

const settle = () => new Promise(resolve => setImmediate(resolve));
function fixture() {
  const calls = [], watches = new Map(), cleared = [], timers = new Map();
  let nextWatch = 0, nextTimer = 0;
  const platform = new EventTarget();
  platform.document = Object.assign(new EventTarget(), {hidden: false});
  platform.isSecureContext = true;
  platform.navigator = {onLine: true, geolocation: {
    watchPosition(success, failure, options) { const id = ++nextWatch; watches.set(id, {success, failure, options}); return id; },
    clearWatch(id) { cleared.push(id); watches.delete(id); },
  }};
  const preferences = {location: true, notifications: true, vibration: true};
  const runtime = {hostState: () => JSON.stringify({settings: preferences}),
    completePlatformTask(id, json) { calls.push({id, value: JSON.parse(json)}); return true; },
    failPlatformTask(id, error) { calls.push({id, error}); return true; },
  };
  const bridge = createBrowserPlatformBridge(runtime, platform, {
    setTimeout(callback, delay) {const id = ++nextTimer; timers.set(id, {callback, delay}); return id;},
    clearTimeout(id) {timers.delete(id);},
  });
  return {platform, preferences, runtime, bridge, calls, watches, cleared, timers};
}

test('browser roles and feature observations never imply an installed-app catalog or a native shell', () => {
  assert.deepEqual(describeBrowser('embedded', {isSecureContext: true}).features, []);
  for (const mode of ['app', 'embedded']) {
    const state = describeBrowser(mode, {isSecureContext: true, navigator: {share() {}, vibrate() {}}});
    assert.equal(state.mode, mode); assert.equal(state.launcher, 'unsupported'); assert.equal(state.shell, 'unsupported');
    assert.deepEqual(state.features, ['nativeSharing', 'haptics']);
  }
  for (const mode of ['launcher', 'shell', 'invented']) assert.throws(() => describeBrowser(mode), /supports app and embedded/);
  const blocked = {isSecureContext: false, navigator: {share() {}, geolocation: {watchPosition() {}, clearWatch() {}}}, Notification: {requestPermission() {}}};
  assert.deepEqual(describeBrowser('app', blocked).features, []);
  blocked.isSecureContext = true; blocked.document = {permissionsPolicy: {allowsFeature: () => false}};
  assert.deepEqual(describeBrowser('app', blocked).features, ['notifications']);
  assert.equal(browserEnvironment({isSecureContext: false}).notification_permission, 'unsupported');
});

test('native sharing starts synchronously, validates links and reports cancellation without a clipboard fallback', async () => {
  const f = fixture(); let shared = 0, resolve;
  f.platform.navigator.share = payload => {shared++; assert.equal(payload.url, 'https://example.org/'); return new Promise(done => {resolve = done;});};
  f.bridge.start(1, 'app', {kind: 'shareText', title: 'Example', text: '', url: 'https://example.org'});
  assert.equal(shared, 1, 'share must be called before start returns');
  resolve(); await settle(); assert.equal(f.calls[0].value.kind, 'accepted');
  f.bridge.start(2, 'app', {kind: 'shareText', title: '', text: 'A', url: 'javascript:alert(1)'});
  await settle(); assert.equal(shared, 1); assert.match(f.calls[1].error, /http or https/);
  f.platform.navigator.share = () => Promise.reject(Object.assign(Error('dismissed'), {name: 'AbortError'}));
  f.bridge.start(3, 'app', {kind: 'shareText', title: '', text: 'A', url: ''});
  await settle(); assert.equal(f.calls[2].value.value.message, 'Sharing cancelled.'); f.bridge.close();
});

test('cancelling a share result does not start concurrent OS share sheets', async () => {
  const f = fixture(); let started = 0, finish;
  f.platform.navigator.share = () => {started++; return new Promise(resolve => {finish = resolve;});};
  const request = {kind: 'shareText', title: '', text: 'A', url: ''};
  f.bridge.start(1, 'app', request); f.bridge.cancel(1);
  f.bridge.start(2, 'app', request); await settle();
  assert.equal(started, 1); assert.match(f.calls[0].error, /already open/);
  finish(); await settle(); assert.equal(f.calls.length, 1, 'cancelled completion is discarded');
  f.bridge.start(3, 'app', request); assert.equal(started, 2); f.bridge.close(); finish(); await settle();
  assert.equal(f.calls.length, 1);
});

test('notification permission preserves the original gesture and one prompt across cancelled consumers', async () => {
  const f = fixture(); let prompts = 0, finish;
  f.platform.Notification = {permission: 'default', requestPermission() {prompts++; return new Promise(resolve => {finish = () => {this.permission = 'granted'; resolve('granted');};});}};
  f.bridge.start(1, 'app', {kind: 'requestNotifications'}); assert.equal(prompts, 1);
  f.bridge.cancel(1); f.bridge.start(2, 'app', {kind: 'requestNotifications'}); assert.equal(prompts, 1);
  finish(); await settle(); assert.deepEqual(f.calls.map(call => call.id), [2]);
  assert.match(f.calls[0].value.value.message, /allowed/); f.bridge.close();
});

test('notification delivery neither asks permission nor posts a banner after cancellation', async () => {
  const f = fixture(); let prompts = 0, shown = 0, finishRegistration;
  class Notification {static permission = 'denied'; static requestPermission() {prompts++;} constructor() {shown++;}}
  f.platform.Notification = Notification;
  f.bridge.start(1, 'app', {kind: 'notify', title: 'Test', body: 'Fixture'}); await settle();
  assert.equal(prompts, 0); assert.equal(shown, 0); assert.match(f.calls[0].error, /not allowed/);
  Notification.permission = 'granted';
  f.platform.navigator.serviceWorker = {getRegistration: () => new Promise(resolve => {finishRegistration = resolve;})};
  f.bridge.start(2, 'app', {kind: 'notify', title: 'Test', body: 'Fixture'});
  f.bridge.cancel(2); finishRegistration({active: {}, showNotification: async () => {shown++;}}); await settle();
  assert.equal(shown, 0); assert.equal(f.calls.length, 1); f.bridge.close();
});

test('geolocation uses one validated fix and clears its watch before delivering it', async () => {
  const f = fixture(); f.bridge.start(1, 'app', {kind: 'locate'});
  const watch = f.watches.get(1); assert.equal(watch.options.timeout, 15000);
  watch.success({coords: {latitude: 51.5, longitude: -0.1}}); assert.deepEqual(f.cleared, [1]);
  await settle(); assert.deepEqual(f.calls[0], {id: 1, value: {kind: 'location', value: {latitude: 51.5, longitude: -0.1}}});
  watch.success({coords: {latitude: 0, longitude: 0}}); await settle(); assert.equal(f.calls.length, 1);
  f.bridge.start(2, 'app', {kind: 'locate'}); f.watches.get(2).success({coords: {latitude: Infinity, longitude: 0}});
  await settle(); assert.match(f.calls[1].error, /invalid location/); f.bridge.close();
});

test('cancellation, suspension and timeout stop location monitoring and discard late callbacks', async () => {
  const f = fixture(); f.bridge.start(1, 'app', {kind: 'locate'}); const old = f.watches.get(1);
  f.bridge.cancel(1); old.success({coords: {latitude: 1, longitude: 2}}); await settle(); assert.equal(f.calls.length, 0);
  f.bridge.start(2, 'app', {kind: 'locate'}); f.bridge.suspend(); assert.match(f.calls[0].error, /suspended/);
  f.bridge.start(3, 'app', {kind: 'locate'}); const timeout = [...f.timers.values()][0]; assert.equal(timeout.delay, 15000); timeout.callback();
  assert.match(f.calls[1].error, /timed out/); assert.deepEqual(f.cleared, [1, 2, 3]);
  assert.equal(f.bridge.pendingCount, 0); f.bridge.close();
});

test('platform capacity, duplicate IDs and settings are checked before side effects', async () => {
  const f = fixture();
  for (let id = 1; id <= 8; id++) f.bridge.start(id, 'app', {kind: 'locate'});
  assert.throws(() => f.bridge.start(9, 'app', {kind: 'locate'}), /Too many/);
  assert.throws(() => f.bridge.start(1, 'app', {kind: 'locate'}), /duplicate/);
  assert.equal(f.watches.size, 8); f.bridge.cancel(1);
  f.preferences.location = false; f.bridge.start(9, 'app', {kind: 'locate'}); await settle();
  assert.equal(f.watches.size, 7); assert.match(f.calls[0].error, /Turn on location/);
  f.bridge.close(); assert.equal(f.watches.size, 0); assert.equal(f.timers.size, 0);
  assert.throws(() => f.bridge.start(10, 'app', {kind: 'refresh'}), /closed/);
});

test('haptics use the bounded enum and distinguish API rejection from accepted requests', async () => {
  const f = fixture(), patterns = [];
  f.platform.navigator.vibrate = value => {patterns.push(value); return false;};
  assert.throws(() => f.bridge.start(1, 'app', {kind: 'haptic', feedback: 'unbounded'}), /Invalid haptic/);
  f.bridge.start(2, 'app', {kind: 'haptic', feedback: 'heavy'}); await settle();
  assert.deepEqual(patterns, [40]); assert.match(f.calls[0].error, /declined/);
  f.platform.navigator.vibrate = value => {patterns.push(value); return true;};
  f.bridge.start(3, 'app', {kind: 'haptic', feedback: 'selection'}); await settle();
  assert.equal(f.calls[1].value.value.message, 'Vibration requested.');
  f.bridge.suspend(); assert.deepEqual(patterns, [40, 8, 0]);
  f.preferences.vibration = false; f.bridge.start(4, 'app', {kind: 'haptic', feedback: 'light'}); await settle();
  assert.equal(patterns.length, 3); f.bridge.close();
});

test('BFCache suspends and resumes one runtime, while a permanent close removes lifecycle listeners', () => {
  const platform = new EventTarget(); platform.document = Object.assign(new EventTarget(), {hidden: false});
  const calls = [];
  const close = bindRuntimeLifecycle({hostVisibility: value => calls.push(['visible', value]), cancelInput: () => calls.push(['cancel']), closeHost: () => calls.push(['close'])}, {platform});
  platform.dispatchEvent(new Event('blur')); assert.deepEqual(calls, [['visible', true], ['cancel']]);
  const event = persisted => Object.assign(new Event('pagehide'), {persisted});
  platform.dispatchEvent(event(true)); assert.equal(calls.filter(call => call[0] === 'close').length, 0);
  platform.dispatchEvent(new Event('pageshow')); assert.deepEqual(calls.at(-1), ['visible', true]);
  platform.dispatchEvent(event(false)); assert.deepEqual(calls.at(-1), ['close']); const count = calls.length;
  platform.dispatchEvent(new Event('pageshow')); platform.document.dispatchEvent(new Event('visibilitychange')); close();
  assert.equal(calls.length, count);
});

test('URL activation accepts only the app protocol and leaves detailed route admission to Rust', () => {
  assert.equal(activationFromUrl('https://example.org/?activate=ferroui-phone%3A%2F%2Fhome'), 'ferroui-phone://home');
  assert.equal(activationFromUrl('https://example.org/#ferroui-phone://app/org.ferroui.wallet?path=cards%2F7'), 'ferroui-phone://app/org.ferroui.wallet?path=cards%2F7');
  const encodedPath = 'ferroui-phone://app/org.ferroui.wallet?path=cards%2FR%26D%2F100%25%2Fliteral%2526';
  for (const outer of [`https://example.org/#${encodedPath}`, `https://example.org/?activate=${encodeURIComponent(encodedPath)}`]) {
    const activation = activationFromUrl(outer);
    assert.equal(activation, encodedPath, 'only the outer query carrier may be decoded by JavaScript');
    assert.deepEqual([...new URL(activation).searchParams], [['path', 'cards/R&D/100%/literal%26']],
      'the inner URI retains one path parameter and decodes ampersands and percent escapes once');
  }
  assert.equal(activationFromUrl('https://example.org/#ferroui-phone://app/org.ferroui.wallet?path=100%'),
    'ferroui-phone://app/org.ferroui.wallet?path=100%', 'a literal percent in an inner URI is not a JavaScript decode error');
  assert.equal(activationFromUrl('https://example.org/#ordinary-page-anchor'), undefined);
  assert.throws(() => activationFromUrl('https://example.org/?activate=javascript%3Aalert(1)'), /activation URI/);
  assert.throws(() => activationFromUrl('https://example.org/?activate=file%3A%2F%2Fsecret'), /activation URI/);
});
