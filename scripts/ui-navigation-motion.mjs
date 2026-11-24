import assert from "node:assert/strict";
import { mkdir, writeFile } from "node:fs/promises";
import path from "node:path";
import { assertUnprojectedFrames } from "./ui-metro-controls.mjs";

const sameRoute = (actual, expected) => JSON.stringify(actual) === JSON.stringify(expected);
const hasImage = frame => frame.image?.imageSource != null;
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
  const scrollMatch = element => /ScrollViewer$/.test(element.type) && element.width > 220 && element.height > 160;
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
    return page?.projection === null && page.opacity === 1 && page.enabled && page.hitTestVisible &&
      outgoing?.opacity === 0 && outgoing.projection === null &&
      !named(view, "PhoneOutgoingImage")?.imageSource ? page : undefined;
  }, "Both native page phases release their transforms and image");
  const settingRow = async () => {
    await page.evaluate('phoneApp.openApp("Settings")');
    await isRoute(settingsRoute);
    await restingPage();
    await clickLabel("system");
    await page.wheel(...centre(await control(scrollMatch, "Settings scroll viewer")), 0, -100000);
    return reveal(element => button(element) && element.automationName === "ringtones+sounds",
      "ringtones+sounds", scrollMatch, "Settings scroll viewer");
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
  const record = async (name, action) => {
    // Install before pointer input. This reads actual completed compositor
    // frames; no clock seeking, native property writes or artificial events.
    const sampled = page.evaluate(`new Promise((resolve,reject)=>{
      const frames=[],started=performance.now();let token,done=false;
      const finish=()=>{if(done)return;done=true;cancelAnimationFrame(token);clearTimeout(timeout);resolve(frames);};
      const timeout=setTimeout(finish,2000);
      const sample=()=>{try{
        const view=JSON.parse(phoneApp.inspectView());
        const find=name=>view.elements.find(element=>element.name===name)??null;
        frames.push({elapsed:performance.now()-started,route:JSON.parse(phoneApp.currentRoute()),
          page:find('PhonePage'),outgoing:find('PhoneOutgoingPage'),image:find('PhoneOutgoingImage'),
          locked:!!find('LockScreen')});
        if(performance.now()-started>=1100)finish();else token=requestAnimationFrame(sample);
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
      });
    }
  } catch (error) {
    await capture("metro-navigation-failure").catch(() => {});
    throw error;
  } finally {
    await load(original);
  }
}
