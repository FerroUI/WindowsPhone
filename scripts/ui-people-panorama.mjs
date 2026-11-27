import assert from "node:assert/strict";
import { mkdir, writeFile } from "node:fs/promises";
import path from "node:path";
import { sleep } from "../FerroUI/scripts/browser/harness.mjs";

// Local, isolated contacts exercise the real native Panorama/list/editor input.
// These sections are application adaptations; no retired social feed is faked.
export async function verifyPeoplePanorama(ctx) {
  const { page, check, state, snapshot, until, capture, centre, control,
    clickName, clickLabel, textPresent, edit, isRoute, output } = ctx;
  const original = await state();
  const viewport = await snapshot();
  const fixture = structuredClone(original);
  Object.assign(fixture.settings, { lock_timeout: 0, reduced_motion: false,
    battery_saver: false, live_tiles: false, high_contrast: false, text_scale: 1, dark: true });
  fixture.alarms = []; fixture.events = []; fixture.contacts = [];
  fixture.installed = [...new Set([...fixture.installed, "People"])];
  for (let index = 0; index < 36; index++) {
    fixture.contacts.push({ id: fixture.next_id++, name: `${["Ada", "Bea", "Cora", "Zoe"][Math.floor(index / 9)]} Panorama ${String(index).padStart(2, "0")}`,
      phone: `+1202555${String(1000 + index)}`, email: `panorama${index}@example.test`,
      company: "Local panorama fixture", favorite: index % 3 === 0, color: "#1BA1E2" });
  }
  const named = (view, name) => view.elements.find(element => element.name === name);
  const frame = () => page.evaluate("new Promise(resolve=>requestAnimationFrame(()=>requestAnimationFrame(()=>resolve(true))))");
  const near = (actual, expected, label, tolerance = 1) =>
    assert(Number.isFinite(actual) && Math.abs(actual - expected) <= tolerance,
      `${label}: ${actual} versus ${expected}`);
  const touch = (type, point) => page.send("Input.dispatchTouchEvent", {
    type, touchPoints: point ? [{ x: point[0], y: point[1], id: 1, radiusX: 2, radiusY: 2, force: 1 }] : [],
  });
  const load = async data => {
    await page.evaluate(`phoneApp.importData(${JSON.stringify(JSON.stringify(data))})`);
    await isRoute("Start");
  };
  const app = async () => {
    await page.evaluate('phoneApp.openApp("People")');
    await isRoute({ App: "People" });
    return control(element => element.name === "PeoplePanorama" && /Carousel$/.test(element.type), "native People Panorama");
  };
  const selected = index => until(async () => {
    const host = named(await snapshot(), "PeoplePanorama");
    return host?.selectedIndex === index && !host.isSwiping ? host : undefined;
  }, `People section ${index} settles`);
  const slide = async (direction, index) => {
    const host = await control(element => element.name === "PeoplePanorama", "Panorama touch region");
    const y = host.y + host.height * 0.65;
    const start = host.x + host.width * (direction > 0 ? 0.82 : 0.18);
    const end = host.x + host.width * (direction > 0 ? 0.18 : 0.82);
    await touch("touchStart", [start, y]);
    for (let step = 1; step <= 10; step++) {
      await touch("touchMove", [start + (end - start) * step / 10, y]);
      await sleep(16);
    }
    await touch("touchEnd");
    await selected(index);
    await control(element => element.name === (index === 0 ? "PeopleAllList" : "PeopleFavoritesList"), "destination native list");
  };
  const topContact = async name => {
    const list = await control(element => element.name === name, `${name} input viewport`);
    const view = await snapshot();
    const candidates = view.elements.filter(element => /Button$/.test(element.type) &&
      fixture.contacts.some(contact => contact.name === element.automationName) &&
      element.x >= list.x - 1 && element.x + element.width <= list.x + list.width + 1 &&
      element.y >= list.y && element.y + element.height <= list.y + list.height && element.hit);
    assert(candidates.length, "a real complete contact row is visible inside the selected list");
    return candidates.sort((a, b) => a.y - b.y)[0];
  };
  const settledContact = async name => {
    let previous, stable = 0;
    return until(async () => {
      const current = await topContact(name);
      stable = previous?.automationName === current.automationName && Math.abs(previous.y - current.y) < 0.2 ? stable + 1 : 0;
      previous = current;
      await frame();
      return stable >= 3 ? current : undefined;
    }, `${name} finishes native vertical scrolling/restoration`);
  };
  const recordRelease = async action => {
    const sampled = page.evaluate(`new Promise((resolve,reject)=>{
      const started=performance.now(),frames=[];let token,done=false;
      const finish=()=>{if(done)return;done=true;cancelAnimationFrame(token);clearTimeout(timeout);resolve(frames);};
      const timeout=setTimeout(finish,1600);
      const sample=()=>{try{
        const view=JSON.parse(phoneApp.inspectView()),find=name=>view.elements.find(e=>e.name===name)??null;
        frames.push({time:performance.now(),elapsed:performance.now()-started,host:find('PeoplePanorama'),title:find('PanoramaTitle'),
          all:find('PeoplePanorama-section-0'),favorites:find('PeoplePanorama-section-1')});
        if(performance.now()-started>=700)finish();else token=requestAnimationFrame(sample);
      }catch(error){done=true;clearTimeout(timeout);reject(error);}};sample();
    })`);
    const [observed, acted] = await Promise.allSettled([sampled, action()]);
    if (observed.status === "rejected") throw observed.reason;
    if (output) {
      await mkdir(output, { recursive: true });
      await writeFile(path.join(output, "people-panorama-release-frames.json"), JSON.stringify({
        releasedAt: acted.status === "fulfilled" ? acted.value : null, frames: observed.value,
      }, null, 2));
    }
    if (acted.status === "rejected") throw acted.reason;
    return { frames: observed.value, releasedAt: acted.value };
  };
  let rememberedAll;

  try {
    await load(fixture);
    await page.send("Emulation.setTouchEmulationEnabled", { enabled: true, maxTouchPoints: 1 });
    await check("People Panorama owns held horizontal input, release motion and cyclic sections without activating rows", async () => {
      const host = await app();
      await selected(0);
      const before = await snapshot();
      const all = named(before, "PeoplePanorama-section-0");
      const title = named(before, "PanoramaTitle");
      assert(all && title, "native title and content layers are separately mounted");
      assert(!before.elements.some(element => /TextBox$/.test(element.type)), "the hub contains no competing editor");
      near(named(before, "PhoneApplicationBar").height, 25, "Panorama reserves the original mini extent");
      const y = host.y + host.height * 0.65, start = host.x + host.width * 0.82;
      await touch("touchStart", [start, y]);
      for (let step = 1; step <= 6; step++) {
        await touch("touchMove", [start - 120 * step / 6, y]);
        await sleep(16);
      }
      await frame();
      const held = await snapshot();
      const heldHost = named(held, "PeoplePanorama");
      const heldAll = named(held, "PeoplePanorama-section-0");
      const heldTitle = named(held, "PanoramaTitle");
      assert(heldHost?.isSwiping && heldAll && heldTitle, "the native Carousel owns the held contact");
      const contentTravel = all.x - heldAll.x, titleTravel = title.x - heldTitle.x;
      assert(contentTravel > 30, "content follows the held horizontal drag");
      assert(titleTravel > 0 && titleTravel < contentTravel, "the title travels independently and slower than content");
      await isRoute({ App: "People" });
      await capture("people-panorama-held");
      const { frames, releasedAt } = await recordRelease(async () => {
        for (let step = 1; step <= 6; step++) {
          await touch("touchMove", [start - 120 - 120 * step / 6, y]);
          await sleep(16);
        }
        await touch("touchEnd");
        return page.evaluate("performance.now()");
      });
      await selected(1);
      const destination = await control(element => element.name === "PeopleFavoritesList", "favorites list after release");
      const settling = frames.filter(frame => frame.time > releasedAt && frame.host?.isSwiping && frame.favorites);
      assert(settling.length >= 2, "native completion renders after the pointer has been released");
      const x = settling.map(frame => frame.favorites.x);
      assert(new Set(x.map(value => Math.round(value))).size >= 2, "release trace contains multiple actual completion positions");
      near(named(await snapshot(), "PeoplePanorama-section-1").x, host.x, "favorites settles at the leading edge");
      assert(destination.width > 200);
      await capture("people-panorama-favorites");
      await slide(1, 0);
      await slide(-1, 1);
      await slide(-1, 0);
      await isRoute({ App: "People" });

      const list = await control(element => element.name === "PeopleAllList", "all contacts vertical viewport");
      const initial = await topContact("PeopleAllList");
      const middle = list.x + list.width * 0.5;
      const fromY = list.y + list.height * 0.8, toY = list.y + list.height * 0.25;
      await touch("touchStart", [middle, fromY]);
      for (let step = 1; step <= 10; step++) {
        await touch("touchMove", [middle + (step % 2), fromY + (toY - fromY) * step / 10]);
        await sleep(16);
      }
      await touch("touchEnd");
      await selected(0);
      await until(async () => (await topContact("PeopleAllList")).automationName !== initial.automationName,
        "vertical touch scroll changes the contact rows without changing sections");
      rememberedAll = await settledContact("PeopleAllList");
      await capture("people-panorama-all-scrolled");
      assert.deepEqual((await state()).contacts, fixture.contacts, "native manipulation never edits or activates fixture contacts");
    });

    await check("People search and contact Back preserve the originating Panorama section and independent list state", async () => {
      await slide(1, 1);
      await clickLabel("more");
      await clickLabel("jump");
      await clickLabel("Jump to z");
      await control(element => element.automationName === "Zoe Panorama 27", "native grouped letter jump");
      const favoritesPosition = await settledContact("PeopleFavoritesList");
      await clickLabel("more");
      await clickLabel("search");
      await control(element => element.name === "PeopleSearchInput" && element.focused, "People search autofocus");
      assert(!named(await snapshot(), "PeoplePanorama"), "editing uses a separate surface");
      const target = fixture.contacts.find(contact => contact.favorite);
      await edit("PeopleSearchInput", "No matching Panorama contact");
      await textPresent("No contacts", "native empty search results");
      assert(!named(await snapshot(), "PeopleSearchList"), "an empty query result replaces only the list surface");
      await control(element => element.name === "PeopleSearchInput" && element.focused,
        "empty search preserves the native editor and focus");
      await capture("people-search-empty-results");
      await edit("PeopleSearchInput", target.name);
      await control(element => element.automationName === target.name, "actual filtered favorite");
      await clickLabel("jump");
      await control(element => element.automationName === "Jump to a", "search alphabet overlay");
      await clickName("HardwareBack");
      await control(element => element.name === "PeopleSearchInput" && element.text === target.name,
        "Back dismisses the alphabet before search");
      await capture("people-search-native-editor");
      await clickLabel(target.name);
      await isRoute({ Contact: target.id });
      assert(!named(await snapshot(), "PeoplePanorama"), "contact details do not contain hidden horizontal sections");
      await clickName("HardwareBack");
      await control(element => element.name === "PeopleSearchInput" && element.text === target.name,
        "contact Back restores the exact search query");
      await clickName("HardwareBack");
      await selected(1);
      await control(element => element.name === "PeopleFavoritesList", "favorite section restored from search");
      const restoredFavorite = await settledContact("PeopleFavoritesList");
      assert.equal(restoredFavorite.automationName, favoritesPosition.automationName);
      near(restoredFavorite.y, favoritesPosition.y, "favorites retains its own letter-jump position", 2);
      await capture("people-panorama-restored-search");
      await slide(1, 0);
      const restored = await settledContact("PeopleAllList");
      assert.equal(restored.automationName, rememberedAll.automationName, "search did not overwrite the all-contacts position");
      near(restored.y, rememberedAll.y, "native remount restores the within-row position", 2);
      assert.deepEqual((await state()).contacts, fixture.contacts, "search/detail navigation preserves saved contacts");
    });

    await check("People mini-bar expansion overlays stable 320-pixel Panorama content in both themes and reduced motion", async () => {
      await page.resize(320, 740);
      for (const [dark, reduced] of [[true, false], [false, false], [false, true]]) {
        const compact = structuredClone(fixture);
        compact.settings.dark = dark;
        compact.settings.reduced_motion = reduced;
        await load(compact);
        await app();
        await selected(0);
        const before = named(await snapshot(), "PhonePanoramaContent");
        near(named(await snapshot(), "PhoneApplicationBar").height, 25, "compact minimized bar");
        await capture(`people-panorama-320-${dark ? "dark" : "light"}${reduced ? "-reduced" : ""}`);
        await clickLabel("more");
        await control(element => element.name === "AppBarAction-search", "expanded native search command");
        const expanded = await snapshot();
        const after = named(expanded, "PhonePanoramaContent");
        for (const key of ["x", "y", "width", "height"]) near(after[key], before[key], `bar expansion keeps Panorama ${key}`);
        assert(named(expanded, "AppBarDismiss"), "expanded commands own a real dismissal overlay");
        for (const label of ["new", "jump", "search"]) {
          const command = named(expanded, `AppBarAction-${label}`);
          assert(command?.hit && command.width >= 44 && command.height >= 44, "expanded actions retain native targets");
        }
        if (!dark && !reduced) await capture("people-panorama-320-minibar-expanded");
        await clickName("HardwareBack");
        await selected(0);
        near(named(await snapshot(), "PhoneApplicationBar").height, 25, "Back returns to the requested mini mode");
        assert(!named(await snapshot(), "AppBarDismiss"));
        await slide(1, 1);
        await clickLabel("more");
        await clickLabel("search");
        await control(element => element.name === "PeopleSearchInput" && element.focused, "compact search remains reachable");
        await clickName("HardwareBack");
        await selected(1);
        assert.deepEqual((await state()).contacts, compact.contacts);
      }
    });
  } catch (error) {
    await capture("people-panorama-failure").catch(() => {});
    throw error;
  } finally {
    await touch("touchCancel").catch(() => {});
    await page.send("Emulation.setTouchEmulationEnabled", { enabled: false });
    await page.resize(viewport.width, viewport.height);
    await frame();
    await load(original);
  }
}
