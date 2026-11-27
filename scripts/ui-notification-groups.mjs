import assert from "node:assert/strict";

// Document fixtures never synthesize device notification authority. Rust
// contract tests cover provider keys and revocation; a mounted shell test covers
// mixed-store gestures. These browser checks use real touch input against that
// same shell controller.
export async function verifyNotificationGroups(ctx) {
  const {page, check, state, snapshot, until, capture, control, centre,
    clickName, clickLabel, textPresent, isRoute} = ctx;
  const original = await state();
  const fixture = structuredClone(original);
  Object.assign(fixture.settings, {lock_timeout:0, reduced_motion:true,
    live_tiles:false, notifications:true, notifications_on_lock:true});
  fixture.settings.app_notifications = {};
  fixture.alarms = [];
  fixture.events = [];
  const now = Math.floor(Date.now()/1000);
  const notice = (app, title, ago) => ({id:fixture.next_id++, app,
    title, body:`${title} body`, timestamp:now-ago, route:{App:app}});
  const first = notice("Outlook", "Newest mail", 1);
  const sibling = notice("Outlook", "Earlier mail", 10);
  const second = notice("Messaging", "Later message", 5);
  const third = notice("Phone", "Oldest call", 20);
  // Interleave input order: the visual groups and their children must sort by
  // timestamp without rewriting the user's stored notification document.
  fixture.notifications = [third, sibling, second, first];
  const frame = () => page.evaluate("new Promise(resolve=>requestAnimationFrame(()=>requestAnimationFrame(()=>resolve(true))))");
  const named = (view, name) => view.elements.find(element => element.name === name);
  const touch = async (type, points=[]) => {
    const result = await page.send("Input.dispatchTouchEvent", {type,
      touchPoints:points.map(([x,y], id) => ({x,y,id,radiusX:3,radiusY:3,force:1}))});
    assert(!result?.error, JSON.stringify(result));
    await frame();
  };
  const load = async data => {
    await page.evaluate(`phoneApp.importData(${JSON.stringify(JSON.stringify(data))})`);
    await isRoute("Start");
    await clickName("StatusBar");
    await control(element => element.name === "ActionCenter", "mounted Action Center");
    await frame();
  };
  const group = id => control(element => element.name === `Notification-${id}`, `notification group ${id}`);
  const start = element => [element.x+34, element.y+42];
  const parallel = async (from, stationary=false) => {
    const other = [from[0], from[1]+24];
    await touch("touchStart", [from, other]);
    for (let step=1; step<=6; step++) {
      const dx=100*step/6;
      await touch("touchMove", [[from[0]+dx,from[1]],
        [other[0]+(stationary?0:dx),other[1]]]);
    }
    await touch("touchEnd");
  };
  let touchEnabled = false;
  let scenarioError;
  try {
    await page.send("Emulation.setTouchEmulationEnabled", {enabled:true,maxTouchPoints:2});
    touchEnabled = true;
    await check("Action Center sorts groups by newest item and cancels native dismissal cleanly", async () => {
      await load(fixture);
      const a=await group(first.id), b=await group(second.id), c=await group(third.id);
      assert(a.y<b.y&&b.y<c.y, "group chronology is independent of the original document order");
      assert(!named(await snapshot(),`Notification-${sibling.id}`), "one group contains both mail items");
      assert.deepEqual((await state()).notifications, fixture.notifications,
        "rendering chronology leaves document order intact");
      await capture("action-center-chronological-groups");
      const from=start(a), to=[from[0]+100,from[1]];
      await touch("touchStart", [from]);
      await touch("touchMove", [to]);
      const moved=await group(first.id);
      assert(moved.x>a.x+30, "the native group follows the contact before release");
      assert.deepEqual((await state()).notifications,fixture.notifications);
      await touch("touchCancel");
      await until(async()=>{
        const restored=await group(first.id);
        return Math.abs(restored.x-a.x)<0.6;
      }, "cancelled notification group returns to its original bounds");
      assert.deepEqual((await state()).notifications,fixture.notifications,
        "cancellation neither dismisses nor activates a notification");
      await isRoute("Start");
    });

    await check("native group swipe and parallel two-finger clear preserve unrelated notifications", async () => {
      const from=start(await group(first.id));
      await touch("touchStart", [from]);
      for(let step=1;step<=6;step++)await touch("touchMove",[[from[0]+100*step/6,from[1]]]);
      await touch("touchEnd");
      await until(async()=>!(await state()).notifications.some(item=>item.app==="Outlook"),
        "one completed native swipe dismisses the complete mail group");
      assert.deepEqual((await state()).notifications,[third,second],"unrelated groups retain exact data");
      await isRoute("Start");
      await capture("action-center-native-group-dismissal");
      const next=start(await group(second.id));
      await parallel(next,true);
      assert.deepEqual((await state()).notifications,[third,second],
        "a stationary second contact cannot convert a swipe into clear-all");
      await parallel(next);
      await until(async()=>(await state()).notifications.length===0,
        "two parallel completed contacts clear the Action Center");
      await textPresent("no new notifications","empty Action Center");
      await capture("action-center-parallel-clear");
    });

    await check("private lock-screen Action Center cannot clear hidden notification content", async () => {
      const locked=structuredClone(fixture);
      locked.settings.notifications_on_lock=false;
      await load(locked);
      await clickLabel("Close action center");
      await page.evaluate("phoneApp.lockPhone()");
      await clickLabel("Open action center from lock screen");
      const view=await snapshot();
      assert(!view.elements.some(element=>element.name?.startsWith("Notification-")),
        "private notification groups are absent from the mounted tree");
      await textPresent("unlock to see notifications","lock privacy message");
      const clear=await until(async()=>(await snapshot()).elements.find(element=>
        element.automationName==="Clear all notifications"&&element.hit&&element.enabled===false),
        "disabled clear command");
      await page.click(...centre(clear));
      const center=await control(element=>element.name==="ActionCenter","private Action Center");
      await parallel([center.x+40,center.y+300]);
      assert.deepEqual((await state()).notifications,fixture.notifications,
        "disabled clear and a two-finger gesture both preserve hidden records");
      await capture("action-center-private-lock-content");
    });
  } catch (error) {
    scenarioError = error;
    await capture("notification-groups-failure").catch(()=>{});
    throw error;
  } finally {
    const cleanupErrors=[];
    const restore=async action=>{try{await action();}catch(error){cleanupErrors.push(error);}};
    await restore(async()=>{
      if(touchEnabled){
        await touch("touchCancel").catch(()=>{});
        await page.send("Emulation.setTouchEmulationEnabled",{enabled:false});
      }
    });
    // Document import deliberately preserves the shell's lock state. Restore
    // this fixture through its mounted controls before importing user data.
    await restore(async()=>{
      if(named(await snapshot(),"ActionCenter"))await clickLabel("Close action center");
      if(named(await snapshot(),"LockScreen"))await clickLabel("Unlock");
      await until(async()=>!named(await snapshot(),"LockScreen"),"notification fixture unlocks the shell");
    });
    await restore(async()=>{
      await page.evaluate(`phoneApp.importData(${JSON.stringify(JSON.stringify(original))})`);
      await isRoute("Start");
      await until(async()=>!named(await snapshot(),"LockScreen"),"restored Start has no lock layer");
    });
    if(cleanupErrors.length){
      if(!scenarioError)throw cleanupErrors[0];
      console.error("Notification fixture cleanup also failed:",...cleanupErrors);
    }
  }
}
