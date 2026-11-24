import assert from "node:assert/strict";
import test from "node:test";
import { assertUnprojectedFrames } from "./ui-metro-controls.mjs";

const destination = { Setting: "ringtones+sounds" };
const frame = (route, projection = null) => ({
  elapsed: 16,
  route,
  page: { x: 440, y: 87, width: 400, height: 740, projection },
});

test("reduced-motion evidence permits an unmounted replacement frame but requires a rendered destination", () => {
  assert.doesNotThrow(() => assertUnprojectedFrames([
    frame({ App: "Settings" }),
    { elapsed: 32, route: destination, page: null },
    frame(destination),
    frame(destination),
  ], destination));
});

test("reduced-motion evidence rejects any actual projection including on the outgoing page", () => {
  for (const projection of [
    { kind: "turnstile", angleY: -40, centerX: -200, depth: 833.33 },
    { kind: "turnstile", angleY: 25, centerX: -200, depth: 833.33 },
    { kind: "slide", translateX: 100 },
  ]) {
    for (const route of [{ App: "Settings" }, destination]) {
      assert.throws(() => assertUnprojectedFrames([
        frame(route, projection), frame(destination), frame(destination),
      ], destination), /rendered a decorative projection/);
    }
  }
});

test("reduced-motion evidence cannot pass on missing frames, the wrong route or a missing final page", () => {
  for (const frames of [
    [{ route: destination, page: null }],
    [frame({ App: "Settings" }), frame({ App: "Settings" })],
    [frame(destination)],
    [frame(destination), frame(destination), { route: destination, page: null }],
    [frame(destination), frame(destination), frame({ App: "Settings" })],
  ]) assert.throws(() => assertUnprojectedFrames(frames, destination));
});

test("reduced-motion evidence requires an explicit native projection observation", () => {
  const unknown = frame(destination);
  delete unknown.page.projection;
  assert.throws(() => assertUnprojectedFrames([
    unknown, frame(destination), frame(destination),
  ], destination), /rendered a decorative projection/);
});
