// BFCache suspension keeps the runtime; permanent unload or an embedding
// owner's close releases it. Focus loss alone must not cancel OS prompts.
export function bindRuntimeLifecycle(runtime, {platform = globalThis, onResume = () => {}, onSuspend = () => {}, onClose = () => {}} = {}) {
  let visible = !platform.document?.hidden, closed = false;
  const removals = [];
  const listen = (target, event, callback) => {
    target?.addEventListener?.(event, callback);
    removals.push(() => target?.removeEventListener?.(event, callback));
  };
  const visibility = value => {
    if (closed || visible === value) return;
    visible = value;
    runtime.hostVisibility(value);
    if (value) onResume();
    else { runtime.cancelInput(); onSuspend(); }
  };
  const close = () => {
    if (closed) return;
    visibility(false);
    closed = true;
    for (const remove of removals) remove();
    runtime.closeHost();
    onClose();
  };
  listen(platform.document, 'visibilitychange', () => visibility(!platform.document.hidden));
  listen(platform, 'blur', () => { if (!closed) runtime.cancelInput(); });
  listen(platform, 'pagehide', event => { visibility(false); if (!event.persisted) close(); });
  listen(platform, 'pageshow', () => visibility(!platform.document.hidden));
  runtime.hostVisibility(visible);
  return close;
}

export function activationFromUrl(value) {
  const url = new URL(value);
  const parameter = url.searchParams.get('activate');
  // The fragment contains an inner URI. Preserve its escapes until Rust parses
  // its query pairs; decoding here could turn an encoded '&' into a parameter.
  const hash = url.hash.startsWith('#ferroui-phone:') ? url.hash.slice(1) : undefined;
  const activation = parameter || hash;
  if (activation && (activation.length > 8192 || !activation.startsWith('ferroui-phone:'))) {
    throw new Error('Use a ferroui-phone activation URI for an app route.');
  }
  return activation;
}
