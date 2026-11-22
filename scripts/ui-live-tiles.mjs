import assert from "node:assert/strict";

// Exercise the mounted native controls through the production clock bridge.
// Freeze only this isolated browser's wall clock: animation/RAF time remains
// real, and the normal host interval cannot race the deterministic tick values.
export async function verifyLiveTiles(ctx) {
  const {page,check,state,snapshot,until,capture,isRoute}=ctx;
  const original=await state();
  const fixture=structuredClone(original);
  const people=fixture.tiles.find(tile=>tile.app==="People"&&!tile.target&&!tile.children.length);
  assert(people,"The live-tile fixture requires the People tile");
  assert(fixture.contacts.length>1&&fixture.contacts.length<=26,"The fixture needs distinct contact initials");
  fixture.contacts.forEach((contact,index)=>{contact.name=`${String.fromCharCode(65+index)} Cadence`;});
  fixture.tiles=[{...people,size:"Wide"}];
  Object.assign(fixture.settings,{lock_timeout:0,live_tiles:true,battery_saver:false,reduced_motion:false});
  fixture.alarms.forEach(alarm=>{alarm.enabled=false;});
  const expected=new Set(fixture.contacts.map((_,index)=>`${String.fromCharCode(65+index)}C`));
  const load=async data=>{
    await page.evaluate(`phoneApp.importData(${JSON.stringify(JSON.stringify(data))})`);
    await isRoute("Start");
  };
  const cells=async()=>{
    const view=await snapshot();
    return Array.from({length:6},(_,index)=>{
      const found=view.elements.filter(element=>element.name===`PeopleCell-${people.id}-${index}`);
      assert.equal(found.length,1,`People cell ${index} must have one mounted native TextBlock`);
      assert(expected.has(found[0].text),`People cell ${index} must show the fixture contact`);
      return found[0].text;
    });
  };
  const settled=()=>page.evaluate("new Promise(resolve=>requestAnimationFrame(()=>requestAnimationFrame(()=>resolve(true))))");
  const tick=async seconds=>{
    await page.evaluate(`globalThis.__phoneSmokeClock.seconds=${seconds};phoneApp.tick(${seconds})`);
    await settled();
  };
  let frozen=false;
  try {
    await check("mounted People live tile advances one cell without input or persistence",async()=>{
      const base=await page.evaluate(`(()=>{
        const seconds=Math.floor(Date.now()/60000)*60+20;
        globalThis.__phoneSmokeClock={original:Date.now,seconds};
        Date.now=()=>globalThis.__phoneSmokeClock.seconds*1000;
        phoneApp.tick(seconds);
        return seconds;
      })()`);
      frozen=true;
      await load(fixture);
      const before=await until(cells,"six mounted People cells");
      await capture("live-people-before-tick");
      const persisted=await page.evaluate("localStorage.getItem('ferroui.windows-phone.v1')");
      const document=await state();
      await page.evaluate(`(()=>{
        const clock=globalThis.__phoneSmokeClock;
        clock.storageSetItem=Storage.prototype.setItem;
        clock.documentWrites=0;
        Storage.prototype.setItem=function(key,value){
          if(key==='ferroui.windows-phone.v1')clock.documentWrites++;
          return clock.storageSetItem.call(this,key,value);
        };
      })()`);

      // No navigation, pointer/keyboard input, data change, minute rollover or
      // resize occurs between these observations; only host_tick can update it.
      await tick(base+3);
      assert.deepEqual(await cells(),before,"A partial four-second bucket must retain all six cells");
      await tick(base+4);
      await until(async()=>{
        const after=await cells();
        assert.equal(after.filter((value,index)=>value!==before[index]).length,1,
          "Exactly one visible People cell must advance at the next four-second bucket");
        return true;
      },"live tile rendered after the host clock tick");
      await capture("live-people-after-tick");
      assert.deepEqual(await state(),document,"Live cadence must not mutate the persisted document");
      assert.equal(await page.evaluate("localStorage.getItem('ferroui.windows-phone.v1')"),persisted,
        "Live cadence must preserve local storage");
      assert.equal(await page.evaluate("globalThis.__phoneSmokeClock.documentWrites"),0,
        "Live cadence must not issue storage writes, including unchanged payloads");

      for(const setting of ["live_tiles","battery_saver","reduced_motion"]){
        const paused=structuredClone(fixture);
        paused.settings[setting]=setting!=="live_tiles";
        await load(paused);
        await settled();
        const still=await until(cells,`People cells with ${setting} preference`);
        await tick(base+8);
        assert.deepEqual(await cells(),still,`${setting} must suspend the live tile clock`);
        await tick(base+4);
      }
    });
  } catch(error) {
    await capture("live-people-failure").catch(()=>{});
    throw error;
  } finally {
    if(frozen)await page.evaluate(`(()=>{
      const clock=globalThis.__phoneSmokeClock;
      Date.now=clock.original;
      if(clock.storageSetItem)Storage.prototype.setItem=clock.storageSetItem;
      delete globalThis.__phoneSmokeClock;
      phoneApp.tick(Date.now()/1000);
    })()`);
    await load(original);
  }
}
