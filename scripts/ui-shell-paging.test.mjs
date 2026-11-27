import assert from "node:assert/strict";
import test from "node:test";
import { assertHeldShellPaging, assertLiveTileHandoff, assertShellPaging, assertShellReturn } from "./ui-navigation-motion.mjs";

const resting = { x: 100, y: 20, width: 320, height: 660 };
const surface = (offset, enabled = false) => ({
  ...resting, enabled, hitTestVisible: enabled, hit: false, opacity: 1,
  projection: { kind: "slide", translateX: offset },
  translation: { x: 0, y: 0, z: 0 }, tabNavigation: "Continue",
  renderedBounds: { ...resting, x: resting.x + offset },
});
const cleanNeighbor = () => ({
  neighbor: { ...surface(0), opacity: 0, projection: null },
  neighborImage: { imageSource: null }, staged: null,
});
const finalFrame = route => ({
  route, elapsed: 400,
  page: { ...surface(0, true), projection: null },
  outgoing: { ...surface(0), projection: null, opacity: 0 },
  image: { imageSource: null }, ...cleanNeighbor(),
});
function fixture(direction = 1) {
  const route = direction === 1 ? "Apps" : "Start";
  const frames = [190, 80, 5].map((distance, index) => ({
    elapsed: index * 100, route,
    page: { ...surface(direction * distance, true), hitTestVisible: false, tabNavigation: "None" },
    outgoing: surface(direction * (distance - resting.width)),
    image: { imageSource: { width: resting.width, height: resting.height } },
    ...cleanNeighbor(),
  }));
  frames.push(finalFrame(route));
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
    frames => { frames[1].page.enabled = false; },
    frames => { frames[1].page.hitTestVisible = true; },
    frames => { frames[1].page.hit = true; },
    frames => { frames[1].page.tabNavigation = "Continue"; },
    frames => frames.forEach(frame => {
      frame.page.renderedBounds.x = resting.x;
      frame.outgoing.renderedBounds.x = resting.x;
    }),
    frames => { frames.at(-1).image.imageSource = { width: 320, height: 660 }; },
    frames => { frames.at(-1).neighborImage.imageSource = { width: 320, height: 660 }; },
    frames => { frames[0].neighborImage.imageSource = { width: 320, height: 660 }; },
    frames => { frames.at(-1).staged = { name: "PhoneNeighborCapture" }; },
  ]) {
    const { route, frames } = fixture();
    mutate(frames);
    assert.throws(() => assertShellPaging(frames, route, 1, resting));
  }
});

function heldFixture(direction = 1) {
  const route = direction === 1 ? "Start" : "Apps";
  const frames = [32, 78, 100, 100].map((distance, index) => {
    const delta = -direction * distance;
    return { route, elapsed: index * 50,
      page: { ...surface(delta, true), projection: null, translation: { x: delta, y: 0, z: 0 } },
      neighbor: { ...surface(delta + direction * resting.width), projection: null,
        translation: { x: delta + direction * resting.width, y: 0, z: 0 } },
      neighborImage: { imageSource: { width: 320, height: 660 }, enabled: false, hitTestVisible: false, hit: false },
      outgoing: { ...surface(0), opacity: 0, projection: null }, image: { imageSource: null }, staged: null,
    };
  });
  // An acquisition observation may still read the preceding compositor frame.
  frames[0].page.renderedBounds.x = resting.x;
  frames[0].neighbor.renderedBounds.x = resting.x;
  return { route, frames };
}

test("held paging evidence requires native content and matching completed compositor geometry in both directions", () => {
  for (const direction of [1, -1]) {
    const { route, frames } = heldFixture(direction);
    assert.doesNotThrow(() => assertHeldShellPaging(frames, route, direction, resting));
  }
});

test("held paging evidence rejects missing content, route mutation, gaps, live preview input and retained owners", () => {
  for (const mutate of [
    frames => frames.forEach(frame => { frame.neighborImage.imageSource = null; }),
    frames => { frames[1].route = "Apps"; },
    frames => { frames[1].neighbor.translation.x += 20; },
    frames => { frames[1].neighbor.enabled = true; },
    frames => { frames[1].neighborImage.hit = true; },
    frames => { frames[1].neighborImage.imageSource.height = 1; },
    frames => { frames[1].staged = { name: "PhoneNeighborCapture" }; },
    frames => { frames[1].image.imageSource = { width: 320, height: 660 }; },
    frames => frames.forEach(frame => { frame.neighbor.renderedBounds.x = resting.x; }),
  ]) {
    const { route, frames } = heldFixture();
    mutate(frames);
    assert.throws(() => assertHeldShellPaging(frames, route, 1, resting));
  }
});

function returnFixture(direction = 1) {
  const route = direction > 0 ? "Start" : "Apps";
  const frames = [16, 8, 1].map((distance, index) => ({
    route, elapsed: index * 60,
    page: { ...surface(-direction * distance, true), hitTestVisible: false, tabNavigation: "None" },
    neighbor: surface(direction * (resting.width - distance)),
    neighborImage: { imageSource: { width: 320, height: 660 }, enabled: false, hitTestVisible: false, hit: false },
    outgoing: { ...surface(0), opacity: 0, projection: null }, image: { imageSource: null }, staged: null,
  }));
  frames.push(finalFrame(route));
  return { route, frames };
}

test("rejected paging evidence retains source ink, pairs the return and restores all transient state", () => {
  for (const direction of [1, -1]) {
    const { route, frames } = returnFixture(direction);
    assert.doesNotThrow(() => assertShellReturn(frames, route, direction, resting));
  }
});

test("rejected paging evidence rejects route changes, detached edges, input admission and leaked images", () => {
  for (const mutate of [
    frames => { frames[1].route = "Apps"; },
    frames => { frames[1].neighbor.projection.translateX += 20; },
    frames => { frames[1].page.enabled = false; },
    frames => { frames[1].page.hitTestVisible = true; },
    frames => { frames[1].page.tabNavigation = "Continue"; },
    frames => { frames.at(-1).neighborImage.imageSource = { width: 320, height: 660 }; },
    frames => { frames.at(-1).neighbor.translation.x = 18; },
  ]) {
    const { route, frames } = returnFixture();
    mutate(frames);
    assert.throws(() => assertShellReturn(frames, route, 1, resting));
  }
});

function liveFixture() {
  return [40, 150, 500].map(elapsed => ({ elapsed, route: "Start",
    page: { projection: elapsed < 350 ? { kind: "slide" } : null },
    tiles: [{ opacity: 1, projection: null }],
    tileButtons: [{ opacity: 1, enabled: true, hit: false }],
  }));
}
test("live handoff evidence requires ordinary native tile ink without a replayed entrance", () => {
  assert.doesNotThrow(() => assertLiveTileHandoff(liveFixture()));
  for (const mutate of [
    frames => frames.forEach(frame => { frame.tiles = []; }),
    frames => { frames[0].tiles[0].projection = { kind: "slide", translateY: 24 }; },
    frames => { frames[0].tiles[0].opacity = 0.12; },
    frames => { frames[0].tileButtons = []; },
    frames => { frames[0].tileButtons[0].opacity = 0.35; },
    frames => { frames[0].tileButtons[0].enabled = false; },
    frames => { frames[0].tileButtons[0].hit = true; },
  ]) {
    const frames = liveFixture();
    mutate(frames);
    assert.throws(() => assertLiveTileHandoff(frames));
  }
});
