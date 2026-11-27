import assert from "node:assert/strict";
import { sleep } from "../FerroUI/scripts/browser/harness.mjs";

// The document fixture supplies enough local rows to scroll. Selection, text
// editing, scrolling and navigation all use real native input. inspectView only
// observes the mounted Pivot and each ScrollViewer's actual offset.
export async function verifyHubPivots(ctx) {
  const { page, check, state, snapshot, until, capture, centre, control,
    clickName, clickLabel, edit, isRoute } = ctx;
  const original = await state();
  const viewport = await snapshot();
  const fixture = structuredClone(original);
  Object.assign(fixture.settings, { lock_timeout: 0, reduced_motion: false,
    battery_saver: false, high_contrast: false, dark: true, text_scale: 1, live_tiles: false });
  fixture.alarms = [];
  fixture.events = [];
  fixture.installed = [...new Set([...fixture.installed, "Office", "Settings"])];
  const added = Array.from({ length: 18 }, (_, index) => ({
    id: fixture.next_id++, name: `Pivot document ${String(index + 1).padStart(2, "0")}`,
    folder: null, modified: 10000 + index,
    content: { Word: { paragraphs: [{ text: `Retained document ${index + 1}.` }] } },
    import_notice: "",
  }));
  fixture.documents.push(...added);
  const named = (view, name) => view.elements.find(element => element.name === name);
  const frame = () => page.evaluate(
    "new Promise(resolve=>requestAnimationFrame(()=>requestAnimationFrame(()=>resolve(true))))");
  const load = async data => {
    await page.evaluate(`phoneApp.importData(${JSON.stringify(JSON.stringify(data))})`);
    await isRoute("Start");
  };
  const app = async name => {
    await page.evaluate(`phoneApp.openApp(${JSON.stringify(name)})`);
    await isRoute({ App: name });
  };
  const selected = (name, index) => until(async () => {
    const pivot = named(await snapshot(), name);
    return pivot?.selectedIndex === index && /Pivot$/.test(pivot.type) ? pivot : undefined;
  }, `${name} selects ${index}`);
  const header = label => control(element => /PivotHeaderItem$/.test(element.type) &&
    element.text === label, `native ${label} header`);
  const selectHeader = async label => page.click(...centre(await header(label)));
  const offset = async name => {
    await control(element => element.name === name, `${name} settles`);
    return named(await snapshot(), name).scrollOffset.y;
  };
  const near = (actual, expected, message) =>
    assert(Math.abs(actual - expected) < 1, `${message}: ${actual} versus ${expected}`);
  const touch = (type, points = []) => page.send("Input.dispatchTouchEvent", {
    type, touchPoints: points.map(([x, y]) => ({ x, y, id: 1, radiusX: 2, radiusY: 2, force: 1 })),
  });
  const swipe = async name => {
    const pane = await control(element => element.name === name, `${name} touch pane`);
    const y = pane.y + Math.min(pane.height * 0.55, 280);
    const start = pane.x + pane.width - 36, end = pane.x + 36;
    await touch("touchStart", [[start, y]]);
    for (let step = 1; step <= 10; step++) {
      await touch("touchMove", [[start + (end - start) * step / 10, y]]);
      await sleep(16);
    }
    await touch("touchEnd");
  };

  try {
    await load(fixture);
    await check("Settings retains independent pane scrolling with fixed native Pivot headers", async () => {
      await app("Settings");
      await selected("SettingsPivot", 0);
      const document = await state();
      const before = await header("system");
      const scroll = await control(element => element.name === "SettingsSystemScroll", "system pane");
      await page.wheel(...centre(scroll), 0, 430);
      await until(async () => named(await snapshot(), "SettingsSystemScroll")?.scrollOffset?.y > 200,
        "system list scrolls vertically");
      const remembered = await offset("SettingsSystemScroll");
      near((await header("system")).y, before.y, "Pivot heading stays fixed during list scroll");
      await capture("metro-settings-system-scrolled");
      await selectHeader("applications");
      await selected("SettingsPivot", 1);
      near(await offset("SettingsApplicationsScroll"), 0, "applications has its own scroll origin");
      await capture("metro-settings-applications-native");
      await page.send("Emulation.setTouchEmulationEnabled", { enabled: true, maxTouchPoints: 5 });
      await swipe("SettingsApplicationsScroll");
      await selected("SettingsPivot", 0);
      near(await offset("SettingsSystemScroll"), remembered, "cyclic touch returns to the retained system offset");
      await isRoute({ App: "Settings" });
      assert.deepEqual(await state(), document, "pivot scrolling and swiping leave all settings unchanged");
      await page.send("Emulation.setTouchEmulationEnabled", { enabled: false });
    });

    await check("Office retains both native panes and keeps search editing outside Pivot gestures", async () => {
      await app("Office");
      await selected("OfficePivot", 0);
      const document = await state();
      const recent = await control(element => element.name === "OfficeRecentScroll", "recent documents pane");
      await page.wheel(...centre(recent), 0, 360);
      await until(async () => named(await snapshot(), "OfficeRecentScroll")?.scrollOffset?.y > 160,
        "recent documents scroll");
      const remembered = await offset("OfficeRecentScroll");
      await selectHeader("places");
      await selected("OfficePivot", 1);
      near(await offset("OfficePlacesScroll"), 0, "places retains an independent scroll origin");
      await capture("metro-office-places-native");
      await control(element => element.name === "HeaderClipper" && element.focused, "Office header keyboard focus");
      await page.press("ArrowLeft");
      await selected("OfficePivot", 0);
      near(await offset("OfficeRecentScroll"), remembered, "keyboard navigation retains recent scroll offset");
      await selectHeader("places");
      await selected("OfficePivot", 1);
      await clickLabel("search");
      await control(element => element.name === "OfficeSearchInput" && element.focused,
        "standalone Office search takes native editor focus");
      assert(!named(await snapshot(), "OfficePivot"), "the editing page does not contain a competing Pivot");
      await edit("OfficeSearchInput", added[3].name);
      await control(element => element.automationName === `${added[3].name}.docx`, "matching local document");
      const matches = (await snapshot()).elements.filter(element => element.hit &&
        element.automationName?.startsWith("Pivot document ") && /Button$/.test(element.type));
      assert.equal(matches.length, 1, "search filters the actual local document list");
      await capture("metro-office-search-page");
      await clickName("HardwareBack");
      await selected("OfficePivot", 1);
      await isRoute({ App: "Office" });
      assert(!named(await snapshot(), "OfficeSearchInput"), "Back closes search before leaving Office");
      near(await offset("OfficePlacesScroll"), 0, "search does not overwrite the places position");
      await selectHeader("recent");
      await selected("OfficePivot", 0);
      near(await offset("OfficeRecentScroll"), remembered,
        "search Back restores the destroyed recent ScrollViewer");
      await capture("metro-office-recent-restored-search");
      const pane = await control(element => element.name === "OfficeRecentScroll", "restored recent pane");
      const insidePane = element => element.x >= pane.x && element.x + element.width <= pane.x + pane.width &&
        element.y >= pane.y && element.y + element.height <= pane.y + pane.height;
      const target = await until(async () => (await snapshot()).elements
        .filter(element => /Button$/.test(element.type) && element.hit && insidePane(element) &&
          added.some(document => `${document.name}.docx` === element.automationName))
        .sort((left, right) => left.y - right.y || left.x - right.x ||
          left.automationName.localeCompare(right.automationName))[0],
      "a fully visible fixture document in the restored pane");
      const visible = await control(element => /Button$/.test(element.type) && insidePane(element) &&
        element.automationName === target.automationName, "the selected visible fixture document");
      const opened = added.find(document => `${document.name}.docx` === visible.automationName);
      assert(opened, "the real row corresponds to the isolated fixture");
      await page.click(...centre(visible));
      await isRoute({ Document: opened.id });
      await clickName("HardwareBack");
      await selected("OfficePivot", 0);
      near(await offset("OfficeRecentScroll"), remembered,
        "document Back restores the same recent position");
      await capture("metro-office-recent-restored-document");
      assert.deepEqual(await state(), document, "search and pane navigation do not edit documents");
    });

    await check("Office reconstructs independent scrolled panes and search results at 320 pixels", async () => {
      await page.resize(320, 460);
      await until(async () => (await snapshot()).width === 320, "compact Office viewport");
      await load(fixture);
      await app("Office");
      await selected("OfficePivot", 0);
      near(await offset("OfficeRecentScroll"), 0, "document replacement starts a fresh scroll epoch");
      const document = await state();
      const recent = await control(element => element.name === "OfficeRecentScroll", "compact recent pane");
      await page.wheel(...centre(recent), 0, 320);
      await until(async () => (await offset("OfficeRecentScroll")) > 160, "compact recent scroll");
      const recentPosition = await offset("OfficeRecentScroll");
      await selectHeader("places");
      await selected("OfficePivot", 1);
      const places = await control(element => element.name === "OfficePlacesScroll", "compact places pane");
      await page.wheel(...centre(places), 0, 100);
      await until(async () => (await offset("OfficePlacesScroll")) > 20, "compact places scroll");
      const placesPosition = await offset("OfficePlacesScroll");
      await clickLabel("search");
      await control(element => element.name === "OfficeSearchInput" && element.focused,
        "compact search editor focus");
      const results = await control(element => element.name === "OfficeSearchScroll", "independent search results");
      near(results.scrollOffset.y, 0, "new search starts at its own origin");
      await page.wheel(...centre(results), 0, 240);
      await until(async () => (await offset("OfficeSearchScroll")) > 120, "search results really scroll");
      await edit("OfficeSearchInput", added[7].name);
      await control(element => element.automationName === `${added[7].name}.docx`, "compact search result");
      near(await offset("OfficeSearchScroll"), 0, "a new filter starts at the top of its results");
      await clickName("HardwareBack");
      await selected("OfficePivot", 1);
      near(await offset("OfficePlacesScroll"), placesPosition, "Back restores the scrolled places pane");
      await capture("metro-office-320-places-restored-search");
      await selectHeader("recent");
      await selected("OfficePivot", 0);
      near(await offset("OfficeRecentScroll"), recentPosition, "recent retains a separate nonzero offset");
      await clickLabel("new");
      await clickLabel("cancel");
      await selected("OfficePivot", 0);
      near(await offset("OfficeRecentScroll"), recentPosition, "cancelling new restores the recent pane");
      await capture("metro-office-320-recent-restored-new");
      assert.deepEqual(await state(), document, "all restored offsets remain session-only state");
    });

    await check("Settings and Office native headings stay reachable at 320 pixels in both themes", async () => {
      await page.resize(320, 740);
      await until(async () => (await snapshot()).width === 320, "narrow native viewport");
      for (const dark of [true, false]) {
        const compact = structuredClone(fixture);
        compact.settings.dark = dark;
        await load(compact);
        await app("Settings");
        await selected("SettingsPivot", 0);
        await selectHeader("applications");
        await selected("SettingsPivot", 1);
        const title = await header("applications");
        const screen = named(await snapshot(), "PhoneScreen");
        assert(title.x >= screen.x && title.x < screen.x + 28,
          "the selected long heading wraps to the leading phone inset");
        await capture(`metro-settings-320-${dark ? "dark" : "light"}`);
        await app("Office");
        await selectHeader("places");
        await selected("OfficePivot", 1);
        await control(element => element.name === "OfficePlacesScroll", "compact Office pane");
        await capture(`metro-office-320-${dark ? "dark" : "light"}`);
      }
    });

    await check("reduced motion preserves Settings and Office hub commands without native paging animation", async () => {
      const reduced = structuredClone(fixture);
      reduced.settings.reduced_motion = true;
      await load(reduced);
      for (const [name, pivot, label, pane] of [
        ["Settings", "SettingsPivot", "applications", "SettingsApplicationsScroll"],
        ["Office", "OfficePivot", "places", "OfficePlacesScroll"],
      ]) {
        await app(name);
        await clickLabel(label);
        await control(element => element.name === pane, `${name} static pane`);
        assert(!(await snapshot()).elements.some(element => element.name === pivot && /Pivot$/.test(element.type)),
          "reduced motion retains the static hub surface");
        await isRoute({ App: name });
      }
      await capture("metro-office-reduced-motion-320");
    });
  } catch (error) {
    await capture("metro-hub-pivots-failure").catch(() => {});
    throw error;
  } finally {
    await touch("touchCancel").catch(() => {});
    await page.send("Emulation.setTouchEmulationEnabled", { enabled: false });
    await page.resize(viewport.width, viewport.height);
    await frame();
    await load(original);
  }
}
