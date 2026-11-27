import assert from "node:assert/strict";
import { sleep } from "../FerroUI/scripts/browser/harness.mjs";
import { buttonLabelMatches, inputGeometryReady } from "./ui-input-geometry.mjs";

function motionDiagnostic(frames) {
  const firstProjected = frames.findIndex(frame => frame.page && frame.page.projection !== null);
  const indices = new Set([0, frames.length - 1, firstProjected]);
  for (let index = 1; index < 10; index++)
    indices.add(Math.floor((frames.length - 1) * index / 10));
  return JSON.stringify({
    sampledFrames: frames.length,
    unmountedFrames: frames.filter(frame => !frame.page).length,
    frames: [...indices].filter(index => index >= 0 && index < frames.length)
      .sort((a, b) => a - b).map(index => frames[index]),
  });
}

// A native route replacement can be observed between detaching the old page
// and laying out the new one. No projection exists on that unmounted frame.
// Require actual destination frames and a mounted final page, while rejecting
// every projection observed on either side of the route replacement.
export function assertUnprojectedFrames(frames, expectedRoute, scenario = "Reduced-motion navigation") {
  const diagnostic = motionDiagnostic(frames);
  const observed = frames.filter(frame => frame.page);
  assert(observed.every(frame => frame.page.projection === null),
    `${scenario} rendered a decorative projection: ${diagnostic}`);
  const expected = JSON.stringify(expectedRoute);
  const destination = observed.filter(frame => JSON.stringify(frame.route) === expected);
  assert(destination.length >= 2,
    `${scenario} destination must render at least two observable frames: ${diagnostic}`);
  const last = frames.at(-1);
  assert(last?.page && JSON.stringify(last.route) === expected && last.page.projection === null,
    `${scenario} must finish on its mounted, untransformed destination: ${diagnostic}`);
}

// Isolated fixtures and real browser pointer, touch and keyboard input. The
// exported document and native inspection API are observation surfaces only;
// no control value, selection or callback is set through an inspection hook.
export async function verifyMetroControls(ctx) {
  const { page, check, state, snapshot, until, capture, button, centre, control,
    clickName, clickLabel, reveal, isRoute, textPresent } = ctx;
  const original = await state();
  const viewport = await snapshot();
  const fixture = structuredClone(original);
  Object.assign(fixture.settings, {
    lock_timeout: 0, reduced_motion: false, battery_saver: false, live_tiles: false,
    high_contrast: false, text_scale: 1, dark: true, time_24h: true,
    sounds: false, ringer_volume: 0, vibration: false,
  });
  fixture.events = [];
  fixture.disabled_extensions = [];
  fixture.installed = [...new Set([...fixture.installed, "Alarms", "Settings", "Games"])];
  fixture.alarms = [{ id: fixture.next_id++, hour: 7, minute: 0,
    label: "Metro control fixture", enabled: false, weekdays: [], snooze_minutes: 10 }];

  const named = (view, name) => view.elements.find(element => element.name === name);
  const frame = () => page.evaluate(
    "new Promise(resolve=>requestAnimationFrame(()=>requestAnimationFrame(()=>resolve(true))))");
  const near = (actual, expected, message, tolerance = 0.6) =>
    assert(Math.abs(actual - expected) <= tolerance, `${message}: ${actual} versus ${expected}`);
  const inside = (child, parent) => {
    const [x, y] = centre(child);
    return x >= parent.x && x <= parent.x + parent.width &&
      y >= parent.y && y <= parent.y + parent.height;
  };
  const scrollMatch = element => /ScrollViewer$/.test(element.type) &&
    element.width > 220 && element.height > 160;
  const load = async data => {
    await page.evaluate(`phoneApp.importData(${JSON.stringify(JSON.stringify(data))})`);
    await isRoute("Start");
    await frame();
    const restored = await state();
    for (const preference of ["reduced_motion", "battery_saver"])
      assert.equal(restored.settings[preference], data.settings[preference],
        `Fixture import must preserve ${preference}`);
  };
  const app = async name => {
    await page.evaluate(`phoneApp.openApp(${JSON.stringify(name)})`);
    await isRoute({ App: name });
  };
  const stable = async (match, label, requireHit = true) => {
    let previous;
    let observations = 0;
    return until(async () => {
      const view = await snapshot();
      if (!inputGeometryReady(view)) {
        previous = undefined;
        observations = 0;
        return;
      }
      const matches = view.elements.filter(element => (!requireHit || element.hit) && match(element));
      assert(matches.length <= 1, `Ambiguous native target: ${label}`);
      const current = matches[0];
      if (!current) return;
      const geometry = [current.x, current.y, current.width, current.height];
      observations = previous && geometry.every((value, index) =>
        Math.abs(value - previous[index]) < 0.1) ? observations + 1 : 0;
      previous = geometry;
      return observations >= 2 ? current : undefined;
    }, `${label} reaches stable native geometry`);
  };
  const clickStable = async name => page.click(...centre(
    await stable(element => element.name === name, name)));
  const settingRow = async name => {
    await app("Settings");
    // The native Pivot's presentation header carries its label as content;
    // the PivotItem owns the automation name. Reduced motion uses the shared
    // labelled Button instead. Click the actual stable header in either case.
    const system = await stable(element => buttonLabelMatches(element, "system"), "system pivot");
    await page.click(...centre(system));
    await page.wheel(...centre(await control(element => element.name === "SettingsSystemScroll", "Settings system pane")), 0, -100000);
    return reveal(element => button(element) && element.automationName === name,
      name, element => element.name === "SettingsSystemScroll", "Settings system pane");
  };
  const setting = async name => {
    const row = await settingRow(name);
    await page.click(...centre(row));
    await isRoute({ Setting: name });
  };
  const restingPage = () => until(async () => {
    const page = named(await snapshot(), "PhonePage");
    return page?.projection === null && page.opacity === 1 ? page : undefined;
  }, "Native PhonePage projection has settled and detached");
  // Register a frame observer before real input, so CDP round trips and the
  // click helper's normal delay cannot hide a short entrance. This only reads
  // native geometry/projection; it does not seek or control the animation.
  const motionFrames = async action => {
    const sampled = page.evaluate(`new Promise((resolve,reject)=>{
      const frames=[];const start=performance.now();let frame,done=false;
      const preferences=()=>{const settings=JSON.parse(phoneApp.exportData()).settings;
        return {reduced_motion:settings.reduced_motion,battery_saver:settings.battery_saver};};
      const finish=()=>{if(done)return;done=true;cancelAnimationFrame(frame);clearTimeout(timeout);
        if(frames.length)frames.at(-1).preferences=preferences();resolve(frames);};
      const timeout=setTimeout(finish,1800);
      const sample=()=>{try{
        const view=JSON.parse(phoneApp.inspectView());
        const page=view.elements.find(element=>element.name==='PhonePage');
        frames.push({elapsed:performance.now()-start,route:JSON.parse(phoneApp.currentRoute()),
          preferences:frames.length===0?preferences():undefined,
          page:page?{x:page.x,y:page.y,width:page.width,height:page.height,projection:page.projection}:null});
        if(performance.now()-start>=850)finish();else frame=requestAnimationFrame(sample);
      }catch(error){done=true;clearTimeout(timeout);reject(error);}};
      sample();
    })`);
    const [frames] = await Promise.all([sampled, action()]);
    return frames;
  };
  const rawClick = async element => {
    const point = centre(element);
    await page.mouseMove(...point);
    await page.mouseDown(...point);
    await page.mouseUp(...point);
  };
  const assertTurnstile = (frames, route, backward, resting) => {
    const visits = frames.filter(frame => JSON.stringify(frame.route) === JSON.stringify(route) && frame.page);
    const poses = visits.map(frame => frame.page).filter(page =>
      page.projection?.kind === "turnstile" && Math.abs(page.projection.angleY) > 0.01);
    const diagnostic = motionDiagnostic(frames);
    assert(poses.some(page => page.projection.angleY * (backward ? 1 : -1) > 1),
      `${backward ? "Back" : "Forward"} navigation must render its signed native turnstile: ${diagnostic}`);
    let previous = backward ? 50 : 80;
    for (const page of poses) {
      const angle = page.projection.angleY * (backward ? 1 : -1);
      assert(angle > 0 && angle <= previous + 0.2, "Turnstile progresses toward zero without reversing");
      previous = angle;
      near(page.projection.centerX, -page.width / 2, "Native turnstile uses the left-edge hinge", 0.001);
      near(page.projection.depth, page.width * 1000 / 480, "Native turnstile uses the scaled reference camera", 0.001);
    }
    const last = visits.at(-1)?.page;
    assert(last && last.projection === null, `Turnstile releases its transform after settling: ${diagnostic}`);
    for (const property of ["x", "y", "width", "height"])
      near(last[property], resting[property], `Turnstile restores resting PhonePage ${property}`);
  };
  const persisted = () => page.evaluate("JSON.parse(localStorage.getItem('ferroui.windows-phone.v1'))");
  const sendTouch = (type, x, y) => page.send("Input.dispatchTouchEvent", {
    type, touchPoints: type === "touchEnd" || type === "touchCancel" ? [] :
      [{ x, y, id: 0, radiusX: 2, radiusY: 2, force: 1 }],
  });
  const touchDrag = async (from, to, midpoint) => {
    await sendTouch("touchStart", ...from);
    let released = false;
    try {
      await sleep(35);
      for (let step = 1; step <= 8; step++) {
        const amount = step / 8;
        await sendTouch("touchMove", from[0] + (to[0] - from[0]) * amount,
          from[1] + (to[1] - from[1]) * amount);
        await sleep(25);
        if (step === 4 && midpoint) await midpoint();
      }
      await sendTouch("touchEnd", ...to);
      released = true;
      await frame();
    } finally {
      if (!released) await sendTouch("touchCancel").catch(() => {});
    }
  };
  const switchParts = async expected => {
    const toggle = await stable(element => /ToggleSwitch$/.test(element.type) &&
      element.automationName === `Vibrate, ${expected ? "on" : "off"}`,
    "native Vibrate switch");
    const view = await snapshot();
    const part = name => {
      const parts = view.elements.filter(element => element.name === name && inside(element, toggle));
      assert.equal(parts.length, 1, `Vibrate has one native ${name}`);
      return parts[0];
    };
    assert.equal(toggle.checked, expected);
    return { toggle, thumb: part("PhoneSwitchThumb"), moving: part("PART_MovingKnobs"),
      travel: part("PART_SwitchKnob"), track: part("PhoneSwitchTrack") };
  };
  const selectedPivot = index => until(async () => {
    const pivot = named(await snapshot(), "GamesPivot");
    return pivot?.selectedIndex === index && /Pivot$/.test(pivot.type) ? pivot : undefined;
  }, `native Games pivot selection ${index}`);
  const swipePivot = async () => {
    const pivot = await stable(element => element.name === "GamesPivot", "Games pivot");
    const y = pivot.y + Math.max(120, pivot.height * 0.52);
    await touchDrag([pivot.x + pivot.width - 40, y], [pivot.x + 40, y]);
    await isRoute({ App: "Games" });
  };

  try {
    await load(fixture);
    await check("Metro application bar reveals labels without moving page content", async () => {
      await app("Alarms");
      await stable(element => element.name === "AppBarMore", "Alarms ellipsis");
      const before = await snapshot();
      const bar = named(before, "PhoneApplicationBar");
      const more = named(before, "AppBarMore");
      const circle = named(before, "AppBarCircle-add");
      const commands = named(before, "AppBarPrimaryCommands");
      const action = named(before, "AppBarAction-add");
      const alarm = before.elements.find(element =>
        element.automationName === "Edit alarm 07:00 Metro control fixture");
      assert(bar && more && circle && commands && action && alarm, "The native app bar and alarm row must be mounted");
      near(commands.width, 76, "The single command retains its complete equal-width slot");
      near(commands.x + commands.width / 2, bar.x + bar.width / 2, "Primary commands stay centered on the phone");
      near(action.width, commands.width, "The full command slot accepts input");
      assert(!named(before, "AppBarLabel-add"), "Collapsed application bar hides icon labels");
      assert(!named(before, "AppBarCircle-more"), "The ellipsis is not a circular primary action");
      assert(circle.x + circle.width <= more.x, "The ellipsis follows the primary action slots");
      near(more.x + more.width, bar.x + bar.width, "Ellipsis occupies the right edge");
      const document = await state();
      await clickStable("AppBarMore");
      await until(async () => !!named(await snapshot(), "AppBarLabel-add"), "Expanded add label");
      const expanded = await snapshot();
      for (const property of ["x", "width"])
        near(named(expanded, "AppBarAction-add")[property], action[property],
          `Showing captions preserves the command's horizontal ${property}`);
      const expandedRow = expanded.elements.find(element => element.automationName === alarm.automationName);
      assert(expandedRow, "Opening the application bar retains the alarm row");
      for (const property of ["x", "y", "width", "height"])
        near(expandedRow[property], alarm[property], `Expansion preserves alarm ${property}`);
      assert(named(expanded, "PhoneApplicationBar").height > bar.height,
        "Labels expand upward over the existing page");
      assert(!named(expanded, "AppBarMenuScroll"), "A label-only bar needs no fabricated menu commands");
      await capture("metro-app-bar-labels-overlay");
      await clickName("HardwareBack");
      await isRoute({ App: "Alarms" });
      await until(async () => !named(await snapshot(), "AppBarLabel-add"), "Back collapses the bar first");
      await clickStable("AppBarMore");
      await clickName("AppBarDismiss");
      await isRoute({ App: "Alarms" });
      await until(async () => !named(await snapshot(), "AppBarLabel-add"), "Outside tap collapses the bar");
      assert.deepEqual(await state(), document, "Transient bar interaction must not mutate saved data");
      await capture("metro-app-bar-collapsed");
    });

    await check("native page entrances survive route cleanup and reverse on Back", async () => {
      const row = await settingRow("ringtones+sounds");
      const resting = await restingPage();
      const document = await state();
      const forward = await motionFrames(() => rawClick(row));
      await isRoute({ Setting: "ringtones+sounds" });
      assertTurnstile(forward, { Setting: "ringtones+sounds" }, false, resting);

      const back = await control(element => element.name === "HardwareBack", "Back hardware key");
      const backward = await motionFrames(() => rawClick(back));
      await isRoute({ App: "Settings" });
      assertTurnstile(backward, { App: "Settings" }, true, resting);

      const reflow = await motionFrames(async () => {
        await page.resize(viewport.width + 1, viewport.height);
        await frame();
        await page.resize(viewport.width, viewport.height);
      });
      assertUnprojectedFrames(reflow, { App: "Settings" }, "Same-route native resize");
      await restingPage();
      assert.deepEqual(await state(), document, "Page motion and reflow do not change saved data");
      await capture("metro-turnstile-navigation-settled");

      const reduced = structuredClone(fixture);
      reduced.settings.reduced_motion = true;
      await load(reduced);
      const reducedRow = await settingRow("ringtones+sounds");
      await restingPage();
      const still = await motionFrames(() => rawClick(reducedRow));
      await isRoute({ Setting: "ringtones+sounds" });
      assertUnprojectedFrames(still, { Setting: "ringtones+sounds" });
      await load(fixture);
    });

    await check("native Metro switch thumb follows touch and persists on release", async () => {
      await setting("ringtones+sounds");
      await switchParts(false);
      const document = await state();
      await page.send("Emulation.setTouchEmulationEnabled", { enabled: true, maxTouchPoints: 5 });
      for (const value of [true, false]) {
        const parts = await switchParts(!value);
        const from = centre(parts.thumb);
        const distance = parts.travel.width * (value ? 1 : -1);
        await touchDrag(from, [from[0] + distance, from[1]], async () => {
          const view = await snapshot();
          const moved = view.elements.find(element => element.name === "PhoneSwitchThumb" &&
            inside(element, parts.toggle));
          assert(moved, "The moving native thumb remains in its switch");
          assert((moved.x - parts.thumb.x) * (value ? 1 : -1) > 8,
            "The thumb must move during the drag, not merely toggle on a final tap");
          assert.equal((await state()).settings.vibration, !value,
            "A drag publishes its new setting only when released");
        });
        await until(async () => (await state()).settings.vibration === value,
          `Touch release sets vibration ${value}`);
        await until(async () => (await persisted()).settings.vibration === value,
          `Vibration ${value} is persisted`);
        await switchParts(value);
        await isRoute({ Setting: "ringtones+sounds" });
      }
      assert.deepEqual(await state(), document, "Round-trip knob drags change no unrelated data");
      await capture("metro-native-switch-touch-restored");
      await page.send("Emulation.setTouchEmulationEnabled", { enabled: false });
    });

    await check("native Metro pivot supports touch wrapping, headers and keyboard", async () => {
      await app("Games");
      await selectedPivot(0);
      await stable(element => element.name === "AppBarMore", "Games application bar");
      const document = await state();
      const storage = await persisted();
      await page.send("Emulation.setTouchEmulationEnabled", { enabled: true, maxTouchPoints: 5 });
      await swipePivot();
      await selectedPivot(1);
      await textPresent("local player");
      await stable(element => element.text === "local player", "Player pane", false);
      await capture("metro-pivot-native-player");
      await swipePivot();
      await selectedPivot(0);
      await stable(element => element.name === "GameLaunch-org.ferroui.minesweeper", "Collection pane");
      await page.send("Emulation.setTouchEmulationEnabled", { enabled: false });
      // Native Pivot exposes its automation name on PivotItem; its separate
      // presentation header is a hit-testable PivotHeaderItem with text. Wait
      // for that header's rendered position after wrapping, then send real input.
      await page.click(...centre(await stable(element =>
        /PivotHeaderItem$/.test(element.type) && element.text === "player",
      "Native player pivot header")));
      await selectedPivot(1);
      await stable(element => element.text === "local player", "Header-selected player pane", false);
      await control(element => element.name === "HeaderClipper" && element.focused,
        "Native pivot header receives keyboard focus");
      await page.press("ArrowLeft");
      await selectedPivot(0);
      await stable(element => element.name === "GameLaunch-org.ferroui.minesweeper", "Keyboard-selected collection pane");
      await page.press("ArrowRight");
      await selectedPivot(1);
      await isRoute({ App: "Games" });
      assert.deepEqual(await state(), document, "Pivot input changes transient selection only");
      assert.deepEqual(await persisted(), storage, "Pivot input does not write the phone document");
    });

    await check("Metro controls fit 320-pixel light and dark phone surfaces", async () => {
      await page.resize(320, 740);
      await until(async () => {
        const view = await snapshot();
        return view.width === 320 && view.height === 740;
      }, "320-pixel native viewport");
      for (const dark of [true, false]) {
        const compact = structuredClone(fixture);
        compact.settings.dark = dark;
        await load(compact);
        await setting("ringtones+sounds");
        const parts = await switchParts(false);
        const screen = named(await snapshot(), "PhoneScreen");
        assert(parts.toggle.x >= screen.x && parts.toggle.x + parts.toggle.width <= screen.x + screen.width + 0.6,
          "Native switch must remain inside the compact phone surface");
        assert(inside(parts.thumb, screen) && inside(parts.track, screen), "Track and thumb remain reachable");
        await capture(`metro-controls-320-${dark ? "dark" : "light"}`);
      }
    });

    await check("reduced motion keeps phone pivot headings and content usable", async () => {
      const reduced = structuredClone(fixture);
      reduced.settings.reduced_motion = true;
      await load(reduced);
      await app("Games");
      const document = await state();
      const view = await snapshot();
      assert(!view.elements.some(element => element.name === "GamesPivot" && /Pivot$/.test(element.type)),
        "Reduced motion uses the static phone pivot surface");
      await clickLabel("player");
      await textPresent("local player");
      await capture("metro-pivot-reduced-motion-320");
      await clickLabel("collection");
      await control(element => button(element) && element.name === "GameLaunch-org.ferroui.minesweeper",
        "Collection remains reachable without decorative motion");
      assert.deepEqual(await state(), document);
    });
  } catch (error) {
    await capture("metro-controls-failure").catch(() => {});
    throw error;
  } finally {
    await sendTouch("touchCancel").catch(() => {});
    await page.send("Emulation.setTouchEmulationEnabled", { enabled: false });
    await page.resize(viewport.width, viewport.height);
    await until(async () => {
      const view = await snapshot();
      return view.width === viewport.width && view.height === viewport.height;
    }, "Original native viewport restored");
    await load(original);
  }
}
