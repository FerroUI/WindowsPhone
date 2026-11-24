import assert from "node:assert/strict";

// Public native controls retain all selection, scrolling and confirmation.
// The fixture API supplies isolated data; every picker action is browser input.
export async function verifyPhonePickers(ctx) {
  const {page,check,state,snapshot,until,capture,centre,control,
    clickName,clickLabel,reveal,edit,isRoute}=ctx;
  const original=await state(),viewport=await snapshot();
  const fixture=structuredClone(original);
  Object.assign(fixture.settings,{lock_timeout:0,reduced_motion:true,battery_saver:false,
    live_tiles:false,high_contrast:false,text_scale:1,dark:true,time_24h:true,
    sounds:false,ringer_volume:0,vibration:false});
  fixture.alarms=[];fixture.events=[];fixture.disabled_extensions=[];
  fixture.installed=[...new Set([...fixture.installed,"Calendar","Alarms"])];
  const named=(view,name)=>view.elements.find(element=>element.name===name);
  const frame=()=>page.evaluate("new Promise(resolve=>requestAnimationFrame(()=>requestAnimationFrame(()=>resolve(true))))");
  const near=(a,b,label)=>assert(Math.abs(a-b)<0.7,`${label}: ${a} versus ${b}`);
  const inside=(child,parent)=>child.x>=parent.x-0.7&&child.y>=parent.y-0.7&&
    child.x+child.width<=parent.x+parent.width+0.7&&child.y+child.height<=parent.y+parent.height+0.7;
  const load=async data=>{
    await page.evaluate(`phoneApp.importData(${JSON.stringify(JSON.stringify(data))})`);
    await isRoute("Start");await frame();
  };
  const app=async name=>{
    await page.evaluate(`phoneApp.openApp(${JSON.stringify(name)})`);
    await isRoute({App:name});
    await until(async()=>named(await snapshot(),"PhonePage")?.projection===null,"app entrance settles before picker input");
  };
  const scroll=element=>/ScrollViewer$/.test(element.type)&&element.width>220&&element.height>160;
  const revealName=async name=>{
    const existing=(await snapshot()).elements.find(element=>element.name===name&&element.hit);
    return existing||reveal(element=>element.name===name,name,scroll,"native editor scroll viewer");
  };
  const open=async name=>{
    await page.click(...centre(await revealName(name)));
    await until(async()=>{
      const surface=named(await snapshot(),"PhonePickerSurface");
      return surface?.hit&&surface.projection===null;
    },"native phone picker page settles");
  };
  const gone=()=>until(async()=>!named(await snapshot(),"PhonePickerSurface"),"picker page closes");
  const pageGeometry=async()=>{
    const view=await snapshot(),screen=named(view,"PhoneScreen"),host=named(view,"PhonePage");
    const picker=named(view,"PART_PickerPresenter"),surface=named(view,"PhonePickerSurface");
    assert(picker&&surface&&host&&screen,"one native phone page owns the picker surface");
    for(const property of ["x","y","width","height"])near(picker[property],host[property],`phone picker ${property}`);
    assert(inside(picker,screen),"full-page picker remains inside the phone");
    assert(picker.y>screen.y&&picker.height<screen.height,"status and hardware strips remain outside the picker");
    for(const name of ["PART_AcceptButton","PART_DismissButton","PhonePickerMore"]){
      const command=named(view,name);assert(command?.hit&&inside(command,picker),`${name} is reachable inside the picker page`);
    }
    for(const selector of view.elements.filter(element=>element.type.endsWith("DateTimePickerPanel"))){
      const selected=view.elements.filter(element=>element.type.endsWith("ListBoxItem")&&
        element.opacity===1&&element.hit&&inside(element,selector));
      assert.equal(selected.length,1,`${selector.name} renders one selected native cell inside its column`);
      near(selected[0].y+selected[0].height/2,selector.y+selector.height/2,
        `${selector.name} keeps the selected cell vertically centered after full-page layout`);
    }
    assert(named(view,"HardwareBack")?.hit,"native hardware Back remains reachable while the picker is open");
    return view;
  };
  const ownerMonth=async()=>named(await snapshot(),"PART_MonthTextBlock")?.text;
  const rawClick=async element=>{
    const point=centre(element);await page.mouseMove(...point);await page.mouseDown(...point);await page.mouseUp(...point);
  };
  const motionFrames=async action=>{
    const samples=page.evaluate(`new Promise((resolve,reject)=>{
      const frames=[],start=performance.now();let token,done=false;
      const finish=()=>{if(done)return;done=true;cancelAnimationFrame(token);clearTimeout(timeout);resolve(frames);};
      const timeout=setTimeout(finish,1500);
      const sample=()=>{try{
        const view=JSON.parse(phoneApp.inspectView());
        const surface=view.elements.find(element=>element.name==='PhonePickerSurface');
        frames.push({elapsed:performance.now()-start,surface:surface||null});
        if(performance.now()-start>=650)finish();else token=requestAnimationFrame(sample);
      }catch(error){done=true;clearTimeout(timeout);reject(error);}};sample();
    })`);
    const [frames]=await Promise.all([samples,action()]);return frames;
  };
  try {
    await page.resize(320,740);
    await until(async()=>{const view=await snapshot();return view.width===320&&view.height===740;},"320-pixel picker viewport");
    await check("phone date wheels show long months in bounded dark and light native pages",async()=>{
      const month=await page.evaluate("new Date().getMonth()+1");
      for(const dark of [true,false]){
        const compact=structuredClone(fixture);compact.settings.dark=dark;
        await load(compact);await app("Calendar");await clickLabel("new");
        await edit("CalendarSubject",`Picker date ${dark?"dark":"light"}`);
        await open("CalendarDate");await pageGeometry();
        const selector=await control(element=>element.name==="PART_MonthSelector","native month selector");
        await page.click(...centre(selector));
        for(let count=0;count<Math.abs(9-month);count++)await page.press(9>=month?"ArrowDown":"ArrowUp");
        await frame();
        const view=await snapshot();
        const september=view.elements.find(element=>element.name==="PhonePickerSecondary"&&element.text==="September"&&element.hit);
        assert(september,"September is visible in the native month wheel");
        assert(inside(september,selector),"long month text fits its own selector column");
        assert(view.elements.some(element=>element.name==="PhonePickerPrimary"&&element.text==="9"&&element.hit),
          "the source-style numeric month and localized name render separately");
        assert.equal(named(view,"PhonePickerSurface").projection,null,"reduced motion never leaves a projection attached");
        await capture(`phone-date-picker-320-${dark?"dark":"light"}`,{picker:"DatePickerPresenter"});
        await page.press("Enter");await gone();
        assert.equal(await ownerMonth(),"September","native Enter commits the staged month");
        await open("CalendarDate");
        await page.click(...centre(await control(element=>element.name==="PART_MonthSelector","month selector")));
        await page.press("ArrowDown");await page.press("Escape");await gone();
        assert.equal(await ownerMonth(),"September","Escape discards a staged month");
        await open("CalendarDate");await clickName("PhonePickerMore");
        await clickName("HardwareBack");
        assert(named(await snapshot(),"PhonePickerSurface"),"first Back collapses picker command captions");
        await clickName("HardwareBack");await gone();
        assert.equal(await ownerMonth(),"September","Back preserves the committed date");
        await control(element=>element.name==="CalendarDate","Back returns to the same editor");
        await clickLabel("save");
        const saved=await until(async()=>(await state()).events.find(event=>event.title===`Picker date ${dark?"dark":"light"}`),"native calendar form commits");
        assert.equal(saved.date.slice(5,7),"09");
      }
    });

    await check("native time page commits keyboard selection and cancels before privacy overlays",async()=>{
      await load(fixture);await app("Alarms");await clickLabel("add");await edit("AlarmName","Picker time fixture");
      await open("AlarmTimePicker");await pageGeometry();
      const opened=await snapshot();
      assert(named(opened,"PART_HourSelector")?.focused,"opening enters the hour selector after attachment");
      assert(!named(opened,"PART_PeriodHost"),"24-hour native clock omits the AM/PM column");
      await page.press("ArrowDown");await page.press("Tab");await page.press("ArrowDown");
      assert(named(await snapshot(),"PART_MinuteSelector")?.focused,"native Tab advances between wheel columns");
      await page.press("Enter");await gone();
      await clickLabel("save");
      const saved=await until(async()=>(await state()).alarms.find(alarm=>alarm.label==="Picker time fixture"),"native alarm form saves selected time");
      assert.equal(saved.hour,8);assert.equal(saved.minute,1);
      await clickLabel("Edit alarm 08:01 Picker time fixture");await open("AlarmTimePicker");await page.press("ArrowDown");
      await page.evaluate("phoneApp.lockPhone()");
      await control(element=>element.automationName==="Unlock","native lock surface");
      assert(!named(await snapshot(),"PhonePickerSurface"),"lock cancels the popup above the retained editor");
      await capture("phone-picker-canceled-before-lock");
      await clickLabel("Unlock");await control(element=>element.name==="AlarmName"&&element.text==="Picker time fixture","editor restored after unlock");
      assert.deepEqual((await state()).alarms.find(alarm=>alarm.id===saved.id),saved,"privacy teardown does not commit staged time");
      await clickLabel("cancel");
      const twelve=structuredClone(fixture);twelve.settings.time_24h=false;twelve.settings.text_scale=1.6;
      await load(twelve);await app("Alarms");await clickLabel("add");await open("AlarmTimePicker");
      await pageGeometry();
      assert(named(await snapshot(),"PART_PeriodHost"),"12-hour clock keeps its native period selector");
      await capture("phone-time-picker-320-accessibility",{picker:"TimePickerPresenter"});
      await clickName("PART_DismissButton");await gone();
    });

    await check("phone picker renders original signed swivel entrance and closing poses",async()=>{
      const animated=structuredClone(fixture);animated.settings.reduced_motion=false;
      await load(animated);await app("Alarms");await clickLabel("add");
      const input=await revealName("AlarmTimePicker");
      const entering=await motionFrames(()=>rawClick(input));
      const entrance=entering.map(frame=>frame.surface).filter(surface=>surface?.projection?.kind==="swivel");
      assert(entrance.some(surface=>surface.projection.angleX<-1),"opening renders native negative-X swivel poses");
      assert(entrance.every(surface=>surface.projection.angleX>=-50.1&&surface.projection.angleX<=0.1),"entrance follows -50 to zero degrees");
      assert(entrance.every(surface=>surface.hitTestVisible===false),"moving picker content does not accept a second pointer action");
      const resting=entering.at(-1).surface;assert(resting&&resting.projection===null&&resting.hitTestVisible,
        "entrance releases its transform and restores native input");
      const leaving=await motionFrames(async()=>rawClick(named(await snapshot(),"HardwareBack")));
      const exit=leaving.map(frame=>frame.surface).filter(surface=>surface?.projection?.kind==="swivel");
      assert(exit.some(surface=>surface.projection.angleX>1),"Back renders the positive-X closing swivel");
      assert(exit.every(surface=>surface.projection.angleX>=-0.1&&surface.projection.angleX<=90.1),"closing follows zero to +90 degrees");
      assert.equal(leaving.at(-1).surface,null,"closing removes the native popup and transform");
      await capture("phone-picker-swivel-restored-editor");
    });
  } catch(error) {
    const presenter=named(await snapshot(),"PART_PickerPresenter");
    const picker=presenter?.type.endsWith("DatePickerPresenter")?"DatePickerPresenter":
      presenter?.type.endsWith("TimePickerPresenter")?"TimePickerPresenter":undefined;
    await capture("phone-picker-failure",picker?{picker}:{}).catch(()=>{});throw error;
  } finally {
    await page.resize(viewport.width,viewport.height);
    await until(async()=>{const view=await snapshot();return view.width===viewport.width&&view.height===viewport.height;},"picker fixture restores viewport");
    await load(original);
  }
}
