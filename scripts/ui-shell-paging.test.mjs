import assert from "node:assert/strict";
import test from "node:test";
import { assertShellPaging } from "./ui-navigation-motion.mjs";

const resting = { x: 100, y: 20, width: 320, height: 660 };
function fixture(direction = 1) {
  const route = direction === 1 ? "Apps" : "Start";
  const surface = (offset, enabled = false) => ({
    ...resting, enabled, hitTestVisible: enabled, hit: false, opacity: 1,
    projection: { kind: "slide", translateX: offset },
    renderedBounds: { ...resting, x: resting.x + offset },
  });
  const frames = [190, 80, 5].map((distance, index) => ({
    elapsed: index * 100, route,
    page: surface(direction * distance),
    outgoing: surface(direction * (distance - resting.width)),
    image: { imageSource: { width: resting.width, height: resting.height } },
  }));
  frames.push({ route, elapsed: 400,
    page: { ...surface(0, true), projection: null },
    outgoing: { ...surface(0), projection: null, opacity: 0 },
    image: { imageSource: null },
  });
  return { route, frames };
}

test("shell paging evidence accepts concurrent native movement in both directions", () => {
  for (const direction of [1, -1]) {
    const { route, frames } = fixture(direction);
    assert.doesNotThrow(() => assertShellPaging(frames, route, direction, resting));
  }
});

test("shell paging evidence rejects missing image, a visual gap, snap-back and interactive destination", () => {
  for (const mutate of [
    frames => frames.forEach(frame => { frame.image.imageSource = null; }),
    frames => { frames[1].outgoing.projection.translateX += 30; },
    frames => { frames[1].page.projection.translateX = 250;
      frames[1].outgoing.projection.translateX = -70; },
    frames => { frames[1].page.enabled = true; },
    frames => frames.forEach(frame => {
      frame.page.renderedBounds.x = resting.x;
      frame.outgoing.renderedBounds.x = resting.x;
    }),
    frames => { frames.at(-1).image.imageSource = { width: 320, height: 660 }; },
  ]) {
    const { route, frames } = fixture();
    mutate(frames);
    assert.throws(() => assertShellPaging(frames, route, 1, resting));
  }
});
