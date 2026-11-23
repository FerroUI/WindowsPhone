// Prepare all selected documents synchronously so the native share call retains
// the user's activation. No picker, download or share occurs on partial failure.
const MAX_FILES = 512;
const MAX_BYTES = 32 * 1024 * 1024;

export function documentFiles(runtime, ids, FileType = File) {
  if (!Array.isArray(ids) || !ids.length || ids.length > MAX_FILES ||
      ids.some(id => !Number.isSafeInteger(id) || id <= 0)) {
    throw new Error('Select valid documents to share.');
  }
  let bytes = 0;
  return [...new Set(ids)].map(id => {
    const document = JSON.parse(runtime.exportOffice(id));
    if (typeof document.name !== 'string' || !document.name ||
        typeof document.mime !== 'string' || !Array.isArray(document.bytes)) {
      throw new Error('This document could not be prepared for sharing.');
    }
    bytes += document.bytes.length;
    if (bytes > MAX_BYTES) throw new Error('Share up to 32 MB of documents at a time.');
    return new FileType([Uint8Array.from(document.bytes)], document.name, {type:document.mime});
  });
}

export async function shareDocuments(runtime, ids, platform = navigator) {
  if (!platform.share || !platform.canShare) {
    throw new Error('File sharing is unavailable in this browser. Export the documents to save them.');
  }
  const files = documentFiles(runtime, ids);
  if (!platform.canShare({files})) {
    throw new Error('This browser cannot share these document files. Export them to save a copy.');
  }
  try {
    await platform.share({files});
    return true;
  } catch (error) {
    if (error.name === 'AbortError') return false;
    throw error;
  }
}
