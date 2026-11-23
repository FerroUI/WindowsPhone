import assert from 'node:assert/strict';

// Deterministic service fixtures exercise the real Rust/JS task bridge. They do
// not send requests, play remote audio, open permissions or share personal data.
export async function verifyConnectedApps(ctx) {
  const {page,check,state,until,clickLabel,clickName,edit,isRoute,capture,reload,snapshot}=ctx;
  const original=await state();
  const fixture=structuredClone(original);
  Object.assign(fixture.settings,{lock_timeout:0,reduced_motion:true,dark:true});
  fixture.app_data||={};
  const podcasts='org.ferroui.podcasts',wallet='org.ferroui.wallet';
  for(const id of ['phone.weather',podcasts,wallet])delete fixture.app_data[id];
  fixture.disabled_extensions=(fixture.disabled_extensions||[]).filter(id=>![podcasts,wallet].includes(id));
  const load=async data=>{await page.evaluate(`phoneApp.importData(${JSON.stringify(JSON.stringify(data))})`);await isRoute('Start');};
  const launch=async(id,path='')=>{assert.equal(await page.evaluate(`phoneApp.launchApp(${JSON.stringify(id)},${JSON.stringify(path)})`),true);await isRoute({Extension:{app:id,path}});};
  const saved=async id=>(await state()).app_data?.[id];
  const weather={current:{temperature_2m:10,apparent_temperature:8,relative_humidity_2m:75,wind_speed_10m:12,weather_code:3},hourly:{time:['2026-11-23T10:00','2026-11-23T11:00'],temperature_2m:[10,11],precipitation_probability:[20,30],weather_code:[3,2]},daily:{time:['2026-11-23','2026-11-24'],temperature_2m_min:[2,3],temperature_2m_max:[12,13],precipitation_probability_max:[20,40],weather_code:[3,2]}};
  const feed='<rss version="2.0"><channel><title>Test Podcast</title><description>Local verification feed</description><item><title>First episode</title><description>A deterministic episode.</description><enclosure url="https://example.org/episode.mp3" type="audio/mpeg"/></item></channel></rss>';
  const mock=async()=>page.evaluate(`(()=>{
    globalThis.phoneTestFetch=globalThis.fetch;
    globalThis.fetch=async(input,options)=>{
      const url=new URL(typeof input==='string'?input:input.url);
      if(url.hostname==='geocoding-api.open-meteo.com')return new Response(${JSON.stringify(JSON.stringify({results:[{id:3089424,name:'Raciborz',country:'Poland',latitude:50.091,longitude:18.219}]}))},{headers:{'content-type':'application/json'}});
      if(url.hostname==='api.open-meteo.com')return new Response(${JSON.stringify(JSON.stringify(weather))},{headers:{'content-type':'application/json'}});
      if(url.href==='https://example.org/podcast.xml')return new Response(${JSON.stringify(feed)},{headers:{'content-type':'application/rss+xml'}});
      return globalThis.phoneTestFetch(input,options);
    };return true;
  })()`);
  const unmock=async()=>page.evaluate('if(globalThis.phoneTestFetch){globalThis.fetch=globalThis.phoneTestFetch;delete globalThis.phoneTestFetch;}');
  await load(fixture);
  try{
    await check('Weather searches real task fixtures, saves hourly/daily readings and preserves units across reload',async()=>{
      await mock();await page.evaluate('phoneApp.openApp("Weather")');await isRoute({App:'Weather'});
      await clickLabel('add city');await edit('WeatherCitySearch','Raciborz');await clickLabel('search cities');await clickLabel('Raciborz, Poland');
      await until(async()=>!!(await saved('phone.weather'))?.cities[0]?.forecast,'forecast committed');
      assert.equal((await saved('phone.weather')).cities[0].forecast.hours.length,2);
      await clickLabel('fahrenheit');assert.equal((await saved('phone.weather')).fahrenheit,true);
      await clickLabel('hourly');await capture('weather-hourly-forecast');await clickLabel('daily');await capture('weather-daily-forecast');
      await unmock();await reload();await page.evaluate('phoneApp.openApp("Weather")');await isRoute({App:'Weather'});
      assert.equal((await saved('phone.weather')).cities[0].forecast.temperature,10);
      assert.equal((await saved('phone.weather')).fahrenheit,true);await capture('weather-saved-city');
    });
    await check('Podcasts imports an HTTPS feed through SDK, persists played state and pins a deep route',async()=>{
      await mock();await launch(podcasts);await clickLabel('subscribe');await edit('FormField0','https://example.org/podcast.xml');await clickLabel('subscribe');
      await until(async()=>(await saved(podcasts))?.shows.length===1,'podcast saved');
      const id=(await saved(podcasts)).shows[0].id;await isRoute({Extension:{app:podcasts,path:`show/${id}`}});
      await clickLabel('First episode');await clickLabel('mark played');assert.equal((await saved(podcasts)).shows[0].episodes[0].played,true);
      await capture('podcasts-episode-detail');await clickName('HardwareBack');await clickLabel('pin podcast');
      assert((await state()).app_links.some(link=>link.app===podcasts&&link.path===`show/${id}`));
      await unmock();await reload();await launch(podcasts,`show/${id}`);await capture('podcasts-persisted-collection');
      assert.equal((await saved(podcasts)).shows[0].episodes[0].played,true);
    });
    await check('Wallet creates, validates, displays Code 39, favorites and restores independently',async()=>{
      await launch(wallet);await clickLabel('add');await edit('FormField0','Library card');await edit('FormField1','City Library');await edit('FormField2','ABC123');await edit('FormField3','2027-12-31');await clickLabel('save');
      await until(async()=>(await saved(wallet))?.cards.length===1,'wallet card saved');const id=(await saved(wallet)).cards[0].id;
      await isRoute({Extension:{app:wallet,path:`card/${id}`}});await clickLabel('favorite');await clickLabel('more');await clickLabel('use Code 39 barcode');await clickLabel('more');
      assert.equal((await saved(wallet)).cards[0].barcode,'Code39');assert.equal((await saved(wallet)).cards[0].favorite,true);
      await capture('wallet-membership-barcode');
      const view=await snapshot();const barcode=view.elements.find(element=>element.name==='WalletBarcode');const screen=view.elements.find(element=>element.name==='PhoneScreen');assert(barcode&&screen);assert(barcode.x>=screen.x&&barcode.x+barcode.width<=screen.x+screen.width,'barcode quiet zones fit phone');
      await clickLabel('pin card');assert((await state()).app_links.some(link=>link.app===wallet&&link.path===`card/${id}`));
      await reload();await launch(wallet,`card/${id}`);assert.equal((await saved(wallet)).cards[0].code,'ABC123');await capture('wallet-persisted-card');
      const size=await page.evaluate('({width:innerWidth,height:innerHeight})');
      try{
        await page.resize(320,740);await until(async()=>(await snapshot()).width===320,'320-pixel phone layout');
        await clickLabel('edit');await edit('FormField2','12345678901234567890');await clickLabel('save');
        const narrow=await snapshot();const code=narrow.elements.find(element=>element.name==='WalletBarcode');const phone=narrow.elements.find(element=>element.name==='PhoneScreen');
        assert(code&&phone&&code.x>=phone.x&&code.x+code.width<=phone.x+phone.width,'full 20-character barcode fits narrow phone');
        await capture('wallet-narrow-barcode');await launch(wallet);await clickLabel('archived');
        await until(async()=>{const header=(await snapshot()).elements.find(element=>element.automationName==='archived');return header?.hit;},'selected archived header remains reachable');
        await capture('wallet-narrow-pivot');
      }finally{await page.resize(size.width,size.height);await until(async()=>(await snapshot()).width===size.width,'original phone layout restored');}
    });
  }finally{await unmock();await load(original);}
}
