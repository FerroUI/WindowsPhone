import assert from 'node:assert/strict';

// Saved-place flows stay local. No external map or directions app is launched.
export async function verifyMapsExpansion(ctx) {
  const {page,check,state,until,capture,clickLabel,clickName,edit,isRoute,textPresent,reload,snapshot}=ctx;
  const original=await state();
  const fixture=structuredClone(original);
  Object.assign(fixture.settings,{lock_timeout:0,reduced_motion:true});
  fixture.alarms=[]; fixture.events=[]; delete fixture.app_data['phone.maps'];
  const load=async data=>{await page.evaluate(`phoneApp.importData(${JSON.stringify(JSON.stringify(data))})`);await isRoute('Start');};
  const open=async()=>{await page.evaluate('phoneApp.openApp("Maps")');await isRoute({App:'Maps'});};
  await load(fixture);
  try {
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
      await clickLabel('more');await clickLabel('delete place');await clickLabel('save');
      const saved=(await state()).app_data['phone.maps'];
      assert.equal(saved.places.length,0);assert.equal(saved.home,null);
      await capture('maps-deleted-place-clears-shortcut');
    });
    await check('native Maps coordinate search and recent history need no network lookup',async()=>{
      await edit('MapSearch','50.1, 18.2');await clickLabel('search');
      await textPresent('50.100000, 18.200000','coordinate search result');
      assert.equal((await state()).app_data['phone.maps'].recent[0],'50.1, 18.2');
      await clickName('HardwareBack');await clickLabel('recent');
      await textPresent('50.1, 18.2','recent map query');
      await clickLabel('more');await clickLabel('clear recent searches');await clickLabel('save');
      assert.deepEqual((await state()).app_data['phone.maps'].recent,[]);
      await capture('maps-coordinate-search-and-history');
    });
  } catch(error) {
    await capture('maps-expansion-failure').catch(()=>{});
    throw error;
  } finally {await load(original);}
}
