import assert from "node:assert/strict";

// Local document fixtures exercise acknowledgement through the actual Phone
// command. History rows are never activated, so no dial or host call is made.
export async function verifyPhoneCalls(ctx) {
  const {page, check, state, snapshot, until, capture, control,
    clickName, clickLabel, home, isRoute, reload} = ctx;
  const original = await state();
  const fixture = structuredClone(original);
  Object.assign(fixture.settings, {lock_timeout:0, reduced_motion:true,
    live_tiles:false, sounds:false, ringer_volume:0, vibration:false});
  fixture.alarms = [];
  fixture.events = [];
  fixture.installed = [...new Set([...fixture.installed, "Phone"])];
  const phoneTile = fixture.tiles.find(tile => tile.app === "Phone" &&
    !tile.target && !tile.children.length && !tile.folder_tiles?.length);
  assert(phoneTile, "the local fixture has a built-in Phone tile");
  fixture.tiles = [{...phoneTile, size:"Medium", notifications:[], appearance:{
    label:null, background:"Accent", show_label:true, show_badge:true, live:false, queue:false,
  }}];
  const now = Math.floor(Date.now()/1000);
  const makeCall = (name, seconds, missed, acknowledged=false) => ({
    id:fixture.next_id++, name, number:"+12025550199", timestamp:now-seconds,
    incoming:missed, missed, acknowledged,
  });
  const first = makeCall("Missed acknowledgement A", 30, true);
  const second = makeCall("Missed acknowledgement B", 60, true);
  const old = makeCall("Previously acknowledged call", 90, true, true);
  const outgoing = makeCall("Existing outgoing call", 120, false);
  fixture.call_history = [first, second, old, outgoing];
  const badgeName = `TileBadge-${phoneTile.id}`;
  const load = async data => {
    await page.evaluate(`phoneApp.importData(${JSON.stringify(JSON.stringify(data))})`);
    await isRoute("Start");
    await control(element => element.name === `Tile-${phoneTile.id}`, "settled Phone tile");
  };
  const phone = async () => {
    await clickName(`Tile-${phoneTile.id}`);
    await isRoute({App:"Phone"});
    await control(element => element.automationName === "history", "Phone history pivot");
  };
  const badge = async count => {
    await control(element => element.name === `Tile-${phoneTile.id}`, "Phone tile before badge inspection");
    await until(async () => {
      const found = (await snapshot()).elements.filter(element => element.name === badgeName);
      return count === 0 ? found.length === 0 : found.length === 1 && found[0].text === String(count);
    }, `Phone badge reflects ${count} new missed calls`);
  };
  const assertHistory = (actual, expected) => {
    assert.deepEqual(actual.map(({acknowledged, ...call}) => call),
      expected.map(({acknowledged, ...call}) => call),
      "acknowledgement preserves every history row, direction, number, name and timestamp");
  };
  const persistedCalls = () => page.evaluate(
    "JSON.parse(localStorage.getItem('ferroui.windows-phone.v1')).call_history");
  try {
    await check("Phone acknowledgement clears the badge while preserving missed-call history", async () => {
      await load(fixture);
      await badge(2);
      await capture("phone-new-missed-call-badge");
      await phone();
      await control(element => element.automationName === "acknowledge 2 missed calls", "explicit missed-call acknowledgement");
      assert.deepEqual((await state()).call_history, fixture.call_history,
        "opening history alone does not acknowledge calls");
      await clickLabel("acknowledge 2 missed calls");
      const calls = await until(async () => {
        const calls = (await state()).call_history;
        return [first.id, second.id].every(id => calls.find(call => call.id === id)?.acknowledged) && calls;
      }, "only the selected missed calls are acknowledged");
      assertHistory(calls, fixture.call_history);
      assert(calls.find(call => call.id === old.id).acknowledged);
      assert.equal(calls.find(call => call.id === outgoing.id).acknowledged, false,
        "an outgoing call does not become an acknowledged missed call");
      await until(async () => {
        const view = await snapshot();
        return view.elements.filter(element => element.type.endsWith("TextBlock") &&
          element.text?.startsWith("missed call · ")).length === 3 &&
          !view.elements.some(element => element.automationName?.startsWith("acknowledge "));
      }, "missed direction remains visible after the acknowledgement command disappears");
      assert.deepEqual(await persistedCalls(), calls);
      await capture("phone-acknowledged-history");
      await home();
      await badge(0);
      await capture("phone-acknowledged-badge-cleared");
    });

    await check("later missed calls remain new and acknowledgement survives runtime reload", async () => {
      const acknowledged = await state();
      const later = {id:acknowledged.next_id++, name:"Later missed call", number:"+12025550198",
        timestamp:now+1, incoming:true, missed:true, acknowledged:false};
      acknowledged.call_history.unshift(later);
      await load(acknowledged);
      await badge(1);
      await phone();
      await control(element => element.automationName === "acknowledge 1 missed call", "new arrival acknowledgement");
      assert.equal((await state()).call_history.find(call => call.id === later.id).acknowledged, false,
        "a previous acknowledgement cannot consume a later arrival");
      await reload();
      await isRoute("Start");
      await badge(1);
      const restored = (await state()).call_history;
      assert.deepEqual(restored, acknowledged.call_history,
        "reload restores the prior acknowledgements and the later unacknowledged call");
      await capture("phone-later-missed-call-restored");
      await phone();
      await clickLabel("acknowledge 1 missed call");
      const finalCalls = await until(async () => {
        const calls = (await state()).call_history;
        return calls.find(call => call.id === later.id)?.acknowledged && calls;
      }, "explicit second acknowledgement consumes the later arrival");
      assertHistory(finalCalls, acknowledged.call_history);
      assert.equal(finalCalls.filter(call => call.missed && !call.acknowledged).length, 0);
      assert.deepEqual(await persistedCalls(), finalCalls);
      await reload();
      await isRoute("Start");
      await badge(0);
      assert.deepEqual((await state()).call_history, finalCalls);
    });
  } catch (error) {
    await capture("phone-calls-failure").catch(() => {});
    throw error;
  } finally {
    await page.evaluate(`phoneApp.importData(${JSON.stringify(JSON.stringify(original))})`);
    await isRoute("Start");
  }
}
