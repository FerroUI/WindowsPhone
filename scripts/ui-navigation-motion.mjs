import assert from "node:assert/strict";
import { mkdir, writeFile } from "node:fs/promises";
import path from "node:path";
import { assertUnprojectedFrames } from "./ui-metro-controls.mjs";

const sameRoute = (actual, expected) => JSON.stringify(actual) === JSON.stringify(expected);
const hasImage = frame => frame.image?.imageSource != null;
const hasNeighbor = frame => frame.neighborImage?.imageSource != null;
const near = (actual, expected, label, tolerance = 0.6) =>
  assert(Number.isFinite(actual) && Math.abs(actual - expected) <= tolerance,
    `${label}: ${actual} versus ${expected}`);
const diagnostic = frames => JSON.stringify({
  count: frames.length,
  frames: frames.filter((frame, index) => index === 0 || index === frames.length - 1 ||
    index % Math.max(1, Math.floor(frames.length / 12)) === 0),
});

function assertRestored(frame, route, resting) {
  assert(frame?.page && sameRoute(frame.route, route), "The destination must remain mounted");
  assert.equal(frame.page.projection, null, "The incoming native transform must be released");
  assert.equal(frame.page.opacity, 1, "The destination must restore its opacity");
  assert.equal(frame.page.enabled, true, "The destination must restore native input");
  assert.equal(frame.page.hitTestVisible, true, "The destination must restore hit testing");
  assert(!hasImage(frame), "The outgoing native Image must release its source");
  assert(frame.outgoing, "The reusable outgoing surface must remain mounted");
  assert.equal(frame.outgoing.opacity, 0, "The empty outgoing surface must be transparent");
  assert.equal(frame.outgoing.projection, null, "The outgoing transform must be released");
  assert(frame.neighbor, "The reusable held-neighbor surface must remain mounted");
  assert(!hasNeighbor(frame), "The held-neighbor image must release its source");
  assert.equal(frame.neighbor.opacity, 0);
  assert.equal(frame.neighbor.projection, null);
  near(frame.neighbor.translation?.x, 0, "Retired neighbor translation");
  assert(!frame.staged, "No temporary destination tree may survive the gesture");
  for (const property of ["x", "y", "width", "height"])
    near(frame.page[property], resting[property], `Restored PhonePage ${property}`);
}

// These assertions require native source, rendered geometry and input state.
// A named empty Border, a simulated angle, or an unmounted final page cannot
// satisfy the retiring-page evidence.
export function assertNavigationChoreography(frames, route, backward, resting) {
  const details = diagnostic(frames);
  const destination = frames.filter(frame => sameRoute(frame.route, route));
  const retiring = destination.filter(frame => hasImage(frame) && frame.outgoing?.opacity > 0);
  assert(retiring.length >= 2, `The outgoing rendered image must survive multiple frames: ${details}`);
  const sign = backward ? -1 : 1;
  const poses = retiring.filter(frame => frame.outgoing.projection?.kind === "turnstile" &&
    frame.outgoing.projection.angleY * sign > 1);
  assert(poses.length, `The outgoing native rotation must have its ${backward ? "backward" : "forward"} sign: ${details}`);
  let previous = 0;
  for (const frame of poses) {
    const { outgoing, page, image } = frame;
    const angle = outgoing.projection.angleY * sign;
    assert(angle >= previous - 0.2 && angle <= (backward ? 80 : 50) + 0.1,
      "The outgoing rotation must accelerate toward its reference angle");
    previous = angle;
    near(outgoing.projection.centerX, -outgoing.width / 2, "Outgoing left-edge hinge", 0.001);
    near(outgoing.projection.depth, outgoing.width * 1000 / 480, "Outgoing perspective camera", 0.001);
    near(image.imageSource.width, resting.width, "Native image source logical width");
    near(image.imageSource.height, resting.height, "Native image source logical height");
    assert.equal(outgoing.enabled, false, "The retiring image must not own input");
    assert.equal(outgoing.hitTestVisible, false, "The retiring image must not participate in hit testing");
    assert.equal(outgoing.hit, false, "The retiring image must not be a native hit target");
    assert(page, "The destination must already own the live page while the image retires");
    assert.equal(page.opacity, 0, "The destination must remain hidden during the outgoing phase");
    assert.equal(page.enabled, false, "Input must not fall through into the hidden destination");
    assert.equal(page.hitTestVisible, false, "The hidden destination must reject hit testing");
    assert.equal(page.projection, null, "The incoming rotation starts after the outgoing image retires");
  }
  assert(poses.some(({ outgoing }) => outgoing.renderedBounds &&
    Math.abs(outgoing.renderedBounds.width - resting.width) > 2),
  `A turnstile must change the compositor's rendered bounds: ${details}`);
  const incoming = destination.filter(frame => frame.page?.projection?.kind === "turnstile" &&
    frame.page.projection.angleY * -sign > 1);
  assert(incoming.length, `The live destination must render the opposite incoming rotation: ${details}`);
  for (const frame of incoming) {
    assert(!hasImage(frame), "No outgoing image may survive into the incoming phase");
    assert.equal(frame.outgoing.opacity, 0);
    assert.equal(frame.page.enabled, true, "The live incoming page restores input ownership");
    assert.equal(frame.page.hitTestVisible, true);
  }
  assertRestored(frames.at(-1), route, resting);
}

// Shell paging is a concurrent translation, not two independent entrances.
// Require both native tracks plus compositor movement: two dummy adjacent
// controls or the outgoing image parked at rest cannot satisfy this evidence.
export function assertShellPaging(frames, route, direction, resting) {
  const details = diagnostic(frames);
  const paired = frames.filter(frame => sameRoute(frame.route, route) && hasImage(frame) &&
    frame.outgoing?.opacity > 0 && frame.outgoing.projection?.kind === "slide" &&
    frame.page?.projection?.kind === "slide");
  assert(paired.length >= 2, `Shell paging must retain both moving surfaces: ${details}`);
  let previousIncoming = Infinity;
  for (const { page, outgoing, image } of paired) {
    const incomingX = page.projection.translateX * direction;
    const outgoingX = outgoing.projection.translateX * direction;
    assert(incomingX >= -0.1 && incomingX <= resting.width + 0.1);
    assert(outgoingX <= 0.1 && outgoingX >= -resting.width - 0.1);
    assert(incomingX <= previousIncoming + 0.6, "The settle cannot move back toward the pointer origin");
    previousIncoming = incomingX;
    near(incomingX - outgoingX, resting.width, "Native page edges remain adjacent", 0.01);
    near(image.imageSource.width, resting.width, "Outgoing image logical width");
    near(image.imageSource.height, resting.height, "Outgoing image logical height");
    assert.equal(page.opacity, 1, "Paging cannot expose the background by fading either page");
    assert.equal(outgoing.opacity, 1);
    assert.equal(page.enabled, true, "The visible destination preserves normal enabled ink");
    assert.equal(page.hitTestVisible, false);
    assert.equal(page.hit, false, "The moving destination must not be a native hit target");
    assert.equal(page.tabNavigation, "None", "Tab cannot activate the moving destination");
    assert.equal(outgoing.enabled, false);
    assert.equal(outgoing.hitTestVisible, false);
    assert.equal(outgoing.hit, false);
  }
  for (const frame of frames.filter(frame => sameRoute(frame.route, route)))
    assert(!hasNeighbor(frame), "Commit releases the held image before outgoing capture owns a bitmap");
  assert(paired.some(({ page, outgoing }) => page.renderedBounds && outgoing.renderedBounds &&
    Math.abs(page.renderedBounds.x - resting.x) > 1 &&
    Math.abs(outgoing.renderedBounds.x - resting.x) > 1),
  `Both pages must actually move in the compositor: ${details}`);
  assertRestored(frames.at(-1), route, resting);
}

// Visual.Translation is the native direct-manipulation value; renderedBounds
// comes from the latest completed compositor frame. Initial readbacks may lag
// that value. Require multiple independently matching observations while held,
// without claiming zero-frame readback latency at the acquisition boundary.
export function assertHeldShellPaging(frames, route, direction, resting) {
  const held = frames.filter(frame => hasNeighbor(frame) && frame.neighbor?.opacity > 0 &&
    frame.page?.projection === null && frame.neighbor.projection === null);
  assert(held.length >= 2, `A held drag must expose the real neighboring image: ${diagnostic(frames)}`);
  let matches = 0;
  for (const frame of held) {
    const { page, neighbor, neighborImage } = frame;
    assert(sameRoute(frame.route, route), "Holding must preserve the source route and its lifecycle");
    assert(!hasImage(frame), "Held and outgoing bitmap ownership cannot overlap");
    assert(!frame.staged, "The temporary native destination tree is disposed after capture");
    const delta = page.translation?.x;
    assert(delta * direction < 0 && Math.abs(delta) <= resting.width,
      "The native current page tracks an inward bounded drag");
    near(neighbor.translation?.x - delta, direction * resting.width, "Held native page adjacency", 0.01);
    near(neighborImage.imageSource.width, resting.width, "Held image logical width");
    near(neighborImage.imageSource.height, resting.height, "Held image logical height");
    assert.equal(page.opacity, 1);
    assert.equal(neighbor.opacity, 1);
    for (const surface of [neighbor, neighborImage]) {
      assert.equal(surface.enabled, false, "The preview cannot own destination input");
      assert.equal(surface.hitTestVisible, false);
      assert.equal(surface.hit, false);
    }
    if (page.renderedBounds && neighbor.renderedBounds &&
      Math.abs(page.renderedBounds.x - resting.x - delta) <= 0.6 &&
      Math.abs(neighbor.renderedBounds.x - resting.x - neighbor.translation.x) <= 0.6) {
      near(neighbor.renderedBounds.x - page.renderedBounds.x, direction * resting.width,
        "Completed compositor page adjacency", 0.6);
      near(page.renderedBounds.width, resting.width, "Held page rendered width");
      near(neighbor.renderedBounds.width, resting.width, "Held neighbor rendered width");
      matches++;
    }
  }
  assert(matches >= 2, `Both held surfaces must reach matching completed compositor geometry: ${diagnostic(held)}`);
}

export function assertShellReturn(frames, route, direction, resting) {
  assert(frames.every(frame => sameRoute(frame.route, route)), "A rejected pan never changes the source route");
  const paired = frames.filter(frame => hasNeighbor(frame) && frame.neighbor?.opacity > 0 &&
    frame.neighbor.projection?.kind === "slide" && frame.page?.projection?.kind === "slide");
  assert(paired.length >= 2, `A rejected pan must return both native surfaces: ${diagnostic(frames)}`);
  let previous = Infinity;
  for (const { page, neighbor } of paired) {
    const distance = -page.projection.translateX * direction;
    assert(distance >= -0.1 && distance <= previous + 0.6, "Rejected motion moves toward the source rest position");
    previous = distance;
    near(neighbor.projection.translateX - page.projection.translateX,
      direction * resting.width, "Rejected-pan native adjacency", 0.01);
    assert.equal(page.enabled, true, "The visible source keeps its normal ink during return");
    assert.equal(page.hitTestVisible, false);
    assert.equal(page.tabNavigation, "None", "Tab cannot enter the returning source subtree");
    assert.equal(neighbor.enabled, false);
    assert.equal(neighbor.hitTestVisible, false);
  }
  assertRestored(frames.at(-1), route, resting);
}

export function assertLiveTileHandoff(frames) {
  const mounted = frames.filter(frame => sameRoute(frame.route, "Start") && frame.tiles?.length);
  assert(mounted.length >= 2, "Live Start tile controls must be observed after commit");
  for (const frame of mounted) {
    for (const tile of frame.tiles) {
      assert.equal(tile.projection, null, "Replacing the shown neighbor must not replay a tile entrance");
      assert.equal(tile.opacity, 1, "The live tile content remains opaque at handoff");
    }
    assert(frame.tileButtons?.length, "Actual native tile buttons must accompany their content");
    for (const button of frame.tileButtons) {
      assert.equal(button.enabled, true, "The handoff cannot activate disabled tile styling");
      assert.equal(button.opacity, 1, "The tile button's own opacity remains normal");
      if (frame.page?.projection?.kind === "slide")
        assert.equal(button.hit, false, "Visible moving tile buttons cannot activate");
    }
  }
}

export async function verifyNavigationMotion(ctx) {
  const { page, check, state, snapshot, until, capture, button, centre, control,
    clickLabel, reveal, isRoute, output } = ctx;
  const original = await state();
  const fixture = structuredClone(original);
  Object.assign(fixture.settings, { lock_timeout: 0, reduced_motion: false, battery_saver: false,
    live_tiles: false, high_contrast: false, text_scale: 1, dark: true,
    sounds: false, ringer_volume: 0, vibration: false });
  fixture.events = [];
  fixture.alarms = [];
  fixture.disabled_extensions = [];
  fixture.installed = [...new Set([...fixture.installed, "Settings"])];
  const settingsRoute = { App: "Settings" };
  const detailRoute = { Setting: "ringtones+sounds" };
  const named = (view, name) => view.elements.find(element => element.name === name);
  const sendTouch = (type, point) => page.send("Input.dispatchTouchEvent", {
    type, touchPoints: type === "touchEnd" || type === "touchCancel" ? [] :
      [{ x: point[0], y: point[1], id: 0, radiusX: 2, radiusY: 2, force: 1 }],
  });
  const systemScroll = element => element.name === "SettingsSystemScroll";
  const rawClick = async element => {
    const point = centre(element);
    await page.mouseMove(...point);
    await page.mouseDown(...point);
    await page.mouseUp(...point);
  };
  const load = async data => {
    await page.evaluate(`phoneApp.importData(${JSON.stringify(JSON.stringify(data))})`);
    await isRoute("Start");
    await restingPage();
  };
  const restingPage = () => until(async () => {
    const view = await snapshot();
    const page = named(view, "PhonePage");
    const outgoing = named(view, "PhoneOutgoingPage");
    const neighbor = named(view, "PhoneNeighborPage");
    return page?.projection === null && page.opacity === 1 && page.enabled && page.hitTestVisible &&
      outgoing?.opacity === 0 && outgoing.projection === null &&
      !named(view, "PhoneOutgoingImage")?.imageSource && neighbor?.opacity === 0 &&
      neighbor.projection === null && neighbor.translation?.x === 0 &&
      !named(view, "PhoneNeighborImage")?.imageSource && !named(view, "PhoneNeighborCapture") ? page : undefined;
  }, "Both native page phases release their transforms and image");
  const settingRow = async () => {
    await page.evaluate('phoneApp.openApp("Settings")');
    await isRoute(settingsRoute);
    await restingPage();
    await clickLabel("system");
    await page.wheel(...centre(await control(systemScroll, "Settings system scroll viewer")), 0, -100000);
    return reveal(element => button(element) && element.automationName === "ringtones+sounds",
      "ringtones+sounds", systemScroll, "Settings system scroll viewer");
  };
  const outgoingFrame = (requireProbe = false) => page.evaluate(`new Promise((resolve,reject)=>{
    const started=performance.now();let observed=false;
    const read=()=>{
      const view=JSON.parse(phoneApp.inspectView());
      const find=name=>view.elements.find(element=>element.name===name);
      const outgoing=find('PhoneOutgoingPage'),image=find('PhoneOutgoingImage');
      const page=find('PhonePage'),probe=view.elements.find(element=>/ToggleSwitch$/.test(element.type)&&
        element.automationName?.startsWith('Vibrate,'));
      if(image?.imageSource&&outgoing?.opacity>0&&page&&(!${requireProbe}||probe)){
        // A live source can appear before the destination has a completed
        // compositor readback. Require consecutive animation-frame samples so
        // the recorder observes the phase before a trusted click interrupts it.
        if(observed){resolve({outgoing,image,page,probe});return;}
        observed=true;
      }else observed=false;
      if(performance.now()-started>900){reject(new Error('No live outgoing render target'));return;}
      requestAnimationFrame(read);
    };requestAnimationFrame(read);
  })`);
  const heldFrame = (route, direction, resting) => page.evaluate(`new Promise((resolve,reject)=>{
    const started=performance.now();let ready=0;
    const read=()=>{
      const view=JSON.parse(phoneApp.inspectView());
      const find=name=>view.elements.find(element=>element.name===name)??null;
      const page=find('PhonePage'),neighbor=find('PhoneNeighborPage'),neighborImage=find('PhoneNeighborImage');
      const delta=page?.translation?.x;
      const same=JSON.stringify(JSON.parse(phoneApp.currentRoute()))===${JSON.stringify(JSON.stringify(route))};
      const valid=same&&neighborImage?.imageSource&&neighbor?.opacity===1&&page?.projection===null&&
        neighbor.projection===null&&delta*${direction}<0&&!find('PhoneNeighborCapture')&&
        Math.abs(page.renderedBounds.x-${resting.x}-delta)<=0.6&&
        Math.abs(neighbor.renderedBounds.x-${resting.x}-neighbor.translation.x)<=0.6;
      ready=valid?ready+1:0;
      if(ready>=3){resolve(true);return;}
      if(performance.now()-started>1200){reject(new Error('Held neighboring page never reaches native/compositor adjacency'));return;}
      requestAnimationFrame(read);
    };requestAnimationFrame(read);
  })`);
  const record = async (name, action, duration = 1100, liveTiles = false) => {
    // Install before pointer input. This reads actual completed compositor
    // frames; no clock seeking, native property writes or artificial events.
    const sampled = page.evaluate(`new Promise((resolve,reject)=>{
      const frames=[],started=performance.now();let token,done=false;
      const finish=()=>{if(done)return;done=true;cancelAnimationFrame(token);clearTimeout(timeout);resolve(frames);};
      const timeout=setTimeout(finish,${duration + 900});
      const sample=()=>{try{
        const view=JSON.parse(phoneApp.inspectView());
        const find=name=>view.elements.find(element=>element.name===name)??null;
        frames.push({elapsed:performance.now()-started,route:JSON.parse(phoneApp.currentRoute()),
          page:find('PhonePage'),outgoing:find('PhoneOutgoingPage'),image:find('PhoneOutgoingImage'),
          neighbor:find('PhoneNeighborPage'),neighborImage:find('PhoneNeighborImage'),staged:find('PhoneNeighborCapture'),
          ${liveTiles ? "tiles:view.elements.filter(element=>element.name?.startsWith('LiveTileContent-')),tileButtons:view.elements.filter(element=>/^Tile-[0-9]+$/.test(element.name??'')),cells:view.elements.filter(element=>element.name?.startsWith('PeopleCell-'))," : ""}
          locked:!!find('LockScreen')});
        if(performance.now()-started>=${duration})finish();else token=requestAnimationFrame(sample);
      }catch(error){done=true;clearTimeout(timeout);reject(error);}};sample();
    })`);
    const [observed, acted] = await Promise.allSettled([sampled, action()]);
    if (observed.status === "rejected") throw observed.reason;
    const frames = observed.value;
    if (output) {
      await mkdir(output, { recursive: true });
      await writeFile(path.join(output, `${name}-frames.json`), JSON.stringify(frames, null, 2));
    }
    if (acted.status === "rejected") throw acted.reason;
    return frames;
  };

  try {
    await load(fixture);
    await check("Start and Apps held touch preview preserves the source and commits adjacent paired pages", async () => {
      const document = await state();
      await page.send("Emulation.setTouchEmulationEnabled", { enabled: true, maxTouchPoints: 1 });
      try {
        for (const [route, direction] of [["Apps", 1], ["Start", -1]]) {
          const source = direction > 0 ? "Start" : "Apps";
          const resting = await restingPage();
          const y = resting.y + Math.min(resting.height * 0.4, 180);
          const from = [resting.x + resting.width * (direction > 0 ? 0.78 : 0.22), y];
          const to = [from[0] - direction * resting.width * 0.42, y];
          const frames = await record(`metro-shell-paging-${route.toLowerCase()}`, async () => {
            await sendTouch("touchStart", from);
            for (let step = 1; step <= 6; step++) {
              await sendTouch("touchMove", [from[0] + (to[0] - from[0]) * step / 6, y]);
              await page.evaluate("new Promise(resolve=>requestAnimationFrame(()=>resolve(true)))");
            }
            await heldFrame(source, direction, resting);
            assert.deepEqual(await state(), document, "Held preview cannot persist state or activate a destination");
            await capture(`metro-shell-held-${route.toLowerCase()}`);
            await sendTouch("touchEnd");
          }, 2000);
          await isRoute(route);
          assertHeldShellPaging(frames, source, direction, resting);
          assertShellPaging(frames, route, direction, resting);
        }
      } finally {
        await sendTouch("touchCancel").catch(() => {});
        await page.send("Emulation.setTouchEmulationEnabled", { enabled: false });
      }
      assert.deepEqual(await state(), document, "Page manipulation cannot alter the saved phone document");
      await capture("metro-shell-paging-restored");
    });

    await check("live Start preview handoff preserves tile ink while later cadence updates still animate", async () => {
      const live = structuredClone(fixture);
      const people = live.tiles.find(tile => tile.app === "People" && !tile.target && !tile.children.length);
      assert(people && live.contacts.length > 1, "The live handoff fixture needs a People tile with multiple contacts");
      live.tiles = [{ ...people, size: "Wide", appearance: { ...people.appearance, live: true } }];
      live.settings.live_tiles = true;
      const tileFrames = view => view.elements.filter(element => element.name?.startsWith("LiveTileContent-"));
      const cells = view => view.elements.filter(element => element.name?.startsWith(`PeopleCell-${people.id}-`))
        .sort((a, b) => a.name.localeCompare(b.name)).map(element => element.text);
      let frozen = false;
      await page.send("Emulation.setTouchEmulationEnabled", { enabled: true, maxTouchPoints: 1 });
      try {
        const base = await page.evaluate(`(()=>{
          const seconds=Math.floor(Date.now()/60000)*60+20;
          globalThis.__phoneMotionClock={original:Date.now,seconds};
          Date.now=()=>globalThis.__phoneMotionClock.seconds*1000;
          phoneApp.tick(seconds);return seconds;
        })()`);
        frozen = true;
        await load(live);
        const before = await until(async () => {
          const view = await snapshot(), tiles = tileFrames(view);
          return tiles.length >= 6 && tiles.every(tile => tile.projection === null && tile.opacity === 1) &&
            cells(view).length === 6 ? cells(view) : undefined;
        }, "ordinary live Start entry completes its tile entrance");
        const document = await state();
        // Navigate to Apps through the real shell gesture before testing the
        // reverse live handoff. The wall clock is fixed; RAF remains real.
        let resting = await restingPage();
        let from = [resting.x + resting.width * 0.78, resting.y + 180];
        await sendTouch("touchStart", from);
        await sendTouch("touchMove", [from[0] - resting.width * 0.42, from[1]]);
        await sendTouch("touchEnd");
        await isRoute("Apps");
        resting = await restingPage();
        from = [resting.x + resting.width * 0.22, resting.y + 180];
        const frames = await record("metro-shell-live-start-handoff", async () => {
          await sendTouch("touchStart", from);
          await sendTouch("touchMove", [from[0] + resting.width * 0.42, from[1]]);
          await heldFrame("Apps", -1, resting);
          await capture("metro-shell-live-start-held");
          await sendTouch("touchEnd");
        }, 2000, true);
        assertHeldShellPaging(frames, "Apps", -1, resting);
        assertShellPaging(frames, "Start", -1, resting);
        assertLiveTileHandoff(frames);
        assert.deepEqual(cells(await snapshot()), before, "The same clock/content phase survives the bitmap-to-live handoff");
        await capture("metro-shell-live-start-restored");
        const cadence = await record("metro-shell-live-cadence", () => page.evaluate(
          `globalThis.__phoneMotionClock.seconds=${base + 4};phoneApp.tick(${base + 4})`), 1100, true);
        assert(cadence.some(frame => frame.tiles?.some(tile => tile.projection?.kind === "slide")),
          "A subsequent live content update retains its native tile animation");
        const after = cells(await snapshot());
        assert.equal(after.filter((value, index) => value !== before[index]).length, 1,
          "Only the next scheduled People cell changes");
        assert.deepEqual(await state(), document, "Handoff and cadence remain presentation-only");
      } finally {
        await sendTouch("touchCancel").catch(() => {});
        await page.send("Emulation.setTouchEmulationEnabled", { enabled: false });
        if (frozen) await page.evaluate(`(()=>{
          const clock=globalThis.__phoneMotionClock;
          Date.now=clock.original;delete globalThis.__phoneMotionClock;
          phoneApp.tick(Math.floor(Date.now()/1000));
        })()`);
        await load(fixture);
      }
    });

    await check("a rejected or canceled held shell pan returns adjacent pages and releases its preview", async () => {
      const document = await state();
      await page.send("Emulation.setTouchEmulationEnabled", { enabled: true, maxTouchPoints: 1 });
      try {
        for (const kind of ["return", "cancel"]) {
          const resting = await restingPage();
          const from = [resting.x + resting.width * 0.78, resting.y + 180];
          // 16 px resolves the horizontal axis without reaching either the
          // distance or minimum-flick threshold. Holding also drains velocity.
          const to = [from[0] - (kind === "return" ? 16 : resting.width * 0.42), from[1]];
          const frames = await record(`metro-shell-held-${kind}`, async () => {
            await sendTouch("touchStart", from);
            await sendTouch("touchMove", to);
            await heldFrame("Start", 1, resting);
            await sendTouch(kind === "return" ? "touchEnd" : "touchCancel");
          });
          assertHeldShellPaging(frames, "Start", 1, resting);
          if (kind === "return") assertShellReturn(frames, "Start", 1, resting);
          else {
            assert(frames.every(frame => sameRoute(frame.route, "Start")), "Pointer cancellation cannot navigate");
            const acquired = frames.findIndex(hasNeighbor);
            const released = frames.findIndex((frame, index) => index > acquired && !hasNeighbor(frame));
            assert(released > acquired, "Cancel must retire an actually captured held image");
            assert(frames.slice(released).every(frame => !hasNeighbor(frame) && !frame.staged),
              "A pending/stale callback cannot resurrect the canceled preview");
            assertRestored(frames.at(-1), "Start", resting);
          }
        }
      } finally {
        await sendTouch("touchCancel").catch(() => {});
        await page.send("Emulation.setTouchEmulationEnabled", { enabled: false });
      }
      assert.deepEqual(await state(), document);
    });

    await check("locking during a held shell pan drops the neighboring image and cancels navigation", async () => {
      const document = await state();
      const resting = await restingPage();
      const from = [resting.x + resting.width * 0.78, resting.y + 180];
      const to = [from[0] - resting.width * 0.42, from[1]];
      await page.send("Emulation.setTouchEmulationEnabled", { enabled: true, maxTouchPoints: 1 });
      try {
        const frames = await record("metro-shell-held-lock", async () => {
          await sendTouch("touchStart", from);
          await sendTouch("touchMove", to);
          await heldFrame("Start", 1, resting);
          await page.evaluate("phoneApp.lockPhone()");
          await sendTouch("touchEnd");
        });
        assertHeldShellPaging(frames, "Start", 1, resting);
        assert(frames.every(frame => sameRoute(frame.route, "Start")), "A release after lock cannot commit the old contact");
        const covered = frames.filter(frame => frame.locked);
        assert(covered.length >= 2, "The lock screen must render after the held phase");
        for (const frame of covered) {
          assert(!frame.page && !hasNeighbor(frame) && !hasImage(frame) && !frame.staged,
            "Privacy takeover drops every private image and temporary page tree");
          assert.equal(frame.neighbor?.opacity, 0);
          assert.equal(frame.neighbor?.projection, null);
        }
        await capture("metro-shell-held-lock-cancelled");
      } finally {
        await sendTouch("touchCancel").catch(() => {});
        await page.send("Emulation.setTouchEmulationEnabled", { enabled: false });
      }
      await clickLabel("Unlock");
      await isRoute("Start");
      await restingPage();
      assert.deepEqual(await state(), document);
    });

    await check("native outgoing page image retires before the incoming page and blocks hidden input", async () => {
      const row = await settingRow();
      const resting = await restingPage();
      const document = await state();
      const forward = await record("metro-navigation-forward", async () => {
        await rawClick(row);
        const frame = await outgoingFrame(true);
        assert(frame.probe && !frame.probe.enabled && !frame.probe.hit,
          "The real destination switch must exist without accepting input");
        await rawClick(frame.probe);
      });
      assertNavigationChoreography(forward, detailRoute, false, resting);
      assert.deepEqual(await state(), document, "A click behind the retiring image must not toggle the destination switch");
      const back = await control(element => element.name === "HardwareBack", "Hardware Back");
      const backward = await record("metro-navigation-backward", () => rawClick(back));
      assertNavigationChoreography(backward, settingsRoute, true, resting);
      assert.deepEqual(await state(), document, "Forward and Back choreography must preserve the saved document");
      await capture("metro-navigation-restored");
    });

    await check("rapid hardware Back cancels the retiring image without resurrecting the old visit", async () => {
      const row = await settingRow();
      const resting = await restingPage();
      const back = await control(element => element.name === "HardwareBack", "Hardware Back");
      const document = await state();
      const frames = await record("metro-navigation-interrupted", async () => {
        await rawClick(row);
        await outgoingFrame();
        await rawClick(back);
      });
      const detail = frames.findIndex(frame => sameRoute(frame.route, detailRoute) && hasImage(frame));
      assert(detail >= 0, `The interruption must occur during an actual outgoing image: ${diagnostic(frames)}`);
      const returned = frames.findIndex((frame, index) => index > detail && sameRoute(frame.route, settingsRoute));
      assert(returned > detail, "Hardware Back must immediately restore the Settings route");
      for (const frame of frames.slice(returned))
        assert(!hasImage(frame), "A canceled or stale completion must not reattach an outgoing image");
      assert(frames.slice(returned).some(frame => frame.page?.projection?.angleY > 1),
        "The superseding Back visit uses its own incoming rotation");
      assertRestored(frames.at(-1), settingsRoute, resting);
      assert.deepEqual(await state(), document);
    });

    await check("locking during navigation releases the private outgoing image", async () => {
      const row = await settingRow();
      const document = await state();
      const frames = await record("metro-navigation-lock", async () => {
        await rawClick(row);
        await outgoingFrame();
        // Production host lock callback works in both handset and desktop
        // viewports, like the automatic-lock regression elsewhere in the suite.
        await page.evaluate("phoneApp.lockPhone()");
      });
      assert(frames.some(hasImage), "Lock must interrupt a captured page, not an idle route");
      const covered = frames.filter(frame => frame.locked);
      assert(covered.length >= 2, "The native lock screen must remain mounted");
      for (const frame of covered) {
        assert(!hasImage(frame), "No private image may remain behind the lock screen");
        assert(!frame.page, "The live private page must be hidden from rendering and input");
        assert.equal(frame.outgoing?.opacity, 0);
        assert.equal(frame.outgoing?.projection, null);
      }
      await capture("metro-navigation-lock-cancelled");
      await clickLabel("Unlock");
      await isRoute(detailRoute);
      await restingPage();
      assert.deepEqual(await state(), document, "Lock cancellation must preserve the phone document");
    });

    for (const preference of ["reduced_motion", "battery_saver"]) {
      await check(`${preference} suppresses outgoing images and incoming projections`, async () => {
        const quiet = structuredClone(fixture);
        quiet.settings[preference] = true;
        await load(quiet);
        const row = await settingRow();
        const resting = await restingPage();
        const frames = await record(`metro-navigation-${preference}`, () => rawClick(row));
        assertUnprojectedFrames(frames, detailRoute, preference);
        for (const frame of frames) {
          assert(!hasImage(frame), `${preference} must not allocate a retiring image`);
          assert.equal(frame.outgoing?.opacity, 0);
          assert.equal(frame.outgoing?.projection, null);
        }
        assertRestored(frames.at(-1), detailRoute, resting);
        await load(quiet);
        const shellResting = await restingPage();
        const from = [shellResting.x + shellResting.width * 0.78, shellResting.y + 180];
        const to = [from[0] - shellResting.width * 0.42, from[1]];
        await page.send("Emulation.setTouchEmulationEnabled", { enabled: true, maxTouchPoints: 1 });
        try {
          const shell = await record(`metro-shell-held-${preference}`, async () => {
            await sendTouch("touchStart", from);
            await sendTouch("touchMove", to);
            await page.evaluate("new Promise(resolve=>requestAnimationFrame(()=>requestAnimationFrame(()=>resolve(true))))");
            await isRoute("Start");
            await sendTouch("touchEnd");
          });
          assertUnprojectedFrames(shell, "Apps", preference);
          for (const frame of shell)
            assert(!hasNeighbor(frame) && !hasImage(frame) && !frame.staged,
              `${preference} suppresses held and committed image allocation`);
          assertRestored(shell.at(-1), "Apps", shellResting);
        } finally {
          await sendTouch("touchCancel").catch(() => {});
          await page.send("Emulation.setTouchEmulationEnabled", { enabled: false });
        }
      });
    }
  } catch (error) {
    await capture("metro-navigation-failure").catch(() => {});
    throw error;
  } finally {
    if ((await snapshot()).elements.some(element => element.name === "LockScreen"))
      await clickLabel("Unlock");
    await load(original);
  }
}
