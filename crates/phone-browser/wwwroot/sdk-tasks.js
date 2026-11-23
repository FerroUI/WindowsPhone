import {chooseFiles} from './file-picker.js';

const MAX_BYTES = 16 * 1024 * 1024;
const validId = value => Number.isInteger(value) && value > 0 && value <= 0xffffffff;

function validate(request) {
  if (!validId(request.request_id)) throw new Error('Invalid file request identifier.');
  if (!Number.isInteger(request.max_bytes) || request.max_bytes < 1 || request.max_bytes > MAX_BYTES) {
    throw new Error('Choose a file size limit between 1 byte and 16 MiB.');
  }
  if (!Array.isArray(request.extensions) || request.extensions.length > 16 ||
      request.extensions.some(value => typeof value !== 'string' || !/^\.[a-z0-9]{1,16}$/.test(value))) {
    throw new Error('Choose at most 16 lowercase file extensions.');
  }
}

function validateFile(file, request) {
  if (!file || typeof file.name !== 'string' || !file.name.trim() ||
      new TextEncoder().encode(file.name).length > 255 || /[/\\\p{Cc}]/u.test(file.name) ||
      ['.', '..'].includes(file.name)) throw new Error('The selected file has an invalid portable name.');
  if (typeof file.type !== 'string' || file.type.length > 128 || /[^\x20-\x7e]/.test(file.type)) {
    throw new Error('The selected file has an invalid media type.');
  }
  if (request.extensions.length && !request.extensions.some(extension => file.name.toLowerCase().endsWith(extension))) {
    throw new Error('The selected file does not match the requested file types.');
  }
  if (!Number.isSafeInteger(file.size) || file.size < 0 || file.size > request.max_bytes) {
    throw new Error('The selected file exceeds the requested size limit.');
  }
  if (typeof file.arrayBuffer !== 'function') throw new Error('The selected file cannot be read.');
}

// The picker is invoked synchronously before the first await. This is required
// for user activation even though the result is delivered asynchronously.
export async function chooseFile(runtime, request, picker = chooseFiles) {
  const id = request.request_id;
  try {
    validate(request);
    if (!runtime.isFileTaskPending(id)) return false;
    const selection = picker(request.extensions.join(','), false);
    const files = await selection;
    if (!runtime.isFileTaskPending(id)) return false;
    if (!Array.isArray(files) || files.length > 1) throw new Error('Choose one file.');
    if (!files.length) return runtime.cancelFileTask(id);
    const file = files[0];
    validateFile(file, request);
    const buffer = await file.arrayBuffer();
    if (!runtime.isFileTaskPending(id)) return false;
    if (!(buffer instanceof ArrayBuffer) || buffer.byteLength !== file.size || buffer.byteLength > request.max_bytes) {
      throw new Error('The selected file changed size or exceeded its limit while reading.');
    }
    return runtime.completeFileTask(id, file.name, file.type, new Uint8Array(buffer));
  } catch (error) {
    if (!validId(id) || !runtime.isFileTaskPending(id)) return false;
    if (error?.name === 'AbortError') return runtime.cancelFileTask(id);
    return runtime.failFileTask(id, String(error?.message || error).slice(0, 1024));
  }
}
