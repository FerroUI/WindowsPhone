// Some Chromium viewport emulators report device-pixel content boxes from the
// physical display while advertising a different window.devicePixelRatio.
// FerroUI must receive one consistent size/scale pair for layout and input.
export function normalizeCanvasResizeEntry(entry, root, pixelRatio) {
  if (!root.contains(entry.target) || !entry.target.matches('canvas.ferroui-canvas')) return entry;
  const deviceBox = entry.devicePixelContentBoxSize?.[0];
  if (!deviceBox || !Number.isFinite(pixelRatio) || pixelRatio <= 0) return entry;
  const width = Math.round(entry.contentRect.width * pixelRatio);
  const height = Math.round(entry.contentRect.height * pixelRatio);
  if (width <= 0 || height <= 0 || !Number.isFinite(width + height)) return entry;
  // Preserve the browser's precise rounding on ordinary displays and zooms.
  if (Math.abs(deviceBox.inlineSize - width) <= 1 &&
      Math.abs(deviceBox.blockSize - height) <= 1) return entry;
  const normalized = Object.freeze([Object.freeze({inlineSize: width, blockSize: height})]);
  return new Proxy(entry, {
    get(target, property) {
      return property === 'devicePixelContentBoxSize'
        ? normalized : Reflect.get(target, property, target);
    },
  });
}

export function installCanvasSizing(root) {
  const NativeResizeObserver = globalThis.ResizeObserver;
  if (!root || typeof NativeResizeObserver !== 'function') return;
  globalThis.ResizeObserver = class extends NativeResizeObserver {
    constructor(callback) {
      super(function (entries, observer) {
        const normalized = entries.map(entry =>
          normalizeCanvasResizeEntry(entry, root, globalThis.devicePixelRatio));
        const changed = normalized.some((entry, index) => entry !== entries[index]);
        callback.call(this, changed ? normalized : entries, observer);
      });
    }
  };
}
