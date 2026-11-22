import assert from "node:assert/strict";

export async function verifyOfflineStartup(page, { ready, until, capture, clickName, isRoute }) {
  const canonical = new URL(".", page.url).href;
  const navigate = async action => {
    const previous = await page.evaluate("performance.timeOrigin");
    await action();
    await until(() => page.evaluate(`location.href === ${JSON.stringify(canonical)} &&
      performance.timeOrigin !== ${JSON.stringify(previous)}`), "canonical directory document loaded");
    await ready();
    await isRoute("Start");
  };
  // Registration is deliberately enabled here; the rest of the smoke suite
  // loads with ?nooffline=1 to isolate ordinary UI failures from shell caching.
  await navigate(() => page.send("Page.navigate", { url: canonical }));
  await until(() => page.evaluate(`navigator.serviceWorker?.controller?.scriptURL ===
    new URL("app-sw.js", location.href).href`), "app worker installed and controlling the page", 30000);

  await page.send("Network.emulateNetworkConditions", {
    offline: true, latency: 0, downloadThroughput: 0, uploadThroughput: 0,
  });
  try {
    await navigate(() => page.send("Page.reload", {}));
    assert.equal(await page.evaluate("navigator.onLine"), false, "Chromium must really be offline");
    await capture("15-start-mobile-offline");
    await clickName("HardwareSearch");
    await isRoute("Search");
    await clickName("HardwareBack");
    await isRoute("Start");
  } finally {
    await page.send("Network.emulateNetworkConditions", {
      offline: false, latency: 0, downloadThroughput: -1, uploadThroughput: -1,
    });
  }
}
