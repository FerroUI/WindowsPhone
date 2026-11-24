import assert from "node:assert/strict";

// All web content in this test is served by the isolated smoke server. Native
// address-entry, tab, history and favorite commands drive the persisted model.
export async function verifyBrowserRefinements(ctx) {
  const {page,check,state,until,capture,clickName,clickLabel,textPresent,edit,isRoute,home}=ctx;
  const original=await state();
  const fixture=structuredClone(original);
  fixture.settings.lock_timeout=0;
  fixture.browser={tabs:[{id:1,journal:[],position:0}],active:1,history:[],next_tab:2,address_button:"Tabs"};
  fixture.bookmarks=[];
  fixture.tiles=fixture.tiles.filter(tile=>!tile.target?.Bookmark);
  const base=await page.evaluate("location.href");
  const first=new URL("icon.svg?phone-smoke=first",base).href;
  const second=new URL("icon.svg?phone-smoke=second",base).href;
  const third=new URL("icon.svg?phone-smoke=third",base).href;
  const current=async()=>{const data=(await state()).browser;return data.tabs.find(tab=>tab.id===data.active);};
  const at=async url=>until(async()=>{const tab=await current();return (tab.journal[tab.position]||"")===url;},`active browser URL ${url}`);
  const navigate=async url=>{await edit("AddressInput",url);await clickLabel("Go to address");await at(url);};
  const load=async data=>{await page.evaluate(`phoneApp.importData(${JSON.stringify(JSON.stringify(data))})`);await isRoute("Start");};
  await load(fixture);
  try {
    await page.evaluate('phoneApp.openApp("Internet Explorer")');
    await isRoute({App:"InternetExplorer"});
    await check("native browser tabs retain independent navigation journals",async()=>{
      await navigate(first);await navigate(second);
      await clickName("HardwareBack");await at(first);
      await clickLabel("Browser menu");await clickLabel("forward");await at(second);
      await clickName("BrowserShortcut");await textPresent("tabs");
      await clickLabel("+ new tab");await at("");
      await until(()=>page.evaluate("!document.querySelector('#browser-surface iframe')"),"blank tab releases the previous embedded document");
      await navigate(third);
      const data=(await state()).browser;
      assert.equal(data.tabs.length,2);
      assert.deepEqual(data.tabs.find(tab=>tab.id===1).journal,[first,second]);
      await clickName("BrowserShortcut");await capture("browser-native-tabs");
      await clickName("BrowserTab-1");await at(second);
      await clickName("BrowserShortcut");await clickName("CloseBrowserTab-2");
      await until(async()=>(await state()).browser.tabs.length===1,"inactive browser tab removed");
      await clickName("BrowserTab-1");
    });
    await check("native favorites and configurable address shortcut persist",async()=>{
      await clickLabel("Browser menu");await clickLabel("add to favorites");
      await edit("FormField0","Local native browser fixture");await clickLabel("save");
      await until(async()=>(await state()).bookmarks.some(item=>item.title==="Local native browser fixture"&&item.url===second),"favorite saved");
      await clickLabel("Browser menu");await clickLabel("settings");
      await clickLabel("favorites");
      await until(async()=>(await state()).browser.address_button==="Favorites","address shortcut preference");
      await clickName("HardwareBack");await clickName("BrowserShortcut");
      await textPresent("favorites");await capture("browser-native-favorites");
      const stored=await page.evaluate("JSON.parse(localStorage.getItem('ferroui.windows-phone.v1'))");
      assert.equal(stored.browser.address_button,"Favorites");
      assert(stored.bookmarks.some(item=>item.title==="Local native browser fixture"));
      await clickName("HardwareBack");
    });
    await check("native browser history clears independently of open tabs and favorites",async()=>{
      await clickLabel("Browser menu");await clickLabel("history");
      await textPresent("history");await capture("browser-native-history");
      assert.equal((await state()).browser.history.length,3);
      await clickLabel("delete history");await clickLabel("confirm");
      await until(async()=>(await state()).browser.history.length===0,"address-bar history cleared");
      assert.equal((await state()).browser.tabs.length,1);
      assert.equal((await state()).bookmarks.length,1);
      await clickName("HardwareBack");await at(second);
      await home();
    });
  } catch(error) {
    await capture("browser-failure").catch(()=>{});
    throw error;
  } finally {
    await load(original);
  }
}
