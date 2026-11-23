// A downloaded worker waits for every old phone window to close. Taking over
// immediately can mix cached JS and WASM versions while another tab is loading.
let registrationPromise;
let notify = () => {};
const scope = new URL('./', import.meta.url);
const observed = new WeakSet();
function waiting() {
  notify('An update is downloaded. Close all Windows Phone windows, then open the app again to install it. Your saved data will be kept.');
}
function observe(registration) {
  if (registration.waiting) waiting();
  if (observed.has(registration)) return registration;
  observed.add(registration);
  const watch = worker => {
    if (!worker) return;
    let installed = ['installed', 'activating', 'activated'].includes(worker.state);
    worker.addEventListener('statechange', () => {
      if (worker.state === 'installed') {
        installed = true;
        if (navigator.serviceWorker.controller) waiting();
      }
      // A successfully installed worker also becomes redundant when a newer
      // worker replaces it. That transition is not an installation failure.
      if (worker.state === 'redundant' && !installed) notify('The update did not finish installing. Your current app is still available.');
    });
  };
  watch(registration.installing);
  registration.addEventListener('updatefound', () => watch(registration.installing));
  return registration;
}
export function installUpdates(report) {
  notify = report;
  if (!('serviceWorker' in navigator)) return Promise.resolve(undefined);
  registrationPromise ||= navigator.serviceWorker.register(new URL('app-sw.js', scope), {scope:scope.href, updateViaCache:'none'}).then(observe).catch(error => {
    registrationPromise = undefined;
    throw error;
  });
  return registrationPromise;
}
export async function checkForUpdates(report) {
  notify = report;
  if (!('serviceWorker' in navigator)) throw new Error('Offline app updates are unavailable in this browser. Reload the page while online to use the current version.');
  if (navigator.onLine === false) throw new Error('Connect to the internet to check for updates.');
  const registration = await (registrationPromise || navigator.serviceWorker.getRegistration(scope.href).then(existing => existing ? observe(existing) : installUpdates(report)));
  await registration.update();
  if (registration.waiting) waiting();
  else if (registration.installing) report('Downloading an update. Keep this app open until the download completes.');
  else report('No newer offline app update is available.');
}
