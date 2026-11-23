// Explicit SDK reads carry no credentials and cannot follow an unreviewed redirect.
const MAX_BYTES = 2 * 1024 * 1024;
const validId = id => Number.isInteger(id) && id > 0 && id <= 0xffffffff;
export function publicHttpsUrl(value) {
  if (typeof value !== 'string' || value.length > 4096 || /\p{Cc}/u.test(value)) throw new Error('Invalid HTTPS address.');
  const url = new URL(value.trim());
  const host = url.hostname.toLowerCase().replace(/\.$/, '');
  if (url.protocol !== 'https:' || url.username || url.password || (url.port && url.port !== '443') ||
      !host.includes('.') || host.startsWith('[') || /^[\d.]+$/.test(host) ||
      /(^|\.)(localhost|local|internal|lan|home|test|invalid)$/.test(host)) {
    throw new Error('Use a public HTTPS address without a username, password or custom port.');
  }
  url.hash = '';
  return url.href;
}

export async function httpGet(runtime, request, fetcher = globalThis.fetch) {
  const id = request.request_id;
  if (!validId(id)) return false;
  const controller = new AbortController();
  let timeout = 0, cancellation = 0, reader;
  try {
    if (!runtime.isHttpTaskPending(id)) return false;
    const limit = request.max_bytes;
    if (!Number.isInteger(limit) || limit < 1 || limit > MAX_BYTES) throw new Error('Invalid HTTPS response limit.');
    const url = publicHttpsUrl(request.url);
    timeout = setTimeout(() => controller.abort(new Error('The HTTPS request timed out.')), 15000);
    cancellation = setInterval(() => { if (!runtime.isHttpTaskPending(id)) controller.abort(); }, 250);
    const response = await fetcher(url, {
      method: 'GET', mode: 'cors', credentials: 'omit', redirect: 'error',
      cache: 'no-store', referrerPolicy: 'no-referrer', signal: controller.signal,
      headers: {Accept: 'application/json, application/rss+xml, application/atom+xml, application/xml, text/xml, text/plain;q=0.8'},
    });
    if (!runtime.isHttpTaskPending(id)) return false;
    if (!response.ok) throw new Error(`The server returned HTTP ${response.status}.`);
    const length = response.headers?.get('content-length');
    if (length !== null && length !== undefined && (!/^\d+$/.test(length) || Number(length) > limit)) {
      throw new Error('The response exceeds the requested size limit.');
    }
    if (!response.body?.getReader) throw new Error('This browser cannot read a bounded response stream.');
    reader = response.body.getReader();
    const chunks = [];
    let size = 0;
    for (;;) {
      const {done, value} = await reader.read();
      if (!runtime.isHttpTaskPending(id)) return false;
      if (done) break;
      if (!(value instanceof Uint8Array) || value.byteLength > limit - size) throw new Error('The response exceeds the requested size limit.');
      size += value.byteLength;
      chunks.push(value);
    }
    const bytes = new Uint8Array(size);
    let offset = 0;
    for (const chunk of chunks) { bytes.set(chunk, offset); offset += chunk.byteLength; }
    return runtime.completeHttpTask(id, bytes);
  } catch (error) {
    if (!runtime.isHttpTaskPending(id)) return false;
    const message = error instanceof TypeError
      ? 'The browser could not read this address. The server must allow cross-origin access and use the final HTTPS URL. You can import a saved feed file instead.'
      : String(error?.message || error);
    return runtime.failHttpTask(id, message.slice(0, 1024));
  } finally {
    clearTimeout(timeout); clearInterval(cancellation);
    controller.abort();
    if (reader) { try { await reader.cancel(); } catch {} }
  }
}
