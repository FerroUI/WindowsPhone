import assert from "node:assert/strict";

// Isolated calendar fixtures. Date/view selection and all mutations below use
// the retained native controls; no account, invitation, or outbound service runs.
export async function verifyCalendarRefinements(ctx) {
  const {page,check,state,snapshot,until,capture,button,centre,control,
    clickName,clickLabel,textPresent,reveal,edit,isRoute,reload}=ctx;
  const original=await state();
  const fixture=structuredClone(original);
  Object.assign(fixture.settings,{lock_timeout:0,reduced_motion:true,high_contrast:false});
  for(const alarm of fixture.alarms)alarm.enabled=false;
  const dateInfo=await page.evaluate(`(()=>{const date=new Date();return {
    day:[date.getFullYear(),String(date.getMonth()+1).padStart(2,'0'),String(date.getDate()).padStart(2,'0')].join('-'),
    month:date.toLocaleString('en-US',{month:'long'}),year:date.getFullYear()};})()`);
  const date=dateInfo.day;
  const series=fixture.next_id++,overlap=fixture.next_id++,allDay=fixture.next_id++;
  const base={date,time:"09:00",duration_minutes:60,location:"Local calendar",details:"Native calendar fixture.",
    all_day:false,repeat:"never",repeat_until:null,excluded_dates:[],reminder_minutes:null,status:"free"};
  fixture.events=[
    {...base,id:series,title:"Calendar recurring fixture",repeat:"daily"},
    {...base,id:overlap,title:"Calendar overlap fixture",time:"09:30"},
    {...base,id:allDay,title:"Calendar all-day fixture",time:"00:00",all_day:true,duration_minutes:1440},
  ];
  const load=async data=>{
    await page.evaluate(`phoneApp.importData(${JSON.stringify(JSON.stringify(data))})`);
    await isRoute("Start");
  };
  const open=async()=>{await page.evaluate('phoneApp.openApp("Calendar")');await isRoute({App:"Calendar"});};
  const scroller=element=>/ScrollViewer$/.test(element.type)&&element.width>220&&element.height>220;
  const visible=async(match,label)=>{
    const matches=(await snapshot()).elements.filter(element=>element.hit&&match(element));
    assert(matches.length<=1,`Ambiguous calendar control: ${label}`);
    if(matches[0])return control(match,label);
    await page.wheel(...centre(await control(scroller,"calendar scroll viewer")),0,-10000);
    return reveal(match,label,scroller,"calendar scroll viewer");
  };
  const revealName=name=>visible(element=>element.name===name,name);
  const show=async name=>page.click(...centre(await revealName(name)));
  const showLabel=async label=>page.click(...centre(await visible(
    element=>button(element)&&element.automationName===label,label)));
  const chooseView=async label=>{await clickLabel("view");await clickLabel(`Choose ${label}`);};
  let replacementId,newId;
  await load(fixture);
  try {
    await check("native calendar month, week, day and year views drill into appointments",async()=>{
      await open();
      await clickName(`CalendarDay${date}`);
      await revealName(`CalendarEvent${series}-${date}`);
      await capture("calendar-month-inline-appointments");
      await chooseView("week");await show(`CalendarWeekDay${date}`);
      await control(element=>element.name==="CalendarDayScroll","native hourly day view");
      const first=await control(element=>element.name===`CalendarAppointment${series}`,"first timed appointment");
      const second=await control(element=>element.name===`CalendarAppointment${overlap}`,"overlapping timed appointment");
      assert(first.x+first.width<=second.x+1||second.x+second.width<=first.x+1,
        "overlapping appointments must have separate native hit regions");
      await capture("calendar-day-overlapping-appointments");
      const timelineY=async()=>(await snapshot()).elements.find(element=>element.name==="CalendarTimeline")?.y;
      const beforeScroll=await timelineY();
      await page.wheel(...centre(await control(element=>element.name==="CalendarDayScroll","day scroll viewer")),0,180);
      const remembered=await until(async()=>{
        const y=await timelineY();return y<beforeScroll-1?{y}:false;
      },"native day timeline moves on wheel input");
      await chooseView("month");await chooseView("day");
      await until(async()=>Math.abs((await timelineY())-remembered.y)<0.6,
        "day view restores the user's native scroll offset after remount");
      await chooseView("year");await capture("calendar-year-navigation");
      await showLabel(`${dateInfo.month} ${dateInfo.year}`);
      await control(element=>element.name===`CalendarDay${date}`,"month restored from native year grid");
    });

    await check("native recurring appointment edit changes one occurrence atomically",async()=>{
      await show(`CalendarDay${date}`);await show(`CalendarEvent${series}-${date}`);
      await clickLabel("edit");await clickLabel("this appointment");
      await edit("CalendarSubject","Edited calendar occurrence");
      const datePicker=await revealName("CalendarDate");
      assert(/DatePicker$/.test(datePicker.type));
      await page.click(...centre(datePicker));
      await control(element=>element.name==="PART_PickerPresenter"&&/DatePickerPresenter$/.test(element.type),
        "native date picker presenter");
      await capture("calendar-native-date-picker", {picker:"DatePickerPresenter"});
      await clickName("PART_DismissButton");
      const timePicker=await revealName("CalendarTime");
      assert(/TimePicker$/.test(timePicker.type));
      await page.click(...centre(timePicker));
      await control(element=>element.name==="PART_PickerPresenter"&&/TimePickerPresenter$/.test(element.type),
        "native time picker presenter");
      await until(async()=>(await snapshot()).elements.some(element=>
        element.name==="PART_HourSelector"&&element.focused),
      "opening the time picker moves keyboard focus into its hour selector");
      // Down advances one hour and accept commits the two-way selection.
      await page.press("ArrowDown");
      await capture("calendar-native-time-picker", {picker:"TimePickerPresenter"});
      await clickName("PART_AcceptButton");
      await capture("calendar-native-occurrence-editor");
      await clickLabel("save");
      replacementId=await until(async()=>{
        const data=await state();const source=data.events.find(event=>event.id===series);
        const replacement=data.events.find(event=>event.title==="Edited calendar occurrence");
        return source?.excluded_dates.includes(date)&&replacement?.repeat==="never"&&replacement.id;
      },"one recurrence exception and standalone replacement committed");
      const persisted=await page.evaluate("JSON.parse(localStorage.getItem('ferroui.windows-phone.v1'))");
      assert(persisted.events.find(event=>event.id===series).excluded_dates.includes(date));
      const replacement=persisted.events.find(event=>event.id===replacementId);
      assert.equal(replacement.title,"Edited calendar occurrence");
      assert.equal(replacement.date,date,"canceling the date flyout preserves its previous selection");
      assert.equal(replacement.time,"10:00","native time picker commits the selected hour");
      assert.equal(persisted.events.find(event=>event.id===overlap).title,"Calendar overlap fixture");
      await show(`CalendarEvent${replacementId}-${date}`);await clickLabel("edit");
      await edit("CalendarSubject","Unsaved calendar change");await clickLabel("cancel");
      assert.equal((await state()).events.find(event=>event.id===replacementId).title,"Edited calendar occurrence");
      await clickLabel("calendar");
    });

    await check("native all-day recurring appointment preferences survive reload",async()=>{
      await clickLabel("new");await edit("CalendarSubject","Native weekly all-day appointment");
      await show("CalendarLocation");await edit("CalendarLocation","Native fixture room");
      const toggle=await visible(element=>button(element)&&element.automationName==="all day, off",
        "all-day switch");
      await page.click(...centre(toggle));
      const more=await visible(element=>button(element)&&element.automationName==="more details",
        "more appointment details");
      await page.click(...centre(more));
      await show("CalendarChoiceoccurs");await clickLabel("Choose every week");
      await show("CalendarChoicereminder");await clickLabel("Choose none");
      await show("CalendarChoicestatus");await clickLabel("Choose busy");
      await capture("calendar-recurring-appointment-details");
      await clickLabel("save");
      newId=await until(async()=>(await state()).events.find(event=>event.title==="Native weekly all-day appointment")?.id,
        "new recurring appointment saved");
      await reload();await open();
      const saved=(await state()).events.find(event=>event.id===newId);
      assert.equal(saved.all_day,true);assert.equal(saved.duration_minutes,1440);
      assert.equal(saved.repeat,"weekly");assert.equal(saved.status,"busy");assert.equal(saved.reminder_minutes,null);
      assert.equal(saved.time,"00:00");assert.equal(saved.date,date);
      await clickName(`CalendarDay${date}`);
      await revealName(`CalendarEvent${newId}-${date}`);
      await capture("calendar-reloaded-recurring-appointment");
    });
  } catch(error) {
    await capture("calendar-failure").catch(()=>{});
    throw error;
  } finally {
    await load(original);
  }
}
