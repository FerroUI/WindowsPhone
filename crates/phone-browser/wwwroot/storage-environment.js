function bytes(value) {
  return Number.isFinite(value) && value >= 0 && value <= Number.MAX_SAFE_INTEGER
    ? Math.floor(value) : null;
}

// Origin estimates include this origin's other installed apps and HTTP caches.
// They are deliberately separate from the exact saved phone-document size.
export async function readStorageEnvironment(storage) {
  const result = {
    storage_usage_bytes: null,
    storage_quota_bytes: null,
    storage_persisted: null,
    persistent_storage_supported: Boolean(storage?.persist),
  };
  const [estimate, persisted] = await Promise.allSettled([
    Promise.resolve().then(() => storage?.estimate?.()),
    Promise.resolve().then(() => storage?.persisted?.()),
  ]);
  if (estimate.status === 'fulfilled') {
    result.storage_usage_bytes = bytes(estimate.value?.usage);
    result.storage_quota_bytes = bytes(estimate.value?.quota);
  }
  if (persisted.status === 'fulfilled' && typeof persisted.value === 'boolean') {
    result.storage_persisted = persisted.value;
  }
  return result;
}

export async function requestPersistentStorage(storage = globalThis.navigator?.storage) {
  if (!storage?.persist) throw new Error('Persistent storage is unavailable in this browser.');
  // This may display a host prompt; callers must originate from user input.
  return Boolean(await storage.persist());
}
