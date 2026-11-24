import assert from "node:assert/strict";

// Native alarm editing/presentation with an isolated clock and muted fixtures.
// The production Rust scheduler receives real host ticks; no OS notification,
// sharing, sound permission, or background wake-up is requested.
export async function verifyAlarmRefinements(ctx) {
  const {page,check,state,snapshot,until,capture,button,centre,control,
    clickName,clickLabel,edit,isRoute,reveal}=ctx;
  const original=await state();
  const fixture=structuredClone(original);
  Object.assign(fixture.settings,{lock_timeout:0,time_24h:true,sounds:false,
    ringer_volume:0,vibration:false,reduced_motion:true,high_contrast:false});
  fixture.alarms=[];fixture.events=[];
  const load=async data=>{
    await page.evaluate(`phoneApp.importData(${JSON.stringify(JSON.stringify(data))})`);
    await isRoute("Start");
  };
  const app=async name=>{
    await page.evaluate(`phoneApp.openApp(${JSON.stringify(name)})`);
    await isRoute({App:name});
  };
  const settled=()=>page.evaluate("new Promise(resolve=>requestAnimationFrame(()=>requestAnimationFrame(()=>resolve(true))))");
  const scroller=element=>/ScrollViewer$/.test(element.type)&&element.width>220&&element.height>160;
  const choose=async label=>{
    let target=(await snapshot()).elements.find(element=>element.hit&&button(element)&&element.automationName===label);
    if(!target){
      await page.wheel(...centre(await control(scroller,"alarm editor scroll viewer")),0,-10000);
      target=await reveal(element=>button(element)&&element.automationName===label,label,scroller,"alarm editor scroll viewer");
    }
    await page.click(...centre(target));
  };
  const tick=async seconds=>{
    await page.evaluate(`globalThis.__phoneAlarmClock.seconds=${seconds};phoneApp.tick(${seconds})`);
    await settled();
  };
  const noAlert=async()=>!(await snapshot()).elements.some(element=>element.name==="PhoneAlarm");
  const snoozeChoices=async selected=>{
    const minutes=[5,10,20,30,60];
    const rendered=await until(async()=>{
      const view=await snapshot();
      const rows=minutes.map(value=>view.elements.find(element=>button(element)&&
        element.automationName===`Snooze ${value} minutes`&&element.hit&&element.enabled));
      return rows.every(row=>row&&row.width>0&&row.height>=56)&&{view,rows};
    },"all five native snooze choices are rendered and interactive");
    const screen=rendered.view.elements.find(element=>element.name==="PhoneScreen");
    const color=value=>value?.toLowerCase().replace(/^#ff(?=[0-9a-f]{6}$)/,"#");
    for(const [index,row] of rendered.rows.entries()){
      assert(Math.abs(row.width-(screen.width-40))<0.01,
        `Snooze ${minutes[index]} must retain the full editor row target`);
      const caption=rendered.view.elements.find(element=>element.type==="TextBlock"&&
        element.text===`${minutes[index]} minutes`&&element.width>0&&element.height>0&&
        element.x>=row.x&&element.x<row.x+row.width&&element.y>=row.y&&element.y<row.y+row.height);
      assert(caption,`Snooze ${minutes[index]} must retain its visible native caption`);
      assert.equal(color(caption.foreground),color(minutes[index]===selected?fixture.settings.accent:
        fixture.settings.dark?"#FFFFFF":"#000000"),
      `Snooze ${minutes[index]} caption must expose the actual selection color`);
    }
  };
  let frozen=false;
  try {
    await load(fixture);
    await check("native alarm editor saves repeat days and configurable snooze",async()=>{
      await app("Alarms");await clickLabel("add");
      await control(element=>element.name==="AlarmTimePicker"&&/TimePicker$/.test(element.type),"native alarm time picker");
      await edit("AlarmName","Native repeating alarm");
      await choose("Choose alarm repeat days");
      await clickLabel("Monday, not selected");await clickLabel("Wednesday, not selected");
      await clickLabel("done");
      await choose("Choose alarm snooze time");await snoozeChoices(5);
      await clickLabel("Snooze 20 minutes");
      await choose("Choose alarm snooze time");await snoozeChoices(20);
      await capture("alarm-native-snooze-choices");await clickLabel("done");
      await capture("alarm-native-editor");
      await clickLabel("save");
      const saved=await until(async()=>(await state()).alarms.find(alarm=>alarm.label==="Native repeating alarm"),
        "native alarm saved");
      assert.equal(saved.hour,7);assert.equal(saved.minute,0);
      assert.deepEqual(saved.weekdays,[0,2]);assert.equal(saved.snooze_minutes,20);
      const persisted=await page.evaluate("JSON.parse(localStorage.getItem('ferroui.windows-phone.v1'))");
      assert.deepEqual(persisted.alarms.find(alarm=>alarm.id===saved.id),saved);
      await clickLabel("Edit alarm 07:00 Native repeating alarm");
      await edit("AlarmName","Unsaved alarm change");await clickName("HardwareBack");
      assert.equal((await state()).alarms.find(alarm=>alarm.id===saved.id).label,"Native repeating alarm");
    });

    const clock=await page.evaluate(`(()=>{
      const seconds=Math.floor(Date.now()/60000)*60+20;
      globalThis.__phoneAlarmClock={original:Date.now,seconds};
      Date.now=()=>globalThis.__phoneAlarmClock.seconds*1000;
      phoneApp.setUtcOffset(-new Date(seconds*1000).getTimezoneOffset()*60);
      phoneApp.tick(seconds);
      const due=new Date((seconds+60)*1000);
      return {base:seconds,hour:due.getHours(),minute:due.getMinutes()};
    })()`);
    frozen=true;
    const alarmFixture=structuredClone(fixture),alarmId=alarmFixture.next_id++;
    alarmFixture.alarms=[{id:alarmId,hour:clock.hour,minute:clock.minute,label:"Foreground alarm fixture",
      enabled:true,weekdays:[],snooze_minutes:10}];
    await load(alarmFixture);
    await check("foreground alarm preserves an unsaved form and Back snoozes two minutes",async()=>{
      await app("Office");await clickLabel("recent");await clickLabel("new");
      await clickLabel("Word document.docx");await edit("FormField0","Unsaved alarm-interrupted draft");
      const before=await state();
      await tick(clock.base+60);
      await control(element=>element.name==="PhoneAlarm","native ringing alarm");
      assert.equal((await state()).alarms.find(alarm=>alarm.id===alarmId).enabled,false,
        "a one-time alarm is consumed on first delivery");
      assert(!(await snapshot()).elements.some(element=>element.hit&&element.name==="FormField0"),
        "the interrupted form must not remain interactive above the alert");
      await capture("alarm-native-ringing");
      await clickName("HardwareBack");await until(noAlert,"Back snoozed the alarm");
      await control(element=>element.name==="FormField0"&&element.text==="Unsaved alarm-interrupted draft",
        "unsaved draft restored after snooze");
      await tick(clock.base+179);assert(await noAlert(),"Back snooze must not fire early");
      await tick(clock.base+180);
      await control(element=>element.name==="PhoneAlarm","two-minute snoozed alarm");
      await clickLabel("Change reminder snooze time");await clickLabel("Remind in 20 minutes");
      await clickName("AlarmSnooze");await until(noAlert,"selected snooze applied");
      await tick(clock.base+1379);assert(await noAlert(),"selected snooze must not fire early");
      await tick(clock.base+1380);
      await control(element=>element.name==="PhoneAlarm","twenty-minute snoozed alarm");
      await clickName("AlarmDismiss");await until(noAlert,"alarm dismissed");
      await tick(clock.base+1381);assert(await noAlert(),"dismissed alarm must not repeat within the minute");
      await control(element=>element.name==="FormField0"&&element.text==="Unsaved alarm-interrupted draft",
        "exact draft survives both alarm deliveries");
      await capture("alarm-restored-office-draft");
      await clickLabel("cancel");
      const after=await state();
      assert.deepEqual(after.documents,before.documents);
      assert.equal(after.next_id,before.next_id,"alarm delivery must not commit an interrupted draft");
    });

    const reminderBase=clock.base+1500;
    const occurrence=await page.evaluate(`(()=>{
      const date=new Date((${reminderBase}+900)*1000);
      const storageDate=date=>[date.getFullYear(),String(date.getMonth()+1).padStart(2,'0'),String(date.getDate()).padStart(2,'0')].join('-');
      const first=new Date(date);first.setDate(first.getDate()-1);
      return {date:storageDate(date),firstDate:storageDate(first),
        longDate:date.toLocaleString('en-US',{weekday:'long'})+', '+date.getDate()+' '+date.toLocaleString('en-US',{month:'long'})+' '+date.getFullYear(),
        time:[String(date.getHours()).padStart(2,'0'),String(date.getMinutes()).padStart(2,'0')].join(':')};
    })()`);
    const reminderFixture=structuredClone(fixture),eventId=reminderFixture.next_id++;
    reminderFixture.events=[{id:eventId,date:occurrence.firstDate,time:occurrence.time,title:"Native recurring reminder",location:"Local fixture room",
      details:"Foreground reminder fixture",duration_minutes:30,all_day:false,repeat:"daily",repeat_until:null,
      excluded_dates:[],reminder_minutes:15,status:"busy"}];
    await tick(reminderBase-60);await load(reminderFixture);
    await check("foreground calendar reminder snoozes and opens its exact occurrence",async()=>{
      await app("Alarms");await clickLabel("add");await edit("AlarmName","Interrupted picker draft");
      await clickName("AlarmTimePicker");
      await control(element=>element.name==="PART_PickerPresenter"&&/TimePickerPresenter$/.test(element.type),
        "time picker before Calendar interruption");
      await page.press("ArrowDown");
      await tick(reminderBase);
      await control(element=>element.name==="PhoneAlarm"&&element.automationName==="Calendar reminder",
        "native Calendar reminder");
      assert(!(await snapshot()).elements.some(element=>element.name==="PART_PickerPresenter"),
        "an incoming alert must cancel the native wheel popup above the phone page");
      await capture("calendar-native-reminder");
      await clickName("AlarmSnooze");await until(noAlert,"Calendar reminder snoozed");
      await control(element=>element.name==="AlarmName"&&element.text==="Interrupted picker draft",
        "canceling the interrupted popup preserves the unsaved alarm editor");
      await tick(reminderBase+299);assert(await noAlert());
      await tick(reminderBase+300);
      await control(element=>element.name==="PhoneAlarm","snoozed Calendar reminder");
      await clickLabel("Open reminder appointment");await isRoute({App:"Calendar"});
      await until(noAlert,"opening appointment dismisses its reminder");
      await control(element=>button(element)&&element.automationName==="edit","opened Calendar appointment");
      assert((await snapshot()).elements.some(element=>element.text==="Native recurring reminder"));
      assert((await snapshot()).elements.some(element=>element.text===occurrence.longDate),
        "opening a recurring reminder must select its due occurrence, not the series start date");
      assert.deepEqual((await state()).events,reminderFixture.events,"reminder delivery must preserve the recurring series");
      await capture("calendar-reminder-opened-occurrence");
    });
  } catch(error) {
    await capture("alarm-failure").catch(()=>{});
    throw error;
  } finally {
    if(frozen)await page.evaluate(`(()=>{
      Date.now=globalThis.__phoneAlarmClock.original;
      delete globalThis.__phoneAlarmClock;
      phoneApp.setUtcOffset(-new Date().getTimezoneOffset()*60);
      phoneApp.tick(Date.now()/1000);
    })()`);
    await load(original);
  }
}
