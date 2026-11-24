import assert from 'node:assert/strict';

// The global UI harness supplies local 256px PNG fixtures before navigation.
// No public OSM scan or external map/directions app is launched by these tests.
export async function verifyMapsExpansion(ctx) {
  const {page,check,state,until,capture,clickLabel,clickName,edit,isRoute,textPresent,reload,snapshot,control,reveal,centre}=ctx;
  const original=await state();
  const fixture=structuredClone(original);
  Object.assign(fixture.settings,{lock_timeout:0,reduced_motion:true});
  fixture.alarms=[]; fixture.events=[]; delete fixture.app_data['phone.maps'];
  const load=async data=>{await page.evaluate(`phoneApp.importData(${JSON.stringify(JSON.stringify(data))})`);await isRoute('Start');};
  const open=async()=>{await page.evaluate('phoneApp.openApp("Maps")');await isRoute({App:'Maps'});};
  const settled=()=>page.evaluate('new Promise(resolve=>requestAnimationFrame(()=>requestAnimationFrame(()=>resolve(true))))');
  const coordinates=async()=>(await snapshot()).elements.find(element=>element.name==='MapCoordinates')?.text;
  const zoom=value=>Number(/zoom ([\d.]+)/.exec(value)?.[1]);
  const touch=async(type,points=[])=>{
    const result=await page.send('Input.dispatchTouchEvent',{type,touchPoints:points.map(([x,y],id)=>({x,y,id,radiusX:3,radiusY:3,force:1}))});
    assert(!result?.error,JSON.stringify(result));await settled();
  };
  const drag=async(from,to)=>{
    await page.send('Input.dispatchMouseEvent',{type:'mouseMoved',x:from[0],y:from[1],buttons:0,pointerType:'mouse'});
    await page.send('Input.dispatchMouseEvent',{type:'mousePressed',x:from[0],y:from[1],button:'left',buttons:1,clickCount:1,pointerType:'mouse'});
    for(let step=1;step<=5;step++){
      const fraction=step/5;
      await page.send('Input.dispatchMouseEvent',{type:'mouseMoved',x:from[0]+(to[0]-from[0])*fraction,y:from[1]+(to[1]-from[1])*fraction,buttons:1,pointerType:'mouse'});
      await settled();
    }
    await page.send('Input.dispatchMouseEvent',{type:'mouseReleased',x:to[0],y:to[1],button:'left',buttons:0,clickCount:1,pointerType:'mouse'});await settled();
  };
  let touchEnabled=false;
  await load(fixture);
  try {
    await check('native map tiles render with attribution and bounded visible images; pan and zoom preserve data',async()=>{
      await open();
      await until(async()=>{
        const elements=(await snapshot()).elements;
        const tiles=elements.filter(element=>element.name?.startsWith('MapTile-'));
        const images=elements.filter(element=>element.name?.startsWith('MapTileImage-')&&element.type.endsWith('Image'));
        return tiles.length>0&&tiles.length<=16&&images.length===tiles.length&&!elements.some(element=>element.name==='MapLoading');
      },'fixture PNGs decoded into the visible native Image controls');
      const canvas=await control(element=>element.name==='MapCanvas','interactive map surface');
      const attribution=await control(element=>element.name==='MapAttribution','visible OpenStreetMap attribution');
      assert(attribution.x>=canvas.x&&attribution.y>=canvas.y&&attribution.x+attribution.width<=canvas.x+canvas.width+1&&attribution.y+attribution.height<=canvas.y+canvas.height+1,'Attribution remains inside the map surface');
      const data=await state();const before=await coordinates();const [x,y]=centre(canvas);
      await drag([x+35,y],[x-35,y+16]);
      await until(async()=>(await coordinates())!==before,'native pointer drag changes the viewport');
      const panned=await coordinates();await clickName('MapZoomIn');
      await until(async()=>zoom(await coordinates())===zoom(panned)+1,'native zoom button changes zoom');
      await clickName('MapZoomOut');await until(async()=>zoom(await coordinates())===zoom(panned),'zoom out returns to the previous level');
      assert.deepEqual(await state(),data,'Exploration does not silently save camera or coordinates');
      await capture('maps-native-tiles-pan-zoom');
    });
    await check('trusted map pinch, pointer cancellation and route cleanup stay within the map',async()=>{
      const canvas=await control(element=>element.name==='MapCanvas','pinch map surface');const [x,y]=centre(canvas);const before=await coordinates();
      await page.send('Emulation.setTouchEmulationEnabled',{enabled:true,maxTouchPoints:2});touchEnabled=true;
      await touch('touchStart',[[x-24,y],[x+24,y]]);
      for(let spread=30;spread<=54;spread+=8)await touch('touchMove',[[x-spread,y],[x+spread,y]]);
      await touch('touchEnd');
      await until(async()=>zoom(await coordinates())>zoom(before)+0.8,'two contacts increase the geographic zoom');
      await isRoute({App:'Maps'});
      assert((await snapshot()).elements.some(element=>element.name==='MapCanvas'),'Pinch must not change the app pivot');
      const current=await control(element=>element.name==='MapCanvas','map after pinch');const [cx,cy]=centre(current);
      await touch('touchStart',[[cx,cy]]);await touch('touchMove',[[cx+15,cy+8]]);await touch('touchCancel');
      const cancelled=await coordinates();await touch('touchStart',[[cx,cy]]);await touch('touchMove',[[cx-32,cy+16]]);await touch('touchEnd');
      await until(async()=>(await coordinates())!==cancelled,'a new drag works after pointer cancellation');
      // Remove the actual mounted map while it owns a pointer. Releasing that
      // pointer later cannot activate a dropped pin or revive its old scope.
      await touch('touchStart',[[cx,cy]]);await touch('touchMove',[[cx+10,cy]]);
      await page.evaluate('phoneApp.openApp("Calculator")');await isRoute({App:'Calculator'});
      await touch('touchCancel');await settled();
      assert(!(await snapshot()).elements.some(element=>element.name==='MapCanvas'||element.name==='MapCoordinates'),'The old retained viewport is removed');
      await page.send('Emulation.setTouchEmulationEnabled',{enabled:false});touchEnabled=false;
      await open();await control(element=>element.name==='MapSearch','Map overview after route cleanup');
      await clickName('MapZoomIn');await control(element=>element.name==='MapCanvas','remounted map remains interactive');
      await capture('maps-pinch-cancel-and-remount');
    });
    await check('native Maps saves coordinates, home shortcuts and directions preferences',async()=>{
      await open();await clickLabel('add place');
      await edit('FormField0','Raciborz home');await edit('FormField1','Example address');await edit('FormField2','50.091, 18.219');await clickLabel('save');
      const id=await until(async()=>(await state()).app_data['phone.maps']?.places.find(place=>place.name==='Raciborz home')?.id,'saved map place');
      await clickLabel('more');await clickLabel('set as home');
      await until(async()=>(await state()).app_data['phone.maps']?.home===id,'home shortcut');
      await clickLabel('×');
      await until(async()=>!(await snapshot()).elements.some(element=>element.text==='Home updated.'),'home status dismissed through its native control');
      await clickLabel('directions');
      await textPresent('directions','native directions page');
      await edit('MapDestination','Main station');await clickLabel('walking');await clickLabel('from home');
      await until(async()=>(await snapshot()).elements.some(element=>element.name==='MapOrigin'&&element.text==='50.091000, 18.219000'),'origin from saved coordinates');
      await clickLabel('swap');
      await until(async()=>(await snapshot()).elements.some(element=>element.name==='MapDestination'&&element.text==='50.091000, 18.219000'),'swapped destination');
      await until(async()=>(await snapshot()).elements.some(element=>element.name==='MapOrigin'&&element.text==='Main station'),'swapped origin');
      await textPresent('from','origin label remains visible with a populated field');
      await textPresent('to','destination label remains visible with a populated field');
      assert.equal((await state()).app_data['phone.maps'].travel_mode,'walking');
      await capture('maps-saved-place-directions');
      await reload();await open();await clickLabel('home');
      await textPresent('Raciborz home','restored home shortcut');
      assert.equal((await state()).app_data['phone.maps'].places[0].coordinates.latitude,50.091);
      await clickLabel('more');await clickLabel('pin place to Start');
      const link=await until(async()=>(await state()).app_links.find(link=>link.app==='phone.maps'&&link.path===`place/${id}`),'canonical saved-place Start link');
      const tile=(await state()).tiles.find(tile=>tile.target?.AppLink===link.id);assert(tile,'Pinned place has a real Start tile');
      await clickName('HardwareStart');await isRoute('Start');
      const target=await reveal(element=>element.name===`Tile-${tile.id}`,'pinned saved place',element=>element.name==='StartScroll','Start scroll viewer');
      await page.click(...centre(target));await isRoute({App:'Maps'});
      await textPresent('Raciborz home','Start tile activates the exact saved place');
      await capture('maps-saved-place-start-activation');
      await clickLabel('more');await clickLabel('delete place');await clickLabel('confirm');
      const deleted=await state();const saved=deleted.app_data['phone.maps'];
      assert.equal(saved.places.length,0);assert.equal(saved.home,null);
      assert(!deleted.app_links.some(item=>item.id===link.id),'Deleting the saved place removes its obsolete Start link');
      assert(!deleted.tiles.some(item=>item.id===tile.id),'Deleting the saved place removes its Start tile');
      await capture('maps-deleted-place-clears-shortcut');
    });
    await check('native Maps coordinate search and recent history need no network lookup',async()=>{
      await edit('MapSearch','50.1, 18.2');await clickLabel('search');
      await textPresent('50.100000, 18.200000','coordinate search result');
      assert.equal((await state()).app_data['phone.maps'].recent[0],'50.1, 18.2');
      await clickName('HardwareBack');await clickLabel('recent');
      await textPresent('50.1, 18.2','recent map query');
      await clickLabel('more');await clickLabel('clear recent searches');await clickLabel('confirm');
      assert.deepEqual((await state()).app_data['phone.maps'].recent,[]);
      await capture('maps-coordinate-search-and-history');
    });
  } catch(error) {
    await capture('maps-expansion-failure').catch(()=>{});
    throw error;
  } finally {
    if(touchEnabled){await touch('touchCancel').catch(()=>{});await page.send('Emulation.setTouchEmulationEnabled',{enabled:false});}
    await load(original);
  }
}
