// Shared wire format with the native host. Asset keys identify immutable blobs,
// never paths. A restore gives every supplied blob a new key before committing.
const MiB = 1024 * 1024;
const keyPattern = /^media:[A-Za-z0-9][A-Za-z0-9_-]{0,127}$/;
const alphabet = 'ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/';
const utf8 = new TextEncoder();

export function mediaSources(phone) {
  return new Set(mediaReferences(phone).keys());
}

function mediaReferences(phone) {
  const references = new Map();
  const add = (source, kind) => {
    if (typeof source !== 'string' || !source.startsWith('media:')) return;
    if (!keyPattern.test(source)) throw new Error('Invalid media reference in phone data.');
    if (references.has(source) && references.get(source) !== kind) throw new Error('A media identifier has conflicting photo and playback references.');
    references.set(source, kind);
  };
  for (const photo of phone.photos || []) add(photo.source, 'image');
  for (const track of phone.tracks || []) add(track.source, track.media_kind === 'video' ? 'video' : 'audio');
  add(phone.settings?.wallpaper, 'image');
  add(phone.settings?.lock_wallpaper, 'image');
  if (references.size > 10000) throw new Error('The backup contains too many media references.');
  return references;
}

function base64Length(data, limit) {
  if (!data.length || data.length % 4 || data.length > Math.ceil(limit / 3) * 4) throw new Error('Invalid or oversized media data.');
  const padding = data.endsWith('==') ? 2 : data.endsWith('=') ? 1 : 0;
  const end = data.length - padding;
  // A repeating-group regexp over a 128 MiB file can exhaust V8's regexp
  // stack. Fixed-size chunks keep validation linear and bounded instead.
  for (let start = 0; start < end; start += 32768) {
    if (/[^A-Za-z0-9+/]/.test(data.slice(start, Math.min(end, start + 32768)))) throw new Error('Invalid media base64 characters.');
  }
  const final = alphabet.indexOf(data[end - 1]);
  if ((padding === 2 && (final & 15)) || (padding === 1 && (final & 3))) throw new Error('Invalid media base64 padding.');
  const length = data.length / 4 * 3 - padding;
  if (!length || length > limit) throw new Error('Invalid or oversized media data.');
  return length;
}

export function prepareBackup(payload, validate, newKey = () => `media:${crypto.randomUUID()}`) {
  if (!payload || typeof payload !== 'object' || Array.isArray(payload)) throw new Error('Invalid phone backup.');
  const envelope = payload.format === 'ferroui-windows-phone';
  if (payload.format !== undefined && !envelope) throw new Error('This backup format is not supported.');
  if (envelope && payload.version !== 1) throw new Error('This backup format is not supported.');
  const document = envelope ? payload.phone : payload;
  if (document?.version !== 1) throw new Error('Select a Windows Phone backup with a supported document version.');
  const json = JSON.stringify(document);
  if (typeof json !== 'string' || utf8.encode(json).length > 16 * MiB) throw new Error('Phone data exceeds the 16 MB import limit.');
  validate(json);
  const phone = JSON.parse(json);
  const references = mediaReferences(phone);
  const sources = new Set(references.keys());
  const manifest = envelope ? payload.assets ?? [] : [];
  if (!Array.isArray(manifest) || manifest.length > 10000) throw new Error('Invalid media manifest.');
  const seen = new Set(), allocated = new Set(), remapped = new Map(), assets = [];
  let total = 0;
  for (const item of manifest) {
    if (!item || !keyPattern.test(item.key) || seen.has(item.key)
      || !['image', 'audio', 'video'].includes(item.kind)
      || typeof item.name !== 'string' || !item.name.length || utf8.encode(item.name).length > 1024 || /[\u0000-\u001f\u007f-\u009f]/.test(item.name)
      || typeof item.mime !== 'string' || utf8.encode(item.mime).length > 128 || /[\u0000-\u001f\u007f-\u009f]/.test(item.mime)
      || typeof item.data !== 'string') throw new Error('Invalid or duplicate media entry.');
    seen.add(item.key);
    const limit = (item.kind === 'image' ? 16 : 128) * MiB;
    if ((total += base64Length(item.data, limit)) > 192 * MiB) throw new Error('Backup media exceeds the import limits.');
    if (!sources.has(item.key)) throw new Error('The backup contains an unreferenced media asset.');
    if (item.kind !== references.get(item.key)) throw new Error('The media manifest kind does not match its phone reference.');
    const key = newKey();
    if (!keyPattern.test(key) || sources.has(key) || allocated.has(key)) throw new Error('Could not allocate a fresh media identifier.');
    allocated.add(key);
    remapped.set(item.key, key);
    const raw = atob(item.data), bytes = new Uint8Array(raw.length);
    for (let i = 0; i < raw.length; i++) bytes[i] = raw.charCodeAt(i);
    assets.push({key, name: item.name, mime: item.mime, kind: item.kind, bytes: bytes.buffer});
  }
  const map = key => remapped.get(key) || key;
  for (const photo of phone.photos || []) photo.source = map(photo.source);
  for (const track of phone.tracks || []) track.source = map(track.source);
  phone.settings ||= {};
  phone.settings.wallpaper = map(phone.settings.wallpaper);
  phone.settings.lock_wallpaper = map(phone.settings.lock_wallpaper);
  const restored = JSON.stringify(phone);
  validate(restored);
  return {json: restored, assets, missing: [...sources].filter(key => !remapped.has(key)).length};
}
