import assert from 'node:assert/strict';

// Real Rust byte callbacks plus native People/Calendar menus. These checks do
// not open a system picker/share sheet or transmit personal information.
export async function verifyPimRefinements(ctx) {
  const {page, check, state, capture, control, button, clickLabel, clickName, isRoute} = ctx;
  const original = await state();
  const fixture = structuredClone(original);
  Object.assign(fixture.settings, {lock_timeout: 0, reduced_motion: true});
  fixture.alarms = []; fixture.events = []; fixture.contacts = [];
  fixture.conversations = [];
  const load = async data => {
    await page.evaluate(`phoneApp.importData(${JSON.stringify(JSON.stringify(data))})`);
    await isRoute('Start');
  };
  const app = async name => { await page.evaluate(`phoneApp.openApp(${JSON.stringify(name)})`); await isRoute({App: name}); };
  let contactId, eventId, day;
  await load(fixture);
  try {
    await check('vCard host bytes import atomically and the native People card exposes file sharing', async () => {
      const source = 'BEGIN:VCARD\r\nVERSION:4.0\r\nFN:PIM contact Żółć\r\nTEL;VALUE=text:+12025550101\r\nEMAIL:pim@example.test\r\nORG:Interop lab\r\nEND:VCARD\r\n';
      const before = await state();
      const rejected = await page.evaluate(`(()=>{try{phoneApp.importVcards(new TextEncoder().encode(${JSON.stringify(source.replace('END:VCARD', 'TEL:555\r\nEND:VCARD'))}));return false;}catch{return true;}})()`);
      assert(rejected); assert.deepEqual(await state(), before);
      const count = await page.evaluate(`phoneApp.importVcards(new TextEncoder().encode(${JSON.stringify(source)}))`);
      assert.equal(count, 1);
      const contact = (await state()).contacts.find(value => value.name === 'PIM contact Żółć');
      assert(contact); contactId = contact.id; assert.equal(contact.email, 'pim@example.test');
      const exported = await page.evaluate(`JSON.parse(phoneApp.exportVcards(${JSON.stringify(JSON.stringify([contactId]))}))`);
      const text = new TextDecoder().decode(Uint8Array.from(exported.bytes));
      assert(text.includes('BEGIN:VCARD\r\nVERSION:4.0')); assert(text.includes('FN:PIM contact Żółć'));
      await app('People'); await clickLabel('PIM contact Żółć'); await isRoute({Contact: contactId});
      await clickLabel('more');
      await control(element => button(element) && element.automationName === 'share contact', 'native contact share command');
      await control(element => button(element) && element.automationName === 'save contact copy', 'native contact copy command');
      await capture('people-native-vcard-contact');
    });

    await check('iCalendar byte callbacks preserve local recurrence and native series sharing', async () => {
      day = await page.evaluate(`(()=>{const now=new Date();return [now.getFullYear(),String(now.getMonth()+1).padStart(2,'0'),String(now.getDate()).padStart(2,'0')].join('-');})()`);
      const stamp = day.replaceAll('-', '');
      const source = `BEGIN:VCALENDAR\r\nVERSION:2.0\r\nPRODID:Windows Phone test\r\nBEGIN:VEVENT\r\nUID:pim-native-series\r\nSUMMARY:PIM local series\r\nDTSTART:${stamp}T090000\r\nDURATION:PT45M\r\nRRULE:FREQ=DAILY\r\nLOCATION:Local calendar\r\nTRANSP:OPAQUE\r\nEND:VEVENT\r\nEND:VCALENDAR\r\n`;
      const count = await page.evaluate(`phoneApp.importIcalendar(new TextEncoder().encode(${JSON.stringify(source)}))`);
      assert.equal(count, 1);
      const event = (await state()).events.find(value => value.title === 'PIM local series');
      assert(event); eventId = event.id; assert.equal(event.repeat, 'daily'); assert.equal(event.duration_minutes, 45);
      const exported = await page.evaluate(`JSON.parse(phoneApp.exportIcalendar(${JSON.stringify(JSON.stringify([eventId]))}))`);
      const text = new TextDecoder().decode(Uint8Array.from(exported.bytes));
      assert(text.includes('RRULE:FREQ=DAILY')); assert(text.includes(`DTSTART:${stamp}T090000`));
      const before = await state();
      const rejected = await page.evaluate(`(()=>{try{phoneApp.importIcalendar(new TextEncoder().encode(${JSON.stringify(source.replace(`DTSTART:${stamp}`, `DTSTART;TZID=Europe/Warsaw:${stamp}`))}));return false;}catch{return true;}})()`);
      assert(rejected); assert.deepEqual(await state(), before);
      await app('Calendar'); await clickName(`CalendarDay${day}`); await clickName(`CalendarEvent${eventId}-${day}`);
      await clickLabel('more');
      await control(element => button(element) && element.automationName === 'share series', 'native series share command');
      await control(element => button(element) && element.automationName === 'save series copy', 'native series copy command');
      await capture('calendar-native-icalendar-series');
    });
  } catch (error) {
    await capture('pim-failure').catch(() => {});
    throw error;
  } finally { await load(original); }
}
