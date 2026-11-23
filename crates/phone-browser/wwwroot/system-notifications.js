// Permission is requested only by the explicit Settings command. Delivering a
// foreground phone event never opens a permission prompt.
export function notificationPermission(platform = globalThis) {
  if (!platform.Notification) return 'unsupported';
  const permission = platform.Notification.permission;
  return permission === 'default' ? 'prompt'
    : ['granted', 'denied'].includes(permission) ? permission : 'unknown';
}

export async function requestNotificationPermission(platform = globalThis) {
  const current = notificationPermission(platform);
  if (current === 'unsupported') throw new Error('System notifications are unavailable on this host.');
  if (current !== 'prompt') return current;
  // Invoke before any await so the browser receives the original user gesture.
  await platform.Notification.requestPermission();
  return notificationPermission(platform);
}

export async function showSystemNotification(title, body, platform = globalThis, isCurrent = () => true) {
  if (!isCurrent()) return false;
  if (notificationPermission(platform) !== 'granted') return false;
  const options = {
    body: String(body).slice(0, 4096),
    icon: new URL('./icon-192.png', import.meta.url).href,
    badge: new URL('./icon-192.png', import.meta.url).href,
    data: {phone: true},
    silent: true,
  };
  const caption = String(title).slice(0, 256);
  options.tag = `windows-phone:${caption}`;
  const workers = platform.navigator?.serviceWorker;
  // `ready` can wait forever when offline installation is disabled. Query the
  // app's registration once and use its active worker when available instead.
  const registration = workers?.getRegistration
    ? await workers.getRegistration(new URL('./', import.meta.url).href) : undefined;
  if (!isCurrent()) return false;
  if (registration?.active && registration.showNotification) {
    await registration.showNotification(caption, options);
    return true;
  }
  // Some mobile platforms expose Notification but forbid its constructor.
  // Their rejection is reported, never described as delivered successfully.
  const notification = new platform.Notification(caption, options);
  notification.addEventListener('click', () => {
    notification.close();
    platform.focus?.();
  }, {once: true});
  return true;
}
