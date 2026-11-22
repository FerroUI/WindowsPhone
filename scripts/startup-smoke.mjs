import assert from "node:assert/strict";
import path from "node:path";
import { open, sleep, colours } from "../FerroUI/scripts/browser/harness.mjs";

// Routine publication checks one real frame. The longer interaction/capture
// suite remains opt-in, so application startup cannot silently break Pages.
let page;
try {
  page = await open(path.resolve(process.argv[2]), {
    query: "?nooffline=1", width: 390, height: 844, network: true,
  });
  const deadline = Date.now() + 45000;
  let ready = false;
  while (Date.now() < deadline) {
    assert.equal(page.errors.length, 0, page.errors.join("\n"));
    ready = await page.evaluate(`!!globalThis.phoneApp?.inspectView &&
      JSON.parse(phoneApp.inspectView())?.elements.some(element =>
        element.hit && element.name === "HardwareStart")`);
    if (ready) break;
    await sleep(100);
  }
  assert(ready, "The native phone did not finish its first layout.");
  await sleep(150);
  assert.equal(page.errors.length, 0, page.errors.join("\n"));
  assert.equal(JSON.parse(await page.evaluate("phoneApp.currentRoute()")), "Start");
  assert(colours(await page.screenshot()) > 40, "The first frame is empty.");
  console.log("PASS: native mobile Start rendered without runtime errors.");
} finally {
  if (page) await page.close();
}
