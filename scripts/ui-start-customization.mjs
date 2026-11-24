import assert from "node:assert/strict";

// Retained controls, native input and the production host clock. Fixtures use
// only existing local photos; no tile initiates external image requests.
export async function verifyStartCustomization(ctx) {
  const {page,check,state,snapshot,until,capture,control,clickName,clickLabel,edit,isRoute}=ctx;
  const original=await state();
  const fixture=structuredClone(original);
  const leaves=fixture.tiles.filter(tile=>!tile.target&&!tile.children.length&&!tile.folder_tiles?.length).slice(0,6);
  assert.equal(leaves.length,6,"Template fixture needs six distinct tiles");
  assert(fixture.photos.length>1,"Template fixture needs two local photos");
  const appearance={label:null,background:"Accent",show_label:true,show_badge:true,live:true,queue:true};
  const base=await page.evaluate(`(()=>{
    const seconds=Math.floor(Date.now()/60000)*60+8;
    globalThis.__startTilesSmoke={dateNow:Date.now,seconds};Date.now=()=>globalThis.__startTilesSmoke.seconds*1000;
    phoneApp.tick(seconds);return seconds;
  })()`);
  const card=(tag,template,extra={})=>({tag,template,starts_at:base,expires_at:null,badge:null,...extra});
  const photos=fixture.photos.map(photo=>photo.id);
  const templates=[
    {Text:{title:"Today",body:"Six retained tile templates"}},
    {Flip:{title:"Front",body:"First side",back_title:"Back",back_body:"Second side"}},
    {List:{title:"Take along",lines:["Keys","Notebook","Headphones"]}},
    {Progress:{title:"Trip planning",body:"Ready for the weekend",percent:65}},
    {Photo:{photo:photos[0],title:"Local photo",body:"Saved on this phone"}},
    {Cycle:{photos:photos.slice(0,2),title:"Favorite places"}},
  ];
  fixture.tiles=leaves.map((tile,index)=>({...tile,size:"Medium",appearance:structuredClone(appearance),notifications:[card(`template.${index}`,templates[index],index===0?{badge:{Count:105}}:{})]}));
  Object.assign(fixture.settings,{more_tiles:false,lock_timeout:0,live_tiles:true,reduced_motion:false,battery_saver:false,wallpaper:""});
  fixture.alarms.forEach(alarm=>{alarm.enabled=false;});
  const load=async data=>{await page.evaluate(`phoneApp.importData(${JSON.stringify(JSON.stringify(data))})`);await isRoute("Start");};
  const settled=()=>page.evaluate("new Promise(resolve=>requestAnimationFrame(()=>requestAnimationFrame(()=>resolve(true))))");
  const tick=async seconds=>{await page.evaluate(`globalThis.__startTilesSmoke.seconds=${seconds};phoneApp.tick(${seconds})`);await settled();};
  const named=async name=>(await snapshot()).elements.filter(element=>element.name===name);
  const text=async(name,value)=>until(async()=>{const found=await named(name);return found.length===1&&found[0].text===value;},`${name}: ${value}`);
  const menuClick=async match=>{
    const item=await until(async()=>{
      const view=await snapshot();const candidates=view.elements.filter(element=>/(?:Button|ToggleSwitch|CheckBox|PivotHeaderItem)$/.test(element.type)&&match(element));
      const visible=candidates.filter(element=>element.hit);assert(visible.length<=1,"Tile command must be unique");if(visible[0])return visible[0];
      const menu=view.elements.find(element=>element.name==="TileMenuSurface");
      if(menu){const target=candidates[0];await page.wheel(menu.x+menu.width/2,menu.y+menu.height/2,0,target&&target.y<menu.y?-240:240);await settled();}
      return false;
    },"reachable tile command");
    await page.click(item.x+item.width/2,item.y+item.height/2);
  };
  const menuName=name=>menuClick(element=>element.name===name);
  const actions=async id=>{
    const tile=await control(element=>element.name===`Tile-${id}`,`tile ${id}`);
    const x=tile.x+tile.width/2,y=tile.y+tile.height/2;
    await page.send("Input.dispatchMouseEvent",{type:"mousePressed",x,y,button:"right",buttons:2,clickCount:1,pointerType:"mouse"});
    await page.send("Input.dispatchMouseEvent",{type:"mouseReleased",x,y,button:"right",buttons:0,clickCount:1,pointerType:"mouse"});
    await clickName(`TileActions-${id}`);
  };
  try {
    await check("six native live-card templates, capped badge and finite flip render",async()=>{
      await load(fixture);
      for(const tile of fixture.tiles)await until(async()=>(await named(`TileCard-${tile.id}`)).length===1,`mounted card ${tile.id}`);
      await text(`TileBadge-${leaves[0].id}`,"99+");
      await text(`TileProgress-${leaves[3].id}`,"65%");
      assert((await named(`TileProgressBar-${leaves[3].id}`)).some(element=>element.type.endsWith("ProgressBar")),"Progress uses the real native range control");
      await text(`TileCardTitle-${leaves[1].id}`,"Front");
      await page.evaluate("new Promise(resolve=>setTimeout(()=>resolve(true),400))");
      await capture("start-live-templates");
      await tick(base+4);await text(`TileCardTitle-${leaves[1].id}`,"Back");
    });

    await check("queued cards rotate and scheduled expiry survives reduced motion without saving",async()=>{
      await tick(base);
      const queued=structuredClone(fixture);queued.tiles=queued.tiles.slice(0,1);
      queued.tiles[0].notifications=[card("first",{Text:{title:"First queued card",body:"Retained content"}}),card("newest",{Text:{title:"Newest queued card",body:"Expires at the boundary"}},{expires_at:base+12})];
      await load(queued);await text(`TileCardTitle-${leaves[0].id}`,"Newest queued card");
      const persisted=await state();
      await page.evaluate(`(()=>{const smoke=globalThis.__startTilesSmoke;smoke.storageSet=Storage.prototype.setItem;smoke.writes=0;Storage.prototype.setItem=function(key,value){if(key==='ferroui.windows-phone.v1')smoke.writes++;return smoke.storageSet.call(this,key,value);};})()`);
      await tick(base+8);await text(`TileCardTitle-${leaves[0].id}`,"First queued card");
      await tick(base+12);await text(`TileCardTitle-${leaves[0].id}`,"First queued card");
      assert.deepEqual(await state(),persisted,"Rotation and expiry must not alter the document");
      assert.equal(await page.evaluate("globalThis.__startTilesSmoke.writes"),0,"Live deadlines must not issue storage writes");
      await page.evaluate("Storage.prototype.setItem=globalThis.__startTilesSmoke.storageSet;delete globalThis.__startTilesSmoke.storageSet");

      const scheduled=structuredClone(queued);scheduled.settings.reduced_motion=true;
      scheduled.tiles[0].notifications=[card("scheduled",{Text:{title:"Scheduled with reduced motion",body:"Expires without an animation"}},{starts_at:base+20,expires_at:base+24})];
      await load(scheduled);assert.equal((await named(`TileCard-${leaves[0].id}`)).length,0);
      await tick(base+20);await text(`TileCardTitle-${leaves[0].id}`,"Scheduled with reduced motion");
      await capture("start-scheduled-card-reduced-motion");
      await tick(base+24);await until(async()=>(await named(`TileCard-${leaves[0].id}`)).length===0,"card expires under reduced motion");
    });

    await check("native tile customization, folder creation and dissolution preserve full records",async()=>{
      const data=structuredClone(fixture);data.settings.reduced_motion=true;data.tiles=data.tiles.slice(0,2);
      await load(data);const id=data.tiles[0].id;
      await actions(id);await menuName("TileMenu-customize");await menuName("TileMenu-custom-color");
      await edit("FormField0","#145A72");await clickLabel("save");
      await actions(id);await menuName("TileMenu-customize");await menuName("TileMenu-tile-name");
      await edit("FormField0","my communications");await clickLabel("save");
      await actions(id);await menuName("TileMenu-customize");
      await menuClick(element=>element.automationName==="show tile name, on");
      await menuClick(element=>element.automationName==="show badge, on");
      await menuName("TileMenu-done");
      const saved=(await state()).tiles.find(tile=>tile.id===id);
      assert.deepEqual(saved.appearance.background,{Color:"#145A72"});assert.equal(saved.appearance.label,"my communications");
      assert.equal(saved.appearance.show_label,false);assert.equal(saved.appearance.show_badge,false);
      assert.equal((await named(`TileLabel-${id}`)).length,0);assert.equal((await named(`TileBadge-${id}`)).length,0);
      const stored=await page.evaluate("localStorage.getItem('ferroui.windows-phone.v1')");
      assert.deepEqual(JSON.parse(stored).tiles.find(tile=>tile.id===id).appearance,saved.appearance,"Preferences must be committed to storage");
      await capture("start-customized-tiles");
      const before=(await state()).tiles;
      await actions(id);await menuName("TileMenu-create-folder");await menuClick(element=>element.automationName===data.tiles[1].label);
      await edit("FormField0","communications");await clickLabel("save");
      await until(async()=>{const tiles=(await state()).tiles;return tiles.length===1&&tiles[0].folder_tiles.length===2;},"folder created through its tile menu");
      const folder=(await state()).tiles[0];
      assert.deepEqual([...folder.folder_tiles].sort((a,b)=>a.id-b.id),[...before].sort((a,b)=>a.id-b.id));
      await capture("start-customized-folder");
      await actions(folder.id);await menuName("TileMenu-dissolve-folder");await clickLabel("confirm");
      await until(async()=>(await state()).tiles.length===2,"folder children returned to Start");
      assert.deepEqual([...(await state()).tiles].sort((a,b)=>a.id-b.id),[...before].sort((a,b)=>a.id-b.id));
    });
  } catch(error) {await capture("start-customization-failure").catch(()=>{});throw error;}
  finally {
    await page.evaluate(`(()=>{const smoke=globalThis.__startTilesSmoke;if(smoke){Date.now=smoke.dateNow;if(smoke.storageSet)Storage.prototype.setItem=smoke.storageSet;delete globalThis.__startTilesSmoke;}phoneApp.tick(Date.now()/1000);})()`);
    await load(original);
  }
}
