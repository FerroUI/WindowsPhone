import assert from "node:assert/strict";
import {readFileSync} from "node:fs";
import test from "node:test";
import {inputGeometryReady, stableInputTarget} from "./ui-input-geometry.mjs";

const captured = JSON.parse(readFileSync(new URL("./fixtures/ui-native-input-frames.json", import.meta.url)));
const page = element => element.name === "PhonePage";
const create = element => element.name === "AppBarAction-new";

test("real native retiring and hittable incoming frames cannot supply normal pointer coordinates", async () => {
  assert(captured.incoming.elements.find(page).hit, "the regression frame passes the old hit-only lookup");
  for (const view of [captured.outgoing, captured.incoming]) {
    assert.equal(inputGeometryReady(view), false);
    let frames = 0;
    assert.equal(await stableInputTarget({view, match: page, label: "page",
      frame: async () => frames++, snapshot: async () => captured.settled}), undefined);
    assert.equal(frames, 0, "moving geometry is rejected before retaining a coordinate");
  }
  assert.equal(inputGeometryReady(captured.settled), true);
});

test("pointer acquisition rejects an entrance beginning after the first ready snapshot", async () => {
  let frames = 0;
  const target = await stableInputTarget({view: captured.settled, match: page, label: "page",
    frame: async () => frames++, snapshot: async () => {
      assert.equal(frames, 1, "the second read follows rendered frames");
      return captured.incoming;
    }});
  assert.equal(target, undefined);
});

test("normal clicks reacquire the target and reject layout movement or disappearance", async () => {
  const fresh = structuredClone(captured.hub);
  const acquire = after => stableInputTarget({view: captured.hub, match: create, label: "new",
    frame: async () => {}, snapshot: async () => after});
  assert.equal(await acquire(fresh), fresh.elements.find(create), "dispatch uses the fresh native target");
  const shifted = structuredClone(fresh), moved = shifted.elements.find(create);
  moved.x += 2; moved.renderedBounds.x += 2;
  assert.equal(await acquire(shifted), undefined, "a stable page can still contain a moving target");
  assert.equal(await acquire({elements: fresh.elements.filter(element => !create(element))}), undefined,
    "an unmounted target cannot reuse the previous frame's position");
});

test("the captured lock screen remains actionable without a mounted PhonePage", async () => {
  assert(!captured.lock.elements.some(page));
  const fresh = structuredClone(captured.lock);
  const target = await stableInputTarget({view: captured.lock, label: "Unlock",
    match: element => element.automationName === "Unlock",
    frame: async () => {}, snapshot: async () => fresh});
  assert.equal(target, fresh.elements.find(element => element.automationName === "Unlock"));
  assert(target?.hit, "the lock surface retains its actual native input target");
});
