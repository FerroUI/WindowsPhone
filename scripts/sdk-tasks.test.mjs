import assert from 'node:assert/strict';
import test from 'node:test';
import {chooseFile} from '../crates/phone-browser/wwwroot/sdk-tasks.js';

const request = {request_id: 7, extensions: ['.txt'], max_bytes: 1024};
function runtime() {
  const calls = [];
  let pending = true;
  return {calls, abandon: () => pending = false,
    isFileTaskPending: id => pending && id === 7,
    completeFileTask: (...args) => {calls.push(['success', ...args]); pending = false; return true;},
    cancelFileTask: id => {calls.push(['cancel', id]); pending = false; return true;},
    failFileTask: (id, error) => {calls.push(['error', id, error]); pending = false; return true;},
  };
}
const bytes = text => new TextEncoder().encode(text);
const file = (name = 'note.txt', text = 'hello') => ({name, type: 'text/plain', size: bytes(text).length,
  arrayBuffer: async () => bytes(text).buffer});

test('SDK chooser preserves synchronous activation and actual typed file bytes', async () => {
  const host = runtime(); let invoked = false;
  const task = chooseFile(host, request, (accept, multiple) => {
    invoked = true; assert.equal(accept, '.txt'); assert.equal(multiple, false);
    return Promise.resolve([file('note.TXT', 'Żółć')]);
  });
  assert(invoked, 'picker runs before the caller yields to its executor');
  assert.equal(await task, true);
  assert.deepEqual(host.calls, [['success', 7, 'note.TXT', 'text/plain', bytes('Żółć')]]);
});

test('SDK chooser cancellation is a typed empty result, without reading a file', async () => {
  for (const picker of [() => Promise.resolve([]), () => Promise.reject(Object.assign(Error('cancel'), {name: 'AbortError'}))]) {
    const host = runtime(); await chooseFile(host, request, picker);
    assert.deepEqual(host.calls, [['cancel', 7]]);
  }
});

test('SDK chooser rejects oversized, renamed and failed reads before success', async () => {
  for (const selected of [
    {...file(), size: 1025, arrayBuffer: () => {throw Error('must not read');}},
    file('note.exe'), {...file(), name: '../note.txt'},
    {...file(), arrayBuffer: async () => {throw Error('disk read failed');}},
    {...file(), arrayBuffer: async () => new ArrayBuffer(6)},
  ]) {
    const host = runtime(); await chooseFile(host, request, async () => [selected]);
    assert.equal(host.calls.length, 1); assert.equal(host.calls[0][0], 'error');
    assert(!host.calls[0][2].includes('must not read'), 'size admission runs before file reads');
  }
});

test('SDK chooser does not deliver or read a selection after the app abandons it', async () => {
  const host = runtime(); let resolve;
  const task = chooseFile(host, request, () => new Promise(done => {resolve = done;}));
  host.abandon();
  resolve([{...file(), arrayBuffer: () => {throw Error('obsolete file must not be read');}}]);
  assert.equal(await task, false); assert.deepEqual(host.calls, []);

  const reading = runtime(); let finish;
  const second = chooseFile(reading, request, async () => [{...file(), arrayBuffer: () => new Promise(done => {finish = done;})}]);
  await Promise.resolve();
  reading.abandon(); finish(bytes('hello').buffer);
  assert.equal(await second, false); assert.deepEqual(reading.calls, []);
});

test('SDK malformed options never invoke a picker', async () => {
  for (const options of [{...request, max_bytes: 0}, {...request, extensions: ['text/plain']}, {...request, extensions: Array(17).fill('.txt')}]) {
    const host = runtime(); await chooseFile(host, options, () => {throw Error('must not pick');});
    assert.equal(host.calls[0][0], 'error'); assert(!host.calls[0][2].includes('must not pick'));
  }
});
