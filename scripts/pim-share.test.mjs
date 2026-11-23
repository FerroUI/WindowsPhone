import assert from 'node:assert/strict';
import test from 'node:test';
import {shareContacts, shareCalendar} from '../crates/phone-browser/wwwroot/pim.js';

const card = 'BEGIN:VCARD\r\nVERSION:4.0\r\nFN:Alex\r\nEND:VCARD\r\n';
const appointment = 'BEGIN:VCALENDAR\r\nVERSION:2.0\r\nEND:VCALENDAR\r\n';
const runtime = {
  exportVcards: ids => { assert.deepEqual(JSON.parse(ids), [7]); return JSON.stringify({name: 'contact.vcf', mime: 'text/vcard', bytes: [...new TextEncoder().encode(card)]}); },
  exportIcalendar: ids => { assert.deepEqual(JSON.parse(ids), [8]); return JSON.stringify({name: 'appointment.ics', mime: 'text/calendar', bytes: [...new TextEncoder().encode(appointment)]}); },
};

test('PIM sharing preserves actual file payloads and synchronous user activation', async () => {
  let selected;
  const host = {canShare: () => true, share: ({files}) => { selected = files[0]; return Promise.resolve(); }};
  const result = shareContacts(runtime, [7], host);
  assert.equal(selected.name, 'contact.vcf'); assert.equal(await result, true);
  assert.equal(await selected.text(), card);
  assert.equal(await shareCalendar(runtime, [8], host), true);
  assert.equal(selected.type, 'text/calendar'); assert.equal(await selected.text(), appointment);
});

test('PIM cancellation and failed validation never fall back to other channels', async () => {
  let calls = 0;
  const host = {canShare: () => true, share: () => { calls++; return Promise.reject(Object.assign(Error('Cancelled'), {name: 'AbortError'})); }};
  assert.equal(await shareContacts(runtime, [7], host), false); assert.equal(calls, 1);
  assert.throws(() => shareContacts(runtime, [0], host), /saved items/);
  assert.throws(() => shareContacts(runtime, [7], {...host, canShare: () => false}), /cannot share/);
  assert.throws(() => shareContacts({exportVcards: () => { throw Error('Contact no longer exists'); }}, [7], host), /no longer exists/);
  assert.equal(calls, 1);
});
