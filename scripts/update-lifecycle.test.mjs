import assert from 'node:assert/strict';
import test from 'node:test';
import {readFile} from 'node:fs/promises';
import vm from 'node:vm';

const scope = 'https://example.test/WindowsPhone/';
const files = ['index.html', 'main.js', 'phone.wasm'];
const workerSource = await readFile(new URL('../crates/phone-browser/wwwroot/app-sw.js', import.meta.url), 'utf8');
const updatesSource = await readFile(new URL('../crates/phone-browser/wwwroot/updates.js', import.meta.url), 'utf8');

function cacheStorage() {
  const stores = new Map();
  const deleted = [];
  let fetchAsset = async request => new Response(`asset:${request.url}`);
  const key = value => typeof value === 'string' ? value : value.url;
  const storage = {
    stores, deleted,
    setFetchAsset(callback) { fetchAsset = callback; },
    async keys() { return [...stores.keys()]; },
    async delete(name) { deleted.push(name); return stores.delete(name); },
    async open(name) {
      if (!stores.has(name)) stores.set(name, new Map());
      const entries = stores.get(name);
      return {
        async addAll(requests) {
          // Cache.addAll commits a batch only after every request succeeds. The
          // worker must preserve the existing active cache if that batch fails.
          const fetched = await Promise.all(requests.map(async request => {
            assert.equal(request.cache, 'reload');
            const response = await fetchAsset(request);
            if (!response.ok) throw new TypeError('Precache request failed');
            return [request.url, response];
          }));
          for (const [url, response] of fetched) entries.set(url, response);
        },
        async put(request, response) { entries.set(key(request), response); },
        async match(request, options = {}) {
          const requested = new URL(key(request));
          const match = [...entries].find(([url]) => {
            const cached = new URL(url);
            return options.ignoreSearch
              ? cached.origin === requested.origin && cached.pathname === requested.pathname
              : cached.href === requested.href;
          });
          return match?.[1].clone();
        },
      };
    },
  };
  return storage;
}

function worker({version, caches, appScope = scope, fetch = async () => { throw Error('offline'); }}) {
  const listeners = new Map();
  const calls = {claim: 0, skipWaiting: 0};
  const self = {
    registration: {scope: appScope},
    location: {origin: new URL(appScope).origin},
    addEventListener(type, callback) { listeners.set(type, callback); },
    skipWaiting() { calls.skipWaiting++; return Promise.resolve(); },
    clients: {async claim() { calls.claim++; }},
  };
  vm.runInNewContext(workerSource
    .replace('__BUILD_VERSION__', version)
    .replace('__PRECACHE__', JSON.stringify(files)), {self, caches, URL, Request, fetch});
  return {
    calls,
    async lifecycle(type) {
      const pending = [];
      listeners.get(type)({waitUntil(promise) { pending.push(promise); }});
      assert.equal(pending.length, 1, `${type} must keep its asynchronous work alive`);
      await Promise.all(pending);
    },
    async fetch(path, mode = 'same-origin', method = 'GET') {
      let response;
      listeners.get('fetch')({
        request: {url: new URL(path, appScope).href, method, mode},
        respondWith(value) { response = value; },
      });
      return await response;
    },
  };
}

function serveVersion(caches, version) {
  caches.setFetchAsset(async request => new Response(`${version}:${new URL(request.url).pathname}`));
}

test('a failed update install keeps the active app usable offline and never takes control', async () => {
  const caches = cacheStorage();
  serveVersion(caches, 'old');
  const active = worker({version: 'old', caches});
  await active.lifecycle('install');
  await active.lifecycle('activate');
  const originalKeys = await caches.keys();
  const candidate = worker({version: 'broken', caches});
  caches.setFetchAsset(async request => {
    if (request.url.endsWith('phone.wasm')) throw new TypeError('Network disconnected');
    return new Response('new');
  });
  await assert.rejects(candidate.lifecycle('install'), /Network disconnected/);
  for (const key of originalKeys) assert.ok(caches.stores.has(key), 'the active build cache survives');
  assert.equal(await (await active.fetch('./?native=1', 'navigate')).text(), 'old:/WindowsPhone/index.html');
  assert.equal(await (await active.fetch('main.js')).text(), 'old:/WindowsPhone/main.js');
  assert.equal(await (await active.fetch('phone.wasm')).text(), 'old:/WindowsPhone/phone.wasm');
  assert.deepEqual(candidate.calls, {claim: 0, skipWaiting: 0});
  assert.deepEqual(caches.deleted, []);
});

test('a completely downloaded worker waits while old tabs keep a consistent JS and WASM build', async () => {
  const caches = cacheStorage();
  serveVersion(caches, 'old');
  const active = worker({version: 'old', caches});
  await active.lifecycle('install');
  await active.lifecycle('activate');
  const oldCaches = await caches.keys();
  serveVersion(caches, 'new');
  const candidate = worker({version: 'new', caches});
  await candidate.lifecycle('install');
  assert.deepEqual(candidate.calls, {claim: 0, skipWaiting: 0});
  assert.deepEqual(caches.deleted, []);
  for (const file of files) {
    assert.equal(await (await active.fetch(file)).text(), `old:/WindowsPhone/${file}`);
    assert.equal(await (await candidate.fetch(file)).text(), `new:/WindowsPhone/${file}`);
  }
  for (const name of oldCaches) assert.ok(caches.stores.has(name));
  await caches.open('unrelated-application');
  await candidate.lifecycle('activate');
  assert.equal(candidate.calls.claim, 1);
  assert.equal(candidate.calls.skipWaiting, 0);
  assert.ok(caches.stores.has('unrelated-application'));
  for (const name of oldCaches) assert.ok(!caches.stores.has(name), 'retire old assets only at activation');
  assert.equal(await (await candidate.fetch('./?native=1', 'navigate')).text(), 'new:/WindowsPhone/index.html');
});

test('worker fetch handling leaves writes and cross-origin requests to the browser', async () => {
  const caches = cacheStorage();
  let calls = 0;
  const app = worker({version: 'test', caches, fetch: async () => { calls++; return new Response('network'); }});
  await app.lifecycle('install');
  assert.equal(await app.fetch('https://other.test/api'), undefined);
  assert.equal(await app.fetch('api', 'same-origin', 'POST'), undefined);
  assert.equal(calls, 0);
  assert.equal(await (await app.fetch('uncached.json')).text(), 'network');
  assert.equal(calls, 1);
});

test('activation retires only this installation scope and preserves other phone installations', async () => {
  const caches = cacheStorage();
  serveVersion(caches, 'old');
  const original = worker({version: 'old', caches});
  await original.lifecycle('install');
  await original.lifecycle('activate');
  const originalNames = await caches.keys();
  const previews = ['/Preview/', '/WindowsPhone/:preview/'].map(path => ({
    path, app: worker({version: 'old', caches, appScope: `https://example.test${path}`}),
  }));
  for (const {app} of previews) {
    await app.lifecycle('install');
    await app.lifecycle('activate');
  }
  for (const name of originalNames) assert.ok(caches.stores.has(name));
  await caches.open('windows-phone-legacy-build');
  serveVersion(caches, 'new');
  const replacement = worker({version: 'new', caches});
  await replacement.lifecycle('install');
  await replacement.lifecycle('activate');
  for (const {path, app} of previews) {
    assert.equal(await (await app.fetch('main.js')).text(), `old:${path}main.js`);
    assert.equal(await (await app.fetch('phone.wasm')).text(), `old:${path}phone.wasm`);
  }
  assert.equal(await (await replacement.fetch('main.js')).text(), 'new:/WindowsPhone/main.js');
  assert.ok(caches.stores.has('windows-phone-legacy-build'), 'ambiguous legacy caches remain untouched');
  for (const name of originalNames) assert.ok(!caches.stores.has(name));
});

class FakeWorker extends EventTarget {
  state = 'installing';
  messages = [];
  postMessage(message) { this.messages.push(message); }
  transition(state) { this.state = state; this.dispatchEvent(new Event('statechange')); }
}

class FakeRegistration extends EventTarget {
  installing = null;
  waiting = null;
  updates = 0;
  onUpdate = () => {};
  async update() { this.updates++; await this.onUpdate(); }
  found(worker) { this.installing = worker; this.dispatchEvent(new Event('updatefound')); }
}

function updates({registration = new FakeRegistration(), register, controller = {}, online = true, supported = true} = {}) {
  const calls = {register: [], getRegistration: [], reload: 0};
  const navigator = {onLine: online};
  if (supported) navigator.serviceWorker = {
    controller,
    register(url, options) {
      calls.register.push({url: url.href, ...options});
      return register ? register() : Promise.resolve(registration);
    },
    async getRegistration(url) { calls.getRegistration.push(url); return registration; },
  };
  const context = {
    navigator, URL,
    location: {reload() { calls.reload++; }},
  };
  // Execute the real module in an isolated realm. Only ES module syntax is
  // adapted; its lifecycle and registration logic run unchanged.
  vm.runInNewContext(updatesSource
    .replace('import.meta.url', JSON.stringify(`${scope}updates.js`))
    .replaceAll('export ', '')
    + '\nglobalThis.api = {installUpdates, checkForUpdates};', context);
  return {...context.api, navigator, registration, calls};
}

test('registration is coalesced and a downloaded update never forces an existing window to reload', async () => {
  const registration = new FakeRegistration();
  registration.waiting = new FakeWorker();
  registration.waiting.state = 'installed';
  const adapter = updates({registration});
  const messages = [];
  await Promise.all([
    adapter.installUpdates(message => messages.push(message)),
    adapter.installUpdates(message => messages.push(message)),
  ]);
  assert.deepEqual(adapter.calls.register, [{url: `${scope}app-sw.js`, scope, updateViaCache: 'none'}]);
  assert.equal(messages.length, 1);
  assert.match(messages[0], /Close all Windows Phone windows/);
  assert.equal(adapter.calls.reload, 0);
  assert.deepEqual(registration.waiting.messages, []);
});

test('explicit update checks distinguish downloading, downloaded, and unchanged builds', async () => {
  const adapter = updates();
  const messages = [];
  const report = message => messages.push(message);
  await adapter.installUpdates(report);
  await adapter.checkForUpdates(report);
  assert.match(messages.pop(), /No newer offline app update/);
  const candidate = new FakeWorker();
  adapter.registration.found(candidate);
  await adapter.checkForUpdates(report);
  assert.match(messages.pop(), /Downloading an update/);
  adapter.registration.installing = null;
  adapter.registration.waiting = candidate;
  candidate.transition('installed');
  assert.match(messages.pop(), /Close all Windows Phone windows/);
  await adapter.checkForUpdates(report);
  assert.match(messages.pop(), /Close all Windows Phone windows/);
  assert.equal(adapter.registration.updates, 3);
  assert.deepEqual(candidate.messages, []);
  assert.equal(adapter.calls.reload, 0);
});

test('first installation does not ask the user to close a page with no previous controller', async () => {
  const registration = new FakeRegistration();
  const candidate = new FakeWorker();
  registration.installing = candidate;
  const adapter = updates({registration, controller: null});
  const messages = [];
  await adapter.installUpdates(message => messages.push(message));
  candidate.transition('installed');
  assert.deepEqual(messages, []);
  assert.equal(adapter.calls.reload, 0);
});

test('an installation failure reports that the current app remains available', async () => {
  const adapter = updates();
  const messages = [];
  await adapter.installUpdates(message => messages.push(message));
  const candidate = new FakeWorker();
  adapter.registration.found(candidate);
  candidate.transition('redundant');
  assert.equal(messages.length, 1);
  assert.match(messages[0], /did not finish installing.*current app is still available/);
  assert.equal(adapter.calls.reload, 0);
});

test('unsupported and offline checks do not attempt a network update', async () => {
  const absent = updates({supported: false});
  assert.equal(await absent.installUpdates(() => {}), undefined);
  await assert.rejects(absent.checkForUpdates(() => {}), /unavailable in this browser/);
  const offline = updates({online: false});
  await assert.rejects(offline.checkForUpdates(() => {}), /Connect to the internet/);
  assert.equal(offline.registration.updates, 0);
  assert.deepEqual(offline.calls.getRegistration, []);
});

test('an explicit check registers the offline app when no previous registration is available', async () => {
  const adapter = updates();
  adapter.navigator.serviceWorker.getRegistration = async () => undefined;
  const messages = [];
  await adapter.checkForUpdates(message => messages.push(message));
  assert.equal(adapter.calls.register.length, 1);
  assert.equal(adapter.registration.updates, 1);
  assert.match(messages.at(-1), /No newer offline app update/);
});

test('a failed initial registration can be retried after connectivity recovers', async () => {
  const registration = new FakeRegistration();
  let attempts = 0;
  const adapter = updates({registration, register: async () => {
    if (++attempts === 1) throw new TypeError('Temporary registration failure');
    return registration;
  }});
  await assert.rejects(adapter.installUpdates(() => {}), /Temporary registration failure/);
  assert.equal(await adapter.installUpdates(() => {}), registration);
  assert.equal(attempts, 2);
  await adapter.checkForUpdates(() => {});
  assert.equal(registration.updates, 1);
});

test('superseding a downloaded worker does not report a failed installation', async () => {
  const registration = new FakeRegistration();
  const candidate = new FakeWorker();
  registration.installing = candidate;
  const adapter = updates({registration});
  const messages = [];
  await adapter.installUpdates(message => messages.push(message));
  candidate.transition('installed');
  assert.match(messages.pop(), /Close all Windows Phone windows/);
  candidate.transition('redundant');
  assert.deepEqual(messages, []);
});
