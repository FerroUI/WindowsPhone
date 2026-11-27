import assert from "node:assert/strict";
import { mkdir, writeFile } from "node:fs/promises";
import path from "node:path";
import { open, sleep, colours } from "../FerroUI/scripts/browser/harness.mjs";
import { verifyOfflineStartup } from "./ui-offline.mjs";
import { verifySystemRefinements } from "./ui-system-refinements.mjs";
import { verifyOfficeRefinements } from "./ui-office-refinements.mjs";
import { verifyBrowserRefinements } from "./ui-browser-refinements.mjs";
import { verifyLiveTiles } from "./ui-live-tiles.mjs";
import { verifyStartCustomization } from "./ui-start-customization.mjs";
import { verifyLockscreenDraft } from "./ui-lockscreen.mjs";
import { verifyLiveFolder } from "./ui-folders.mjs";
import { verifyCalendarRefinements } from "./ui-calendar-refinements.mjs";
import { verifyCalendarTasks } from "./ui-calendar-tasks.mjs";
import { verifyAlarmRefinements } from "./ui-alarm-refinements.mjs";
import { verifyShellParity } from "./ui-shell-parity.mjs";
import { verifyPimRefinements } from "./ui-pim-refinements.mjs";
import { verifyHostRefinements } from "./ui-host-refinements.mjs";
import { verifySdkGames } from "./ui-sdk-games.mjs";
import { verifyOfficeExpansion } from "./ui-office-expanded.mjs";
import { verifyMediaLibraries } from "./ui-media-libraries.mjs";
import { verifyConnectedApps } from "./ui-connected-apps.mjs";
import { verifyMapsExpansion } from "./ui-maps-expanded.mjs";
import { installMapTileFixture } from "./ui-map-fixture.mjs";
import { verifyPlatformRefinements } from "./ui-platform-refinements.mjs";
import { verifyMetroControls } from "./ui-metro-controls.mjs";
import { verifyNavigationMotion } from "./ui-navigation-motion.mjs";
import { verifyPhoneControlStates } from "./ui-phone-control-states.mjs";
import { verifyPhonePickers } from "./ui-phone-pickers.mjs";
import { verifyOutlookDrafts } from "./ui-outlook-drafts.mjs";
import { verifyPhoneCalls } from "./ui-phone-calls.mjs";
import { verifyHubPivots } from "./ui-hub-pivots.mjs";
import { verifyMetroProgress } from "./ui-metro-progress.mjs";
import { verifyNotificationGroups } from "./ui-notification-groups.mjs";
import { inputGeometryReady, inputTarget, stableInputTarget } from "./ui-input-geometry.mjs";

// Input goes through Chromium and FerroUI's retained native control tree.
// Exported state is an observation surface. Gallery navigation and controlled
// local photo fixtures use host APIs; no external service is contacted.
const site = path.resolve(process.argv[2] || "artifacts/site");
const output = path.resolve(process.argv[3] || "artifacts/ui-smoke");
const started = Date.now();
// The settled-input suite completed 88 checks before its former 360s budget
// expired during the final DPR checks. Allow 8 minutes for the full workload
// and CI variation without extending any individual UI wait.
const deadline = started + 480000;
const results = { site, viewport: [1280, 936], checks: [], captures: [] };
const apps = [
  ["Phone", "Phone"], ["Messaging", "Messaging"], ["People", "People"],
  ["Outlook", "Outlook"], ["Calendar", "Calendar"], ["Photos", "Photos"],
  ["Camera", "Camera"], ["Internet Explorer", "InternetExplorer"],
  ["Music", "Music"], ["Video", "Video"], ["Games", "Games"],
  ["Store", "Store"], ["Settings", "Settings"], ["Alarms", "Alarms"],
  ["Calculator", "Calculator"], ["OneNote", "OneNote"], ["Maps", "Maps"],
  ["Cortana", "Cortana"], ["Weather", "Weather"], ["Files", "Files"],
  ["Office", "Office"], ["Battery Saver", "BatterySaver"], ["Data Sense", "DataSense"],
];
let page;
await mkdir(output, { recursive: true });

const snapshot = async () => JSON.parse(await page.evaluate("phoneApp.inspectView()"));
const state = async () => JSON.parse(await page.evaluate("phoneApp.exportData()"));
const route = async () => JSON.parse(await page.evaluate("phoneApp.currentRoute()"));
const healthy = () => {
  assert(Date.now() < deadline, "UI verification exhausted the 8-minute suite budget");
  assert.equal(page.errors.length, 0, page.errors.join("\n"));
};
const button = element => /(?:Button|ToggleSwitch|CheckBox|PivotHeaderItem)$/.test(element.type);
const visibleText = element => /(?:TextBlock|AccessText)$/.test(element.type);
const centre = element => [element.x + element.width / 2, element.y + element.height / 2];
const centreInside = (element, viewport) => {
  const [x, y] = centre(element);
  return x >= viewport.x && y >= viewport.y &&
    x < viewport.x + viewport.width && y < viewport.y + viewport.height;
};
const boundsInside = (element, viewport) => element.x >= viewport.x && element.y >= viewport.y &&
  element.x + element.width <= viewport.x + viewport.width &&
  element.y + element.height <= viewport.y + viewport.height;
const renderedFrame = () => page.evaluate(
  "new Promise(resolve=>requestAnimationFrame(()=>requestAnimationFrame(()=>resolve(true))))");

async function until(check, label, timeout = 10000) {
  const end = Math.min(Date.now() + timeout, deadline);
  let lastError;
  while (Date.now() < end) {
    healthy();
    try {
      const result = await check();
      if (result) return result;
    } catch (error) { lastError = error; }
    await sleep(75);
  }
  // The shared suite deadline can end the loop before this action's own
  // timeout. Report that budget exhaustion instead of a missing UI control.
  healthy();
  throw new Error(`Timed out: ${label}${lastError ? ` (${lastError.message})` : ""}`);
}

async function rawControl(match, label) {
  return until(async () => {
    const found = (await snapshot())?.elements.filter(element => element.hit && match(element)) || [];
    assert(found.length <= 1, `Ambiguous control: ${label} (${found.length})`);
    return found[0];
  }, label);
}

async function presence(match, label) {
  return until(async () => {
    const view = await snapshot();
    const screen = view?.elements.find(element => element.name === "PhoneScreen");
    if (!screen) return;
    const found = view.elements.filter(element => match(element) && centreInside(element, screen));
    assert(found.length <= 1, `Ambiguous visible content: ${label} (${found.length})`);
    return found[0];
  }, label);
}
const textPresent = (text, label = text) =>
  presence(element => visibleText(element) && element.text === text, label);

async function click(match, label) {
  await page.click(...centre(await inputControl(match, label)));
}
const inputControl = (match, label) => until(() => stableInputTarget({
  snapshot, frame: renderedFrame, match, label,
}), label);
// Normal callers use controls as pointer coordinates. Motion observers can
// explicitly request rawControl, presence or snapshot without waiting for rest.
const control = inputControl;
const clickName = name => click(element => element.name === name, name);
const clickLabel = async label => {
  const match=element=>button(element)&&element.automationName===label;
  const found=await until(async()=>{
    const view=await snapshot();
    if(!inputGeometryReady(view))return;
    if(inputTarget(view,match,label))return stableInputTarget({view,snapshot,frame:renderedFrame,match,label});
    const menu=view.elements.find(element=>element.name==='AppBarMenuScroll'&&element.height>0);
    if(menu){
      const target=view.elements.find(element=>button(element)&&element.automationName===label);
      const scroll=await inputControl(element=>element.name==='AppBarMenuScroll','application bar menu');
      await page.wheel(...centre(scroll),0,target&&centre(target)[1]<scroll.y?-220:220);await renderedFrame();
    }
    else {
      const strip=view.elements.find(element=>element.name==='PivotHeaderScroll'&&element.height>0);
      const target=view.elements.find(element=>button(element)&&element.automationName===label);
      if(strip&&target&&(centre(target)[0]<strip.x||centre(target)[0]>strip.x+strip.width)){
        const scroll=await inputControl(element=>element.name==='PivotHeaderScroll','pivot header strip');
        await page.wheel(...centre(scroll),centre(target)[0]<scroll.x?-220:220,0);await renderedFrame();
      }
    }
  },label);
  await page.click(...centre(found));
};
const clickText = text => click(element => visibleText(element) && element.text === text, text);
const isRoute = expected => until(async () => JSON.stringify(await route()) === JSON.stringify(expected),
  `route ${JSON.stringify(expected)}`);

async function reveal(match, label, scrollMatch, scrollLabel) {
  let attempts = 0;
  return until(async () => {
    const view = await snapshot();
    if (!inputGeometryReady(view)) return;
    if (inputTarget(view, match, label))
      return stableInputTarget({view, snapshot, frame: renderedFrame, match, label});
    assert(attempts < 14, `Could not reveal ${label} by native scrolling`);
    await page.wheel(...centre(await inputControl(scrollMatch, scrollLabel)), 0, 530);
    attempts++;
  }, label);
}

async function edit(name, text) {
  await clickName(name);
  await page.press("a", ["ctrl"]);
  if (text) {
    // Keep line breaks on the native keyboard path: a single browser text
    // insertion can normalize them before the framework receives the edit.
    for (const [index, line] of text.split("\n").entries()) {
      if (index) await page.press("Enter");
      if (line) await page.send("Input.insertText", { text: line });
    }
  } else await page.press("Backspace");
  await until(async () => (await snapshot()).elements.some(element =>
    element.name === name && element.text === text), `native text in ${name}`);
}

async function ready() {
  await until(() => page.evaluate(`!!globalThis.phoneApp?.inspectView &&
    !!document.querySelector("canvas") &&
    (!document.querySelector(".ferroui-splash") ||
      document.querySelector(".ferroui-splash").classList.contains("splash-close"))`),
  "first native frame", 60000);
  await control(element => element.name === "HardwareStart", "hardware navigation");
}

async function capture(name, {picker} = {}) {
  // A reference capture represents the settled page. Navigation motion has a
  // separate per-frame regression; do not accidentally archive a mid-turnstile
  // pose merely because a control became hittable before its entrance ended.
  await until(async () => {
    const view = await snapshot();
    const page = view?.elements.find(element => element.name === "PhonePage");
    const outgoing = view?.elements.find(element => element.name === "PhoneOutgoingPage");
    const image = view?.elements.find(element => element.name === "PhoneOutgoingImage");
    const surface = view?.elements.find(element => element.name === "PhonePickerSurface");
    return (!page || (page.projection === null && page.opacity === 1)) &&
      (!outgoing || (outgoing.projection === null && outgoing.opacity === 0)) &&
      !image?.imageSource && (!surface || surface.projection === null);
  }, `${name}: page entrance completed`);
  await sleep(150);
  healthy();
  const view = await snapshot();
  const currentRoute = await route();
  const image = await page.screenshot(path.join(output, `${name}.png`));
  const sampledColours = colours(image);
  // Retain diagnostics even when a geometry assertion fails.
  await writeFile(path.join(output, `${name}.json`), JSON.stringify({
    route: currentRoute, width: image.width, height: image.height, sampledColours, view,
  }, null, 2));
  const screens = view?.elements.filter(element => element.name === "PhoneScreen") || [];
  assert.equal(screens.length, 1, `${name}: one native phone surface must remain mounted`);
  const viewport = {x: 0, y: 0, width: view.width, height: view.height};
  assert(boundsInside(screens[0], viewport), `${name}: phone surface must fit the native viewport`);
  if (picker) {
    // Native picker popups belong to TopLevel's OverlayLayer. Their hit region
    // can cover PhoneScreen's center without being a descendant of that screen.
    // Only explicitly requested, interactive picker presenters satisfy this path.
    assert(["DatePickerPresenter", "TimePickerPresenter"].includes(picker));
    const presenters = view.elements.filter(element =>
      element.name === "PART_PickerPresenter" && element.type.endsWith(picker));
    assert.equal(presenters.length, 1, `${name}: expected ${picker} must be mounted`);
    const presenter = presenters[0];
    assert(presenter.hit && boundsInside(presenter, viewport),
      `${name}: native picker presenter must be hittable and fit the viewport`);
    for (const part of ["PART_AcceptButton", "PART_DismissButton"]) {
      const commands = view.elements.filter(element => element.name === part && button(element));
      assert.equal(commands.length, 1, `${name}: expected one ${part}`);
      assert(commands[0].hit && boundsInside(commands[0], presenter),
        `${name}: ${part} must be hittable inside its presenter`);
    }
  } else {
    assert(screens[0].hit, `${name}: phone surface missing from native hit testing`);
  }
  assert(sampledColours > 40, `${name}: empty rendered surface (${sampledColours} colours)`);
  results.captures.push({ name, route: currentRoute, width: image.width, height: image.height, sampledColours });
  return image;
}

async function verifyMobileDensity(name, density) {
  await sleep(150);
  const geometry = await page.evaluate(`(() => {
    const rectangle = element => {
      const r = element.getBoundingClientRect();
      return { x: r.x, y: r.y, width: r.width, height: r.height };
    };
    const canvas = document.querySelector("#out canvas");
    return {
      devicePixelRatio, visualScale: visualViewport?.scale ?? 1,
      host: rectangle(document.getElementById("out")),
      canvas: rectangle(canvas), backing: [canvas.width, canvas.height]
    };
  })()`);
  const view = await snapshot();
  geometry.nativeViewport = [view.width, view.height];
  geometry.phoneScreen = view.elements.find(element => element.name === "PhoneScreen");
  const report = path.join(output, `${name}-geometry.json`);
  await writeFile(report, JSON.stringify(geometry, null, 2));
  const image = await capture(name);
  assert.equal(geometry.devicePixelRatio, density, "The advertised browser DPR must match the test");
  assert.equal(geometry.visualScale, 1, "Page zoom must not conceal a density mismatch");
  const viewport = { x: 0, y: 0, width: 390, height: 844 };
  assert.deepEqual(geometry.host, viewport, "The host must fill the mobile CSS viewport");
  assert.deepEqual(geometry.canvas, viewport, "Canvas CSS bounds must fill the mobile viewport");
  assert.deepEqual(geometry.backing, [390 * density, 844 * density],
    "Canvas backing pixels must equal CSS size times the advertised DPR");
  assert.deepEqual(geometry.nativeViewport, [390, 844], "Native layout must use CSS dimensions");
  const screen = geometry.phoneScreen;
  assert.deepEqual({ x: screen?.x, y: screen?.y, width: screen?.width, height: screen?.height }, viewport,
    "PhoneScreen must fill the native mobile viewport");
  assert.deepEqual([image.width, image.height], [390 * density, 844 * density],
    "Screenshot pixels must match the mobile backing size");

  // Check pixels at expected CSS positions, independently of native inspection.
  // A half-size centered phone leaves all three bottom-edge icon regions empty.
  geometry.hardwareIconPixels = {};
  for (const [index, label] of ["Back", "Start", "Search"].entries()) {
    const centreX = 390 * (index + 0.5) / 3;
    let white = 0;
    for (let y = (844 - 47) * density; y < (844 - 7) * density; y++) {
      for (let x = Math.floor((centreX - 22) * density); x < (centreX + 22) * density; x++) {
        if (image.pixel(x, y).every(channel => channel >= 180)) white++;
      }
    }
    geometry.hardwareIconPixels[label] = white;
  }
  await writeFile(report, JSON.stringify(geometry, null, 2));
  for (const [label, pixels] of Object.entries(geometry.hardwareIconPixels)) {
    assert(pixels >= 20 * density * density, `${label} must render at the bottom of the CSS viewport`);
  }
  await clickName("HardwareSearch");
  await isRoute("Search");
  await clickName("HardwareBack");
  await isRoute("Start");
}

async function check(name, run) {
  const begin = Date.now();
  await run();
  healthy();
  const elapsedMs = Date.now() - begin;
  results.checks.push({ name, elapsedMs, passed: true });
  console.log(`ok  ${name} (${elapsedMs} ms)`);
}

async function home() {
  await clickName("HardwareStart");
  // Start may already be the current route. Let native input and its retained
  // overlay/layout changes commit before using that route as a completion cue.
  await renderedFrame();
  await isRoute("Start");
  await control(element => element.name === "StartScroll", "settled Start scroll viewer");
}

async function settledPage(label) {
  const ready = view => inputGeometryReady(view) &&
    view.elements.some(element => element.name === "PhonePage");
  return until(async () => {
    if (!ready(await snapshot())) return;
    await renderedFrame();
    const current = await snapshot();
    return ready(current) ? current : undefined;
  }, label);
}

async function openApps() {
  await home();
  for (let attempt = 0; attempt < 12; attempt++) {
    const view = await settledPage("Start entrance finishes before app-list navigation");
    const scroll = view.elements.find(element => element.name === "StartScroll" && element.hit);
    assert(scroll, "Start scroll viewer is not hit-testable");
    const allApps = view.elements.find(element => element.name === "AllApps" && element.hit &&
      boundsInside(element, scroll));
    if (allApps) {
      // Wheel dispatch and native rendering are separate schedules. Require
      // the actual All apps command (not the edit-mode Done command) to keep
      // its fully visible, hittable geometry across rendered frames.
      const settled = await until(async () => {
        const firstView = await snapshot();
        if (!inputGeometryReady(firstView)) return;
        const first = firstView.elements.find(element => element.name === "AllApps");
        await renderedFrame();
        const current = await snapshot();
        if (!inputGeometryReady(current)) return;
        const second = current.elements.find(element => element.name === "AllApps");
        const viewport = current.elements.find(element => element.name === "StartScroll");
        if (!first || !second || !viewport || !second.hit || !boundsInside(second, viewport)) return;
        assert.equal(second.automationName, "All apps", "Start must finish leaving tile-edit mode before navigation");
        if (!["x", "y", "width", "height"].every(property => first[property] === second[property])) return;
        return { first, second, viewport };
      }, "stable, fully visible All apps command");
      const [x, y] = centre(settled.second);
      let pressed;
      await page.mouseMove(x, y);
      await page.mouseDown(x, y);
      try {
        // Send one complete native click with a rendered pressed frame. There
        // is deliberately no second click if the navigation assertion fails.
        await renderedFrame();
        pressed = (await snapshot()).elements.find(element => element.name === "AllApps");
      } finally {
        await page.mouseUp(x, y);
      }
      try {
        await isRoute("Apps");
      } catch (error) {
        await writeFile(path.join(output, "app-list-input-failure.json"), JSON.stringify({
          settled, pointer: { x, y }, pressed, route: await route(), view: await snapshot(),
        }, null, 2));
        throw error;
      }
      await control(element => element.name === "AppSearch", "app search field");
      return;
    }
    await page.wheel(...centre(await control(element => element.name === "StartScroll", "Start scroll viewer")), 0, 550);
  }
  throw new Error("All apps did not become visible after scrolling Start");
}

async function launchFromSearch(label, id) {
  await openApps();
  await edit("AppSearch", label);
  await clickLabel(label);
  await isRoute({ App: id });
  await settledPage(`${label} entrance finishes before app content inspection`);
}

async function reload() {
  const previous = await page.evaluate("performance.timeOrigin");
  const result = await page.send("Page.reload", {});
  assert(!result?.error, JSON.stringify(result));
  await until(() => page.evaluate(`performance.timeOrigin !== ${JSON.stringify(previous)}`),
    "reload created a new document", 60000);
  await ready();
}

async function touchSwipe(from, to) {
  const send = (type, x, y) => page.send("Input.dispatchTouchEvent", {
    type,
    touchPoints: type === "touchEnd" ? [] : [{ x, y, id: 0, radiusX: 2, radiusY: 2, force: 1 }],
  });
  await send("touchStart", ...from);
  await sleep(45);
  for (let step = 1; step <= 8; step++) {
    const amount = step / 8;
    await send("touchMove", from[0] + (to[0] - from[0]) * amount,
      from[1] + (to[1] - from[1]) * amount);
    await sleep(25);
  }
  await send("touchEnd", ...to);
  await sleep(180);
}

try {
  page = await open(site, { query: "?nooffline=1", width: 1280, height: 936, network: true, navigate:false });
  await installMapTileFixture(page);
  await page.navigate();
  await ready();
  const initial = await state();
  initial.settings.lock_timeout = 0;
  // Scheduling is exercised with the alarm module's isolated clock fixtures.
  // Unrelated interaction checks must never depend on the runner's wall time.
  initial.alarms.forEach(alarm => { alarm.enabled = false; });
  initial.events.forEach(event => { event.reminder_minutes = null; });
  const originalPhotoIds = new Set(initial.photos.map(photo => photo.id));
  const photoFixtures = Array.from({ length: 48 }, (_, index) => ({
    id: initial.next_id++,
    name: `Smoke photo ${String(index + 1).padStart(2, "0")}`,
    source: ["mountains", "coast", "forest", "city"][index % 4],
    favorite: false,
  }));
  initial.photos.push(...photoFixtures);
  await page.evaluate(`phoneApp.importData(${JSON.stringify(JSON.stringify(initial))})`);
  const phoneTile = initial.tiles.find(tile => tile.app === "Phone" && !tile.children.length);
  assert(phoneTile, "The initial Start screen must contain the Phone tile");
  let resizedTileSize;
  let noteId;
  const noteTitle = "Native UI smoke note";
  const originalBody = "Created through the native form.\nPersistence is checked by reloading.";
  const editedBody = "Edited through the native text box.\nThis survives a complete reload.";

  await check("desktop Start renders", async () => {
    await isRoute("Start");
    await capture("01-start-desktop");
  });
  await check("native tile launch and Back", async () => {
    await clickName(`Tile-${phoneTile.id}`);
    await isRoute({ App: "Phone" });
    await textPresent("history", "phone history");
    await capture("02-phone-history");
    await clickName("HardwareBack");
    await isRoute("Start");
  });
  await check("native context menu resizes a tile", async () => {
    const tile = await control(element => element.name === `Tile-${phoneTile.id}`, "Phone tile");
    const [x, y] = centre(tile);
    await page.mouseMove(x, y);
    await page.send("Input.dispatchMouseEvent", {
      type: "mousePressed", x, y, button: "right", buttons: 2, clickCount: 1, pointerType: "mouse",
    });
    await page.send("Input.dispatchMouseEvent", {
      type: "mouseReleased", x, y, button: "right", buttons: 0, clickCount: 1, pointerType: "mouse",
    });
    await clickName(`Resize-${phoneTile.id}`);
    resizedTileSize = await until(async () => {
      const size = (await state()).tiles.find(tile => tile.id === phoneTile.id)?.size;
      return size && size !== phoneTile.size && size;
    }, "resized tile model");
    await clickName("HardwareBack");
    await isRoute("Start");
    await capture("03-start-resized");
  });
  await check("app list search and native note creation", async () => {
    await launchFromSearch("OneNote", "OneNote");
    await clickLabel("new");
    await edit("FormField0", noteTitle);
    await edit("FormField1", originalBody);
    await capture("04-native-note-form");
    await clickLabel("save");
    noteId = await until(async () => (await state()).notes.find(note =>
      note.title === noteTitle && note.body === originalBody)?.id, "created note");
    await clickText(noteTitle);
    await isRoute({ Note: noteId });
    await capture("05-note-created");
  });
  await check("native note edit and reload persistence", async () => {
    await clickLabel("edit");
    await edit("FormField1", editedBody);
    await clickLabel("save");
    await until(async () => (await state()).notes.find(note => note.id === noteId)?.body === editedBody,
      "edited note");
    const stored = await page.evaluate("JSON.parse(localStorage.getItem('ferroui.windows-phone.v1'))");
    assert.equal(stored.notes.find(note => note.id === noteId)?.body, editedBody);
    assert.equal(stored.tiles.find(tile => tile.id === phoneTile.id)?.size, resizedTileSize);
    await reload();
    const restored = await state();
    assert.equal(restored.notes.find(note => note.id === noteId)?.body, editedBody);
    assert.equal(restored.tiles.find(tile => tile.id === phoneTile.id)?.size, resizedTileSize);
    await launchFromSearch("OneNote", "OneNote");
    await clickText(noteTitle);
    await isRoute({ Note: noteId });
    await capture("06-note-restored");
  });
  await check("native secondary note tile opens its note", async () => {
    await clickLabel("pin to start");
    await isRoute("Start");
    const pinned = await until(async () => (await state()).tiles.find(tile =>
      tile.target?.Note === noteId), "pinned note destination");
    const startScroll = await control(element => element.name === "StartScroll", "Start scroll viewer");
    await page.wheel(...centre(startScroll), 0, -100000);
    const tile = await reveal(element => element.name === `Tile-${pinned.id}`, "pinned note tile",
      element => element.name === "StartScroll", "Start scroll viewer");
    await capture("06b-note-pinned");
    await page.click(...centre(tile));
    await isRoute({ Note: noteId });
    await textPresent(noteTitle, "pinned note content");
  });
  await check("native note deletion", async () => {
    await clickLabel("delete");
    await textPresent("delete this note?", "note deletion confirmation");
    await clickLabel("confirm");
    await until(async () => !(await state()).notes.some(note => note.id === noteId), "deleted note");
    assert(!(await state()).tiles.some(tile => tile.target?.Note === noteId),
      "Deleting a note should also remove its secondary tile");
    await isRoute({ App: "OneNote" });
  });
  await check("app search launches a working calculator", async () => {
    await openApps();
    await edit("AppSearch", "Calculator");
    // TextBox edits are observable before the retained result list commits.
    await until(async () => {
      const filtered = (await snapshot()).elements.filter(element =>
        element.hit && button(element) && element.name?.startsWith("App-"));
      assert.equal(filtered.length, 1, "App search should show one matching app");
      assert.equal(filtered[0].automationName, "Calculator");
      return true;
    }, "exactly one clickable Calculator search result");
    await capture("07-app-search");
    await clickLabel("Calculator");
    await isRoute({ App: "Calculator" });
    for (const label of ["Clear", "7", "+", "8", "Equals"]) await clickLabel(label);
    await presence(element => element.automationName === "Calculator result" && element.text === "15",
      "calculator 7 + 8 = 15");
    await capture("08-calculator");
  });
  await check("Action Center toggles and clears notifications", async () => {
    await home();
    const wifiBefore = (await state()).settings.wi_fi;
    await clickName("StatusBar");
    await control(element => element.name === "ActionCenter", "Action Center");
    await clickLabel("Toggle Wi-Fi");
    await until(async () => (await state()).settings.wi_fi === !wifiBefore, "Wi-Fi setting toggled");
    assert((await state()).notifications.length > 0, "The seeded notification list should be nonempty");
    await clickLabel("Clear all notifications");
    await until(async () => (await state()).notifications.length === 0, "notifications cleared");
    await capture("09-action-center");
    await clickLabel("Close action center");
    await control(element => element.name === "HardwareStart", "Action Center dismissed");
  });
  await check("native quick-action customization updates Action Center", async () => {
    await launchFromSearch("Settings", "Settings");
    await clickText("notifications+actions");
    await isRoute({ Setting: "notifications+actions" });
    await clickLabel("Choose quick action 2: Bluetooth");
    await clickLabel("Use location for quick action 2");
    await until(async () => (await state()).settings.quick_actions[1] === "location",
      "configured second quick-action slot");
    assert.equal(new Set((await state()).settings.quick_actions).size, 4,
      "Each quick-action slot must have a distinct action");
    const stored = await page.evaluate("JSON.parse(localStorage.getItem('ferroui.windows-phone.v1'))");
    assert.equal(stored.settings.quick_actions[1], "location");
    await capture("09b-quick-action-settings");
    await home();
    const locationBefore = (await state()).settings.location;
    await clickName("StatusBar");
    await control(element => button(element) && element.automationName === "Toggle location",
      "configured location quick action");
    assert(!(await snapshot()).elements.some(element =>
      element.hit && button(element) && element.automationName === "Toggle Bluetooth"),
    "The replaced Bluetooth quick action should be absent");
    await clickLabel("Toggle location");
    await until(async () => (await state()).settings.location === !locationBefore,
      "configured quick action changed the local location profile");
    await capture("09c-customized-action-center");
    await clickLabel("Close action center");
  });
  await check("native virtualized Photos rows update and select", async () => {
    await launchFromSearch("Photos", "Photos");
    const listMatch = element => /ListBox$/.test(element.type);
    await control(listMatch, "native Photos list");
    await until(async () => (await snapshot()).elements.some(element =>
      element.hit && /ListBoxItem$/.test(element.type)), "native Photos rows");
    const visiblePhotos = (await snapshot()).elements.filter(element =>
      element.hit && button(element) && element.automationName?.startsWith("Smoke photo "));
    assert(visiblePhotos.length > 0 && visiblePhotos.length < photoFixtures.length,
      "The native gallery must show the visible subset of its photo rows");
    await capture("photos-virtualized-grid");
    const last = photoFixtures.at(-1);
    const lastButton = await reveal(element => button(element) && element.automationName === last.name,
      "last fixture photo", listMatch, "native Photos list");
    await page.click(...centre(lastButton));
    await textPresent(last.name, "selected photo caption");
    await clickLabel("favorite");
    await until(async () => (await state()).photos.find(photo => photo.id === last.id)?.favorite,
      "selected photo favorite state");
    await capture("photos-scrolled-selection");
    await clickName("HardwareBack");
    await control(listMatch, "Photos grid after Back");

    const importedName = "Smoke imported photo";
    await page.evaluate(`phoneApp.photoImported(${JSON.stringify(importedName)}, "forest")`);
    const imported = await until(async () => (await state()).photos.find(photo => photo.name === importedName),
      "photo-import host callback");
    await isRoute({ App: "Photos" });
    const importedButton = await reveal(element => button(element) && element.automationName === importedName,
      "newly imported photo row", listMatch, "updated native Photos list");
    await page.click(...centre(importedButton));
    await textPresent(importedName, "newly imported photo selection");
    await capture("photos-imported-selection");
    assert.equal((await state()).photos.find(photo => photo.id === imported.id)?.source, "forest");

    await home();
    const cleaned = await state();
    cleaned.photos = cleaned.photos.filter(photo => originalPhotoIds.has(photo.id));
    await page.evaluate(`phoneApp.importData(${JSON.stringify(JSON.stringify(cleaned))})`);
    await isRoute("Start");
  });
  await check("all 23 app surfaces render", async () => {
    for (const [label, id] of apps) {
      const before = JSON.stringify((await snapshot()).elements.map(element => [element.name, element.text]));
      await page.evaluate(`phoneApp.openApp(${JSON.stringify(label)})`);
      await isRoute({ App: id });
      await until(async () => JSON.stringify((await snapshot()).elements.map(element =>
        [element.name, element.text])) !== before, `${label} native view committed`);
      await settledPage(`${label} gallery entrance finishes before its reference capture`);
      await capture(`app-${id.replace(/([a-z])([A-Z])/g, "$1-$2").toLowerCase()}`);
    }
  });
  await check("native task switcher previews, resumes, and closes an app", async () => {
    const openSwitcher = async () => {
      const [x, y] = centre(await control(element => element.name === "HardwareBack", "hardware Back"));
      await page.mouseMove(x, y);
      await page.send("Input.dispatchMouseEvent", {
        type: "mousePressed", x, y, button: "right", buttons: 2, clickCount: 1, pointerType: "mouse",
      });
      await page.send("Input.dispatchMouseEvent", {
        type: "mouseReleased", x, y, button: "right", buttons: 0, clickCount: 1, pointerType: "mouse",
      });
      await control(element => element.name === "AppSwitcher", "native task switcher");
    };
    // Leaving the last gallery page captures its real retained-control image.
    await home();
    await openSwitcher();
    await until(async () => (await snapshot()).elements.some(element =>
      element.name === "AppPreview-data" && /Image$/.test(element.type) &&
      element.width > 0 && element.height > 0), "Data Sense route snapshot");
    await capture("task-switcher-preview");
    await clickLabel("Switch to Data Sense");
    await isRoute({ App: "DataSense" });
    await until(async () => !(await snapshot()).elements.some(element => element.name === "AppSwitcher"),
      "task switcher dismissed after resume");
    await openSwitcher();
    await clickLabel("Close Data Sense");
    await until(async () => !(await snapshot()).elements.some(element =>
      element.automationName === "Switch to Data Sense" || element.name === "AppPreview-data"),
    "closed app card and snapshot removed");
    await isRoute("Start");
    await capture("task-switcher-closed-card");
    await clickName("HardwareBack");
    await until(async () => !(await snapshot()).elements.some(element => element.name === "AppSwitcher"),
      "hardware Back dismisses task switcher");
  });
  const refinementContext = { page, check, state, snapshot, until, capture, button, centre, control, rawControl, output,
    clickName, clickLabel, clickText, textPresent, reveal, edit, home, isRoute, reload };
  // These modules restore isolated fixtures in finally. Collect their independent
  // failures in one opt-in run, while keeping every assertion a publication gate.
  const refinementFailures = [];
  for (const verify of [verifyOfficeRefinements, verifyCalendarRefinements, verifyCalendarTasks,
    verifyAlarmRefinements, verifyMetroControls, verifyHubPivots, verifyMetroProgress, verifyNotificationGroups, verifyPhoneControlStates, verifyPhonePickers, verifyNavigationMotion, verifyOutlookDrafts, verifyPhoneCalls, verifyShellParity, verifySystemRefinements,
    verifyBrowserRefinements, verifyPimRefinements, verifyHostRefinements, verifySdkGames,
    verifyOfficeExpansion, verifyMediaLibraries, verifyConnectedApps, verifyMapsExpansion, verifyPlatformRefinements,
    verifyLiveTiles, verifyStartCustomization, verifyLockscreenDraft, verifyLiveFolder]) {
    try { await verify(refinementContext); }
    catch (error) {
      refinementFailures.push({ module: verify.name, error: error.stack || String(error) });
      results.refinementFailures = refinementFailures;
      console.error(`FAIL ${verify.name}: ${error.stack || error}`);
      if (page.errors.length || Date.now() >= deadline) throw error;
    }
  }
  await check("narrow viewport and trusted touch navigation", async () => {
    await page.resize(390, 844);
    await until(async () => {
      const view = await snapshot();
      return view.width === 390 && view.height === 844;
    }, "390 × 844 native viewport");
    await home();
    await capture("10-start-mobile");
    await page.send("Emulation.setTouchEmulationEnabled", { enabled: true, maxTouchPoints: 5 });
    const screen = await control(element => element.name === "PhoneScreen", "mobile phone surface");
    const y = screen.y + Math.min(screen.height * 0.45, 380);
    await touchSwipe([screen.x + screen.width - 42, y], [screen.x + 42, y]);
    await isRoute("Apps");
    await control(element => element.name === "AppSearch", "touch-opened app list");
    await capture("11-app-list-mobile");
    await touchSwipe([screen.x + 42, y], [screen.x + screen.width - 42, y]);
    await isRoute("Start");
    await capture("12-start-after-swipe");
    await page.send("Emulation.setTouchEmulationEnabled", { enabled: false });
  });
  await check("mobile DPR2 canvas, native layout, pixels, and input agree", async () => {
    const firstBrowser = page;
    // A real 2× browser plus explicit metrics also permits reproducing the
    // physical-2×/advertised-1× ResizeObserver mismatch of viewport emulation.
    page = await open(site, {
      query: "?nooffline=1", width: 390, height: 844, scale: 2, network: true, navigate: false,
    });
    await installMapTileFixture(page);
    await firstBrowser.close();
    page.errors.unshift(...firstBrowser.errors);
    page.log.unshift(...firstBrowser.log);
    page.requests.unshift(...firstBrowser.requests);
    await page.send("Emulation.setDeviceMetricsOverride", {
      width: 390, height: 844, deviceScaleFactor: 2, mobile: true,
    });
    await page.navigate();
    await ready();
    await isRoute("Start");
    await verifyMobileDensity("13-start-mobile-dpr2", 2);
  });
  await check("emulated DPR1 on a physical DPR2 browser stays full size", async () => {
    await page.send("Emulation.setDeviceMetricsOverride", {
      width: 390, height: 844, deviceScaleFactor: 1, mobile: true,
    });
    await reload();
    await isRoute("Start");
    await verifyMobileDensity("14-start-mobile-emulated-dpr1", 1);
  });
  await check("offline canonical directory reload renders and accepts native input", async () => {
    await verifyOfflineStartup(page, { ready, until, capture, clickName, isRoute });
  });
  healthy();
  if (refinementFailures.length) {
    throw new Error(`${refinementFailures.length} native refinement module(s) failed: ` +
      refinementFailures.map(failure => failure.module).join(", "));
  }
  results.passed = true;
} catch (error) {
  results.passed = false;
  results.error = error.stack || String(error);
  console.error(results.error);
  if (page) {
    await page.screenshot(path.join(output, "failure.png")).catch(() => {});
    await snapshot().then(view => writeFile(path.join(output, "failure-view.json"),
      JSON.stringify(view, null, 2))).catch(() => {});
  }
  process.exitCode = 1;
} finally {
  results.elapsedMs = Date.now() - started;
  if (page) {
    results.errors = [...page.errors];
    results.requests = [...new Set(page.requests)];
    await writeFile(path.join(output, "browser.log"), `${page.log.join("\n")}\n`);
    await page.close();
  }
  await writeFile(path.join(output, "results.json"), JSON.stringify(results, null, 2));
  console.log(`${results.passed ? "PASS" : "FAIL"}: ${results.checks.length} checks, ` +
    `${results.captures.length} captures, ${results.elapsedMs} ms`);
}
