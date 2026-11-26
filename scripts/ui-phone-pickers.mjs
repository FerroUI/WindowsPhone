import assert from "node:assert/strict";

export function assertPickerColumns(view, expectedNames) {
  const surface=view.elements.find(element=>element.name==="PhonePickerSurface");
  assert(surface&&surface.projection===null&&surface.hitTestVisible,
    "picker input geometry is inspected only on the settled native page");
  const selectors=view.elements.filter(element=>element.type.endsWith("DateTimePickerPanel"));
  assert.equal(new Set(selectors.map(selector=>selector.name)).size,selectors.length,
    "each native picker column is mounted exactly once");
  if(expectedNames)assert.deepEqual(selectors.map(selector=>selector.name).sort(),[...expectedNames].sort(),
    "the picker mounts every visible date/time field");
  assert(selectors.length>=2,"a date/time page mounts separate native selector columns");
  const ordered=[...selectors].sort((a,b)=>a.x-b.x);
  for(let index=0;index<ordered.length;index++){
    const selector=ordered[index];
    assert(selector.width>0&&selector.height>0,`${selector.name} has positive native bounds`);
    assert(selector.x>=surface.x-0.7&&selector.x+selector.width<=surface.x+surface.width+0.7,
      `${selector.name} stays inside the picker page`);
    if(index){
      const previous=ordered[index-1];
      assert(selector.x>=previous.x+previous.width-0.7,
        `${previous.name} and ${selector.name} have separate, nonoverlapping input columns (${previous.x},${previous.width}; ${selector.x},${selector.width})`);
    }
    assert(selector.hit&&selector.enabled,`${selector.name} accepts native input at its selected cell`);
  }
  return selectors;
}

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
    const expected=picker.type.endsWith("DatePickerPresenter")?
      ["PART_YearSelector","PART_MonthSelector","PART_DaySelector"]:
      ["PART_HourSelector","PART_MinuteSelector",...((await state()).settings.time_24h?[]:["PART_PeriodSelector"])];
    for(const selector of assertPickerColumns(view,expected)){
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
  const wheel=async name=>{
    const view=await snapshot(),selector=named(view,name);
    assert(selector?.hit,`${name} has a reachable native selection`);
    const contained=element=>{
      const [x,y]=centre(element);
      return x>=selector.x&&x<selector.x+selector.width&&y>=selector.y&&y<selector.y+selector.height;
    };
    const cells=view.elements.filter(element=>element.type.endsWith("ListBoxItem")&&element.hit&&element.opacity>0&&contained(element));
    const selected=cells.filter(cell=>cell.opacity===1);
    assert.equal(selected.length,1,`${name} has one fully opaque selected cell`);
    const text=view.elements.filter(element=>element.name==="PhonePickerPrimary"&&inside(element,selected[0]));
    assert.equal(text.length,1,`${name} exposes its selected native value`);
    return {selector,cells,selected:selected[0],value:text[0].text};
  };
  const selectedValue=(name,value)=>until(async()=>{
    const current=await wheel(name);return current.value===value?current:undefined;
  },`${name} stages ${value} through native input`);
  const nextCell=current=>{
    const below=current.cells.filter(cell=>cell.y>current.selected.y+1).sort((a,b)=>a.y-b.y)[0];
    assert(below,"the expanded wheel exposes a next-value tap target");return below;
  };
  const countedClick=async(point,count)=>{
    await page.mouseMove(...point);
    for(const [type,buttons] of [["mousePressed",1],["mouseReleased",0]])
      await page.send("Input.dispatchMouseEvent",{type,x:point[0],y:point[1],button:"left",buttons,clickCount:count,pointerType:"mouse"});
  };
  const touch=async(type,point)=>page.send("Input.dispatchTouchEvent",{
    type,touchPoints:type==="touchEnd"||type==="touchCancel"?[]:[{x:point[0],y:point[1],id:0,radiusX:2,radiusY:2,force:1}],
  });
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

    await check("native picker taps, double taps and touch gestures stage values until explicit confirmation",async()=>{
      await load(fixture);await app("Alarms");await clickLabel("add");await edit("AlarmName","Pointer time fixture");
      const document=await state();
      await open("AlarmTimePicker");await pageGeometry();
      let hour=await selectedValue("PART_HourSelector","7");
      await rawClick(hour.selected);await frame();
      hour=await wheel("PART_HourSelector");
      assert.equal(hour.value,"7","expanding the selected wheel does not change its value");
      const next=centre(nextCell(hour));
      await countedClick(next,1);await selectedValue("PART_HourSelector","8");
      await countedClick(next,2);await selectedValue("PART_HourSelector","9");
      assert(named(await snapshot(),"PhonePickerSurface"),"a rapid second tap does not confirm or close the picker");
      assert.deepEqual(await state(),document,"tap selection remains staged, without persisting the alarm");

      let minute=await selectedValue("PART_MinuteSelector","00");
      await rawClick(minute.selected);await frame();minute=await wheel("PART_MinuteSelector");
      const touchEvidence={events:[],frames:[]};
      let touchActive=false;
      const observedTouch=async()=>{
        touchEvidence.events=await page.evaluate("globalThis.__phonePickerTouchProbe.events");
        return touchEvidence.events;
      };
      await page.send("Emulation.setTouchEmulationEnabled",{enabled:true,maxTouchPoints:5});
      try{
        // Observe trusted browser delivery separately from native selection so
        // a missing touch stream cannot make the cancellation assertion pass.
        await page.evaluate(`(()=>{
          const events=[],types=['pointerdown','pointermove','pointerup','pointercancel'];
          const observe=event=>{
            if(event.pointerType==='touch')events.push({type:event.type,id:event.pointerId,
              x:event.clientX,y:event.clientY,trusted:event.isTrusted,target:event.target?.tagName});
          };
          for(const type of types)document.addEventListener(type,observe,true);
          globalThis.__phonePickerTouchProbe={events,dispose:()=>{
            for(const type of types)document.removeEventListener(type,observe,true);
          }};
        })()`);
        await touch("touchStart",centre(nextCell(minute)));touchActive=true;
        const cancelDown=(await observedTouch()).find(event=>event.type==="pointerdown");
        assert(cancelDown?.trusted,"cancel fixture delivers a trusted DOM touch press");
        await touch("touchCancel");touchActive=false;await frame();
        assert((await observedTouch()).some(event=>event.type==="pointercancel"&&event.id===cancelDown.id&&event.trusted),
          "cancel fixture terminates that same native touch contact");
        await selectedValue("PART_MinuteSelector","00");
        assert.deepEqual(await state(),document,"canceling a touch does not synthesize selection or a value commit");

        const from=centre(minute.selected),distance=minute.selected.height*1.25;
        const firstDragEvent=touchEvidence.events.length;
        await touch("touchStart",from);touchActive=true;
        const dragDown=(await observedTouch()).slice(firstDragEvent).find(event=>event.type==="pointerdown");
        assert(dragDown?.trusted,"drag starts a fresh trusted DOM touch contact after cancellation");
        for(let step=1;step<=8;step++){
          await touch("touchMove",[from[0],from[1]-distance*step/8]);await frame();
          const current=await wheel("PART_MinuteSelector");
          touchEvidence.frames.push({step,value:current.value,selectedY:current.selected.y,
            selectedHeight:current.selected.height,selectorY:current.selector.y,selectorHeight:current.selector.height});
        }
        assert((await observedTouch()).some(event=>event.type==="pointermove"&&event.id===dragDown.id&&event.trusted),
          "the browser forwards touch movement to the native canvas");
        assert(touchEvidence.frames.some(current=>current.value!==minute.value||Math.abs(current.selectedY-minute.selected.y)>8),
          "the native minute wheel moves while the touch contact remains down");
        await touch("touchEnd");touchActive=false;
        assert((await observedTouch()).some(event=>event.type==="pointerup"&&event.id===dragDown.id&&event.trusted),
          "the native drag receives a matching touch release");
        await until(async()=>{
          const current=await wheel("PART_MinuteSelector");
          return current.value!=="00"&&Math.abs(current.selected.y+current.selected.height/2-current.selector.y-current.selector.height/2)<0.7;
        },"native touch scrolling advances and snaps the minute wheel");
      }finally{
        if(touchActive)await touch("touchCancel").catch(()=>{});
        await observedTouch().catch(()=>{});
        console.log("Phone picker touch evidence",JSON.stringify(touchEvidence));
        await page.evaluate("globalThis.__phonePickerTouchProbe?.dispose();delete globalThis.__phonePickerTouchProbe").catch(()=>{});
        await page.send("Emulation.setTouchEmulationEnabled",{enabled:false});
      }
      assert.deepEqual(await state(),document,"dragging and snapping do not persist staged picker values");
      await capture("phone-picker-native-pointer-staging",{picker:"TimePickerPresenter"});
      await clickName("HardwareBack");await gone();
      await open("AlarmTimePicker");
      await selectedValue("PART_HourSelector","7");await selectedValue("PART_MinuteSelector","00");
      hour=await wheel("PART_HourSelector");await rawClick(hour.selected);await frame();
      await rawClick(nextCell(await wheel("PART_HourSelector")));await selectedValue("PART_HourSelector","8");
      await clickName("PART_AcceptButton");await gone();await clickLabel("save");
      const saved=await until(async()=>(await state()).alarms.find(alarm=>alarm.label==="Pointer time fixture"),"explicitly confirmed pointer time saves");
      assert.equal(saved.hour,8);assert.equal(saved.minute,0);
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
