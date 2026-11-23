import assert from "node:assert/strict";

// Isolated document fixtures; all scrolling, presses, jumps, searches and picker
// commands use trusted browser input against the mounted native control tree.
export async function verifyShellParity(ctx) {
  const {page,check,state,snapshot,until,capture,centre,control,clickName,
    clickLabel,edit,isRoute}=ctx;
  const original=await state();
  const fixture=structuredClone(original);
  const source=fixture.tiles.find(tile=>tile.app==="Phone"&&!tile.target&&!tile.children.length);
  assert(source,"Shell verification requires a Phone tile");
  Object.assign(fixture.settings,{lock_timeout:0,wallpaper:"mountains",live_tiles:false,
    reduced_motion:false,battery_saver:false,high_contrast:false,more_tiles:false,text_scale:1,dark:true});
  fixture.alarms.forEach(alarm=>{alarm.enabled=false;});
  fixture.installed=["Alarms","BatterySaver","Calculator","Calendar","Camera","Cortana",
    "DataSense","Files","Games","InternetExplorer","Maps","Messaging","Music","Office",
    "OneNote","Outlook","People","Phone","Photos","Settings","Store","Video","Weather"];
  fixture.tiles=Array.from({length:18},(_,index)=>({...structuredClone(source),
    id:fixture.next_id++,size:"Medium",app:index===1?"Office":"Phone",
    label:index===1?"Office":`Shell phone ${index+1}`,target:null,children:[],folder_tiles:[]}));
  const first=fixture.tiles[0],opaque=fixture.tiles[1];
  const frame=()=>page.evaluate("new Promise(resolve=>requestAnimationFrame(()=>requestAnimationFrame(()=>resolve(true))))");
  const named=(view,name)=>view.elements.find(element=>element.name===name);
  const close=(actual,expected,message,tolerance=0.6)=>
    assert(Math.abs(actual-expected)<=tolerance,`${message}: expected ${expected}, received ${actual}`);
  const geometry=element=>[element.x,element.y,element.width,element.height];
  const load=async data=>{
    await page.evaluate(`phoneApp.importData(${JSON.stringify(JSON.stringify(data))})`);
    await isRoute("Start");await frame();
    await until(async()=>{
      const view=await snapshot(),screen=named(view,"PhoneScreen"),scroll=named(view,"StartScroll");
      return screen&&scroll&&Math.abs(scroll.x-screen.x)<0.1;
    },"Start entrance has reached its native resting position");
  };
  const planes=view=>{
    const scroll=named(view,"StartScroll");
    const all=view.elements.filter(element=>element.name?.startsWith("StartWallpaper-"));
    assert.equal(all.length,fixture.tiles.length-1,"Every compatible tile must mount its shared image plane");
    assert(!named(view,`StartWallpaper-${opaque.id}`),"The branded Office tile must remain opaque");
    for(const plane of all){
      close(plane.x,all[0].x,"Transparent tile images share the same native x origin");
      close(plane.y,all[0].y,"Transparent tile images share the same native y origin");
      close(plane.width,scroll.width-28,"Wallpaper uses the native Start content width");
      assert(plane.y<=scroll.y+0.6&&plane.y+plane.height>=scroll.y+scroll.height-0.6,
        "The common image plane must cover the complete visible Start viewport");
    }
    return all[0];
  };
  const bottom=async()=>{
    await page.wheel(...centre(await control(element=>element.name==="StartScroll","Start scroll viewer")),0,100000);
    await control(element=>element.name===`Tile-${fixture.tiles.at(-1).id}`,"last tile in the tall Start layout");
  };
  const shortFixture=()=>{const data=structuredClone(fixture);data.tiles=data.tiles.slice(0,2);return data;};

  try {
    await check("native transparent tiles share a bounded wallpaper plane across tall Start layouts",async()=>{
      await load(fixture);
      const before=planes(await snapshot());
      const document=await state();
      const persisted=await page.evaluate("localStorage.getItem('ferroui.windows-phone.v1')");
      await bottom();
      const after=planes(await snapshot()),scroll=named(await snapshot(),"StartScroll");
      assert(before.y-after.y>1,"Enabled wallpaper motion must produce a bounded pan");
      assert(before.y-after.y<=scroll.height*0.25+0.6,"Pan must remain bounded by the viewport, not the tile count");
      assert.deepEqual(await state(),document,"Scrolling wallpaper must not mutate the phone document");
      assert.equal(await page.evaluate("localStorage.getItem('ferroui.windows-phone.v1')"),persisted);
      await capture("shell-wallpaper-tall-layout-bottom");
      for(const preference of ["reduced_motion","battery_saver"]){
        const paused=structuredClone(fixture);paused.settings[preference]=true;
        await load(paused);const resting=planes(await snapshot());
        await bottom();const scrolled=planes(await snapshot());
        close(scrolled.y,resting.y,`${preference} keeps the image stationary on screen`);
      }
    });

    await check("native tile projection responds to press location while retaining Button bounds",async()=>{
      const data=shortFixture();await load(data);
      const press=async(fraction,enabled)=>{
        const button=await control(element=>element.name===`Tile-${first.id}`,"Phone tile");
        const before=named(await snapshot(),`TileVisual-${first.id}`);
        assert(before,"The native tile content must be mounted");
        const x=button.x+button.width*fraction,y=button.y+button.height*0.15;
        await page.mouseMove(x,y);await page.mouseDown(x,y);
        let delta;
        try {
          await frame();
          const view=await snapshot(),pressed=named(view,`Tile-${first.id}`),visual=named(view,`TileVisual-${first.id}`);
          assert(pressed&&visual,"A native press must retain both controls");
          geometry(pressed).forEach((value,index)=>close(value,geometry(button)[index],"Button input bounds remain unchanged"));
          delta=[visual.x-before.x,visual.y-before.y];
          if(enabled)assert(Math.hypot(...delta)>0.25,"The content must project during a corner press");
          else assert(Math.hypot(...delta)<0.1,"Motion preferences must suppress decorative press projection");
        } finally {await page.mouseUp(x,y);}
        await isRoute({App:"Phone"});
        await clickName("HardwareBack");await isRoute("Start");
        await until(async()=>{
          const view=await snapshot(),tile=named(view,`Tile-${first.id}`),visual=named(view,`TileVisual-${first.id}`);
          const screen=named(view,"PhoneScreen"),scroll=named(view,"StartScroll");
          return tile&&visual&&screen&&scroll&&Math.abs(scroll.x-screen.x)<0.1&&
            Math.abs(tile.x-visual.x)<0.1&&Math.abs(tile.y-visual.y)<0.1;
        },"Released tile content returns to its native resting bounds");
        return delta;
      };
      const left=await press(0.15,true),right=await press(0.85,true);
      assert(Math.hypot(right[0]-left[0],right[1]-left[1])>0.05,
        "Different press locations must produce different projections");
      for(const preference of ["reduced_motion","battery_saver"]){
        const paused=structuredClone(data);paused.settings[preference]=true;
        await load(paused);await press(0.15,false);
      }
      await capture("shell-tile-native-resting-bounds");
    });

    await check("native alphabet jump, sticky push-off and search use actual app-list geometry",async()=>{
      const data=shortFixture();data.settings.reduced_motion=true;await load(data);
      await clickName("AllApps");await isRoute("Apps");
      const group=await until(async()=>{
        const groups=(await snapshot()).elements.filter(element=>element.hit&&element.name?.startsWith("AppGroup-"));
        return groups.sort((a,b)=>a.y-b.y)[0];
      },"first visible native alphabet header");
      await page.click(...centre(group));
      await control(element=>element.name==="AppJump-m","M jump cell");
      const alphabet=await snapshot();
      for(const cell of alphabet.elements.filter(element=>element.name?.startsWith("AppJump-"))){
        close(cell.width,cell.height,"Alphabet jump cells remain square at the phone width");
      }
      const disabled=named(alphabet,"AppJump-z");
      assert(disabled,"The inactive Z group must remain visible in the alphabet grid");
      // inspectView deliberately hit-tests disabled elements too. Verify the
      // disabled command by real input, rather than treating hit as IsEnabled.
      await page.click(...centre(disabled));
      await control(element=>element.name==="AppJump-m","Disabled group leaves the jump menu open");
      await capture("shell-alphabet-native-grid");
      await clickName("AppJump-m");
      await until(async()=>{
        const view=await snapshot(),scroll=named(view,"AppsScroll"),target=named(view,"AppGroup-m");
        return scroll&&target&&Math.abs(target.y-scroll.y)<0.6;
      },"Alphabet jump lands on the measured M group header");
      const jumped=await snapshot(),scroll=named(jumped,"AppsScroll"),next=named(jumped,"AppGroup-o");
      const sticky=named(jumped,"AppStickyGroup");
      assert.equal(sticky?.automationName,"Jump to letter m");
      assert(next,"The next native group header must be present");
      await page.wheel(...centre(scroll),0,next.y-scroll.y-sticky.height/2);
      await until(async()=>{
        const view=await snapshot(),pushed=named(view,"AppStickyGroup"),incoming=named(view,"AppGroup-o");
        if(!pushed||!incoming||incoming.y<=scroll.y||incoming.y>=scroll.y+sticky.height)return false;
        close(pushed.y,incoming.y-pushed.height,"The next actual header pushes the sticky group offscreen");
        return true;
      },"Native next-header push-off");
      await capture("shell-app-list-sticky-push");
      await edit("AppSearch","Phone");
      const result=await control(element=>element.name==="App-phone","Phone search result");
      const searched=await snapshot(),viewport=named(searched,"AppsScroll");
      close(result.y,viewport.y,"Search resets a previously scrolled app list");
      assert(!searched.elements.some(element=>element.name?.startsWith("AppGroup-")||element.name==="AppStickyGroup"),
        "Search results must not retain alphabet headers");
      await capture("shell-app-search-from-scrolled-list");
    });

    await check("native phone date and time pickers preserve popup accept and cancel behavior",async()=>{
      for(const entry of [
        {app:"Alarms",action:"add",picker:"AlarmTimePicker",selector:"PART_HourSelector",capture:"time"},
        {app:"Calendar",action:"new",picker:"CalendarDate",selector:"PART_MonthSelector",capture:"date"},
      ]){
        const focusedSelector=()=>control(element=>element.focused&&/DateTimePickerPanel$/.test(element.type)&&
          (entry.capture==="time"?element.name==="PART_HourSelector":
            ["PART_YearSelector","PART_MonthSelector","PART_DaySelector"].includes(element.name)),
          `${entry.app} popup gives native keyboard focus to its initial selector`);
        await page.evaluate(`phoneApp.openApp(${JSON.stringify(entry.app)})`);
        await isRoute({App:entry.app});await clickLabel(entry.action);
        const picker=await control(element=>element.name===entry.picker,`${entry.app} native picker`);
        const screen=named(await snapshot(),"PhoneScreen");
        assert(picker.width<=screen.width-40+0.6&&picker.height>=44,"Native phone picker fits its form and touch target");
        await clickName(entry.picker);
        await control(element=>element.name===entry.selector,"Native picker selector panel");
        await focusedSelector();
        await capture(`shell-native-${entry.capture}-picker`,{
          picker:entry.capture==="time"?"TimePickerPresenter":"DatePickerPresenter",
        });
        await clickName("PART_DismissButton");
        await until(async()=>!(await snapshot()).elements.some(element=>element.name===entry.selector),"Native picker cancel dismisses its popup");
        await clickName(entry.picker);
        await control(element=>element.name===entry.selector,"Reopened native picker selector");
        await focusedSelector();
        await clickName("PART_AcceptButton");
        await until(async()=>!(await snapshot()).elements.some(element=>element.name===entry.selector),"Native picker accept dismisses its popup");
        await clickLabel("cancel");
      }
    });
  } catch(error) {
    await capture("shell-parity-failure").catch(()=>{});
    throw error;
  } finally {
    await load(original);
  }
}
