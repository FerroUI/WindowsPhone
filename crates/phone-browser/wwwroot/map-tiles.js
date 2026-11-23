// Only visible tiles enter this cache. No prefetch, region-download or arbitrary
// URL API. Normal browser HTTP caching/referrers are retained for OSM's policy.
const CACHE = 'windows-phone-viewed-map-tiles-v1';
const MAX_BYTES = 512 * 1024;
const MAX_FILES = 2048;
const MAX_CACHE_BYTES = 64 * 1024 * 1024;
const WEEK = 7 * 24 * 60 * 60 * 1000;
const EXPIRY = 'X-Phone-Map-Expires';
const SIZE = 'X-Phone-Map-Size';
const locks = new WeakMap();

export function mapTileUrl({zoom, x, y}) {
  if (!Number.isInteger(zoom) || zoom < 0 || zoom > 19 ||
      !Number.isInteger(x) || !Number.isInteger(y) ||
      x < 0 || y < 0 || x >= 2 ** zoom || y >= 2 ** zoom) {
    throw new Error('Invalid map tile coordinate.');
  }
  return `https://tile.openstreetmap.org/${zoom}/${x}/${y}.png`;
}
function validatePng(bytes) {
  const signature = [137,80,78,71,13,10,26,10];
  if (bytes.length < 33 || bytes.length > MAX_BYTES ||
      signature.some((v,i) => bytes[i] !== v) ||
      bytes[8] !== 0 || bytes[9] !== 0 || bytes[10] !== 0 || bytes[11] !== 13 ||
      String.fromCharCode(...bytes.slice(12,16)) !== 'IHDR') {
    throw new Error('The map service returned an invalid PNG tile.');
  }
  const data = new DataView(bytes.buffer,bytes.byteOffset,bytes.byteLength);
  if (data.getUint32(16) !== 256 || data.getUint32(20) !== 256) {
    throw new Error('The map service returned an invalid tile size.');
  }
}
async function read(response, pending) {
  const advertised = response.headers.get('content-length');
  if (advertised !== null && (!/^\d+$/.test(advertised) || Number(advertised) > MAX_BYTES)) {
    throw new Error('The map tile exceeds its size limit.');
  }
  if (!response.body?.getReader) throw new Error('A bounded image stream is unavailable.');
  const reader = response.body.getReader();
  const chunks = [];
  let size = 0;
  try {
    for (;;) {
      const {done,value} = await reader.read();
      if (!pending()) throw new DOMException('Map request cancelled.','AbortError');
      if (done) break;
      if (!(value instanceof Uint8Array) || value.byteLength > MAX_BYTES - size) {
        throw new Error('The map tile exceeds its size limit.');
      }
      chunks.push(value); size += value.byteLength;
    }
  } finally { try { await reader.cancel(); } catch {} }
  const bytes = new Uint8Array(size);
  let offset = 0;
  for (const chunk of chunks) { bytes.set(chunk,offset); offset += chunk.byteLength; }
  validatePng(bytes);
  return bytes;
}
function metadata(response) {
  if (!response) return null;
  const expiry = response.headers.get(EXPIRY), size = response.headers.get(SIZE);
  if (!expiry || !size || !/^\d+$/.test(expiry) || !/^\d+$/.test(size)) return null;
  const expires = Number(expiry), bytes = Number(size);
  return Number.isSafeInteger(expires) && Number.isInteger(bytes) && bytes >= 33 && bytes <= MAX_BYTES
    ? {expires,bytes} : null;
}
function expiry(headers, now) {
  const control = (headers.get('cache-control') || '').toLowerCase().split(',').map(v=>v.trim());
  if (control.includes('no-store')) return null;
  if (control.includes('no-cache')) return now;
  const maximum = control.map(v=>/^max-age="?(\d+)"?$/.exec(v)).find(Boolean);
  if (maximum) {
    const age = Number(headers.get('age')) || 0;
    return now + Math.max(0,Math.min(366 * 86400,Number(maximum[1]) - age)) * 1000;
  }
  const expires = Date.parse(headers.get('expires') || '');
  return Number.isFinite(expires) ? Math.max(0,expires) : now + WEEK;
}
async function serialize(storage, action) {
  const previous = locks.get(storage) || Promise.resolve();
  const next = previous.catch(()=>{}).then(action);
  locks.set(storage,next);
  try { return await next; }
  finally { if (locks.get(storage) === next) locks.delete(storage); }
}
async function store(storage,cache,url,bytes,headers,now,pending) {
  await serialize(storage,async()=>{
    if (!pending()) return;
    const expires = expiry(headers,now);
    if (expires === null) { await cache.delete(url); return; }
    const keys = await cache.keys();
    if (keys.length > MAX_FILES + 1) throw new Error('The viewed map cache exceeds its entry limit.');
    let count = 0, used = 0;
    for (const key of keys) {
      if (!pending()) return;
      if (key.url === url) continue;
      const item = metadata(await cache.match(key));
      if (!item || item.expires <= now) await cache.delete(key);
      else { count++; used += item.bytes; }
    }
    if (count >= MAX_FILES || used + bytes.byteLength > MAX_CACHE_BYTES) {
      throw new Error('The map cache is full of recently viewed tiles. Previously viewed maps remain available.');
    }
    const saved = new Headers();
    for (const name of ['cache-control','expires','etag','last-modified','content-type']) {
      const value = headers.get(name);
      if (value && value.length <= 1024) saved.set(name,value);
    }
    saved.set(EXPIRY,String(expires)); saved.set(SIZE,String(bytes.byteLength));
    if (pending()) await cache.put(url,new Response(bytes,{headers:saved}));
  });
}

// Dependencies are injectable for deterministic tests; the wire request cannot
// select a cache, origin, credentials, limits, or request headers.
export async function mapTile(runtime, request, options = {}) {
  const id = request.request_id;
  if (!Number.isInteger(id) || id < 1 || id > 0xffffffff) return false;
  const pending = () => runtime.isHttpTaskPending(id);
  const controller = new AbortController();
  const fetcher = options.fetcher || globalThis.fetch;
  const storage = options.storage === undefined ? globalThis.caches : options.storage;
  const clock = options.now || Date.now;
  let timeout, cancellation;
  try {
    if (!pending()) return false;
    const url = mapTileUrl(request);
    timeout = setTimeout(()=>controller.abort(new Error('The map request timed out.')),15000);
    cancellation = setInterval(()=>{if (!pending()) controller.abort();},100);
    // CacheStorage can be unavailable in private or restricted browser sessions.
    // The HTTP cache still follows the service's cache headers in that case.
    let cache;
    if (storage) { try { cache = await storage.open(CACHE); } catch {} }
    if (!pending()) return false;
    if (cache) {
      const saved = await cache.match(url);
      const meta = metadata(saved);
      if (meta && meta.expires > clock()) {
        try {
          const bytes = await read(saved,pending);
          if (bytes.byteLength !== meta.bytes) throw new Error('Cached map tile length mismatch.');
          return pending() ? runtime.completeHttpTask(id,bytes) : false;
        } catch (error) {
          if (!pending()) return false;
          await cache.delete(url);
        }
      }
    }
    const response = await fetcher(url,{
      method:'GET',mode:'cors',credentials:'omit',redirect:'error',cache:'default',
      referrerPolicy:'strict-origin-when-cross-origin',signal:controller.signal,
      headers:{Accept:'image/png'},
    });
    if (!pending()) return false;
    if (!response.ok) throw new Error(`Map service returned HTTP ${response.status}.`);
    const bytes = await read(response,pending);
    if (cache) await store(storage,cache,url,bytes,response.headers,clock(),pending);
    return pending() ? runtime.completeHttpTask(id,bytes) : false;
  } catch (error) {
    if (!pending()) return false;
    const message = error instanceof TypeError
      ? 'The browser could not load this map tile. Check your connection and cross-origin access.'
      : String(error?.message || error);
    return runtime.failHttpTask(id,message.slice(0,1024));
  } finally {
    clearTimeout(timeout); clearInterval(cancellation); controller.abort();
  }
}
