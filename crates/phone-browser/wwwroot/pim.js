import {chooseFiles} from './file-picker.js';

const MAX_BYTES = 2 * 1024 * 1024;

async function importFile(runtime, kind, report) {
  const contacts = kind === 'contacts';
  const files = await chooseFiles(contacts ? '.vcf,text/vcard' : '.ics,text/calendar', false);
  if (!files.length) return;
  const file = files[0];
  if (!file.size || file.size > MAX_BYTES) throw new Error('Choose a nonempty file no larger than 2 MB.');
  const bytes = new Uint8Array(await file.arrayBuffer());
  const count = contacts ? runtime.importVcards(bytes) : runtime.importIcalendar(bytes);
  report(`Imported ${count} ${contacts ? (count === 1 ? 'contact' : 'contacts') : (count === 1 ? 'appointment' : 'appointments')}.`);
}

export const importContacts = (runtime, report) => importFile(runtime, 'contacts', report);
export const importCalendar = (runtime, report) => importFile(runtime, 'calendar', report);

function prepare(runtime, ids, contacts) {
  if (!Array.isArray(ids) || !ids.length || ids.length > 512 || ids.some(id => !Number.isSafeInteger(id) || id < 1)) {
    throw new Error('Choose between 1 and 512 saved items.');
  }
  const encoded = JSON.stringify(ids);
  const data = JSON.parse(contacts ? runtime.exportVcards(encoded) : runtime.exportIcalendar(encoded));
  if (!Array.isArray(data.bytes) || data.bytes.length > MAX_BYTES || !data.bytes.length) throw new Error('The exported file is empty or too large.');
  return new File([Uint8Array.from(data.bytes)], data.name, {type: data.mime});
}

function save(file) {
  const url = URL.createObjectURL(file);
  const link = document.createElement('a');
  link.href = url; link.download = file.name;
  document.body.append(link); link.click(); link.remove();
  setTimeout(() => URL.revokeObjectURL(url), 30_000);
}

function share(file, platform) {
  const files = [file];
  if (!platform?.share || !platform.canShare?.({files})) {
    throw new Error('This browser cannot share contact or calendar files. Use save a copy instead.');
  }
  // Invoke the OS share sheet before any asynchronous boundary loses the native
  // input activation. Cancellation never falls back to copying personal data.
  return platform.share({files}).then(() => true).catch(error => { if (error.name !== 'AbortError') throw error; return false; });
}

export const exportContacts = (runtime, ids) => save(prepare(runtime, ids, true));
export const exportCalendar = (runtime, ids) => save(prepare(runtime, ids, false));
export const shareContacts = (runtime, ids, platform = navigator) => share(prepare(runtime, ids, true), platform);
export const shareCalendar = (runtime, ids, platform = navigator) => share(prepare(runtime, ids, false), platform);
