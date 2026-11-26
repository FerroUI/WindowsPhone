import assert from "node:assert/strict";

// Real native controls own every task mutation. The host bridge only installs
// isolated documents/clock ticks and observes persisted state; it never writes
// editor properties or invokes task commands. No account or OS wake-up is used.
export async function verifyCalendarTasks(ctx) {
  const {page, check, state, snapshot, until, capture, button, centre, control,
    clickName, clickLabel, edit, isRoute, reveal, reload, home} = ctx;
  const original = await state();
  const fixture = structuredClone(original);
  Object.assign(fixture.settings, {lock_timeout:0, reduced_motion:true, time_24h:true,
    sounds:false, ringer_volume:0, vibration:false, high_contrast:false});
  fixture.alarms = []; fixture.events = []; fixture.app_data ||= {};
  delete fixture.app_data["phone.calendar"];
  const library = async () => (await state()).app_data?.["phone.calendar"];
  const load = async data => {
    await page.evaluate(`phoneApp.importData(${JSON.stringify(JSON.stringify(data))})`);
    await isRoute("Start");
  };
  const calendar = async () => {
    await page.evaluate('phoneApp.openApp("Calendar")');
    await isRoute({App:"Calendar"});
  };
  const tasks = async () => {
    await calendar(); await clickLabel("more"); await clickLabel("tasks");
    await control(element => element.name === "CalendarTaskFilter", "Calendar tasks list");
  };
  const scroller = element => /ScrollViewer$/.test(element.type) && element.width > 220 && element.height > 170;
  const visible = async (match, label) => {
    if ((await snapshot()).elements.some(element => element.hit && match(element))) return control(match, label);
    await page.wheel(...centre(await control(scroller, "task page scroll viewer")), 0, -10000);
    return reveal(match, label, scroller, "task page scroll viewer");
  };
  const showName = async name => page.click(...centre(await visible(element => element.name === name, name)));
  const showLabel = async label => page.click(...centre(await visible(element => button(element) && element.automationName === label, label)));
  const field = async (name, value) => { await visible(element => element.name === name, name); await edit(name, value); };
  const noAlert = async () => !(await snapshot()).elements.some(element => element.name === "PhoneAlarm");
  const nativeTaskTitle = async title => until(async () => (await snapshot()).elements.some(element =>
    element.name === "CalendarTaskTitle" && element.text === title), `task details: ${title}`);
  const selectFilter = async label => { await clickName("CalendarTaskFilter"); await clickLabel(label); };
  let taskId, savedFields, frozen = false;
  try {
    await load(fixture);
    await check("Calendar overflow creates a persistent task with native due date, priority, reminder and notes", async () => {
      await tasks(); await clickLabel("new");
      await field("CalendarTaskSubject", "Native Calendar task");
      await showLabel("due date, off");
      const date = await visible(element => element.name === "CalendarTaskDue", "native task due date");
      assert(/DatePicker$/.test(date.type));
      await page.click(...centre(date));
      await control(element => element.name === "PART_PickerPresenter" && /DatePickerPresenter$/.test(element.type), "task date picker");
      await clickName("PART_DismissButton");
      await showName("CalendarTaskPriority"); await clickLabel("high");
      await showLabel("reminder, off");
      await showName("CalendarTaskReminderTime");
      await control(element => element.name === "PART_PickerPresenter" && /TimePickerPresenter$/.test(element.type), "task reminder time picker");
      await until(async () => (await snapshot()).elements.some(element => element.name === "PART_HourSelector" && element.focused), "task reminder hour focused");
      await page.press("ArrowDown"); await clickName("PART_AcceptButton");
      await field("CalendarTaskNotes", "First paragraph.\n\nSecond paragraph — local task notes.");
      assert.equal(await library(), undefined, "typing and native picker confirmation do not save the task");
      await capture("calendar-task-native-editor");
      await clickLabel("save");
      const saved = await until(async () => { const value = await library(); return value?.tasks.length === 1 && value.tasks[0]; }, "task saved through AppStorage");
      taskId = saved.id; savedFields = saved.fields;
      assert.equal(savedFields.title, "Native Calendar task");
      assert.equal(savedFields.notes, "First paragraph.\n\nSecond paragraph — local task notes.");
      assert.equal(savedFields.priority, "high"); assert(savedFields.due); assert(savedFields.reminder);
      assert.equal(saved.revision, 1); assert.equal(saved.completed_at, null);
      const persisted = await page.evaluate("JSON.parse(localStorage.getItem('ferroui.windows-phone.v1'))");
      assert.deepEqual(persisted.app_data["phone.calendar"], await library());
      assert.deepEqual(persisted.events, fixture.events, "tasks do not fabricate Calendar appointments");
      await nativeTaskTitle(savedFields.title); await capture("calendar-task-saved-details");
    });

    await check("Calendar task drafts survive app switching and require explicit discard; saved tasks survive reload", async () => {
      await clickLabel("edit"); await field("CalendarTaskNotes", "Unsaved task text retained across apps.");
      await home(); await calendar();
      await visible(element => element.name === "CalendarTaskNotes" && element.text === "Unsaved task text retained across apps.", "retained task notes");
      await clickName("HardwareBack");
      await control(element => element.name === "DialogAction" && element.automationName === "discard", "task discard confirmation");
      await capture("calendar-task-discard-confirmation");
      await clickName("DialogCancel");
      assert.deepEqual((await library()).tasks[0].fields, savedFields);
      await clickName("HardwareBack"); await clickName("DialogAction");
      await nativeTaskTitle(savedFields.title);
      await reload(); await tasks(); await clickName(`CalendarTask${taskId}`);
      await nativeTaskTitle(savedFields.title);
      assert.deepEqual((await library()).tasks[0].fields, savedFields);
      await capture("calendar-task-restored-details");
    });

    await check("Calendar task hold actions, completion, reopening, search and due filtering update one saved item", async () => {
      await clickName("HardwareBack");
      const [x, y] = centre(await control(element => element.name === `CalendarTask${taskId}`, "task context target"));
      await page.send("Input.dispatchMouseEvent", {type:"mousePressed", x, y, button:"right", buttons:2, clickCount:1, pointerType:"mouse"});
      await page.send("Input.dispatchMouseEvent", {type:"mouseReleased", x, y, button:"right", buttons:0, clickCount:1, pointerType:"mouse"});
      await control(element => element.name === "CalendarTaskContext", "in-surface task context menu");
      await capture("calendar-task-context-menu"); await clickLabel("complete");
      await until(async () => (await library()).tasks[0].completed_at !== null, "context completion persisted");
      await selectFilter("completed"); await clickName(`CalendarTask${taskId}`); await clickLabel("reopen");
      await until(async () => (await library()).tasks[0].completed_at === null, "task reopened");
      await clickName("HardwareBack"); await selectFilter("active");
      await clickLabel("search"); await edit("CalendarTaskSearch", "SECOND PARAGRAPH");
      await control(element => element.name === `CalendarTask${taskId}`, "case-insensitive notes search result");
      await edit("CalendarTaskSearch", "no task should match this");
      await until(async () => (await snapshot()).elements.some(element => element.text === "no matching tasks"), "empty task search result");
      await edit("CalendarTaskSearch", ""); await selectFilter("due today");
      await control(element => element.name === `CalendarTask${taskId}`, "due today includes the saved native due date");
      assert.equal((await library()).tasks[0].revision, 3);
      await capture("calendar-task-search-and-filter");
      await clickName(`CalendarTask${taskId}`);
    });

    await check("a pinned task activates its typed identity and confirmed deletion retires that identity", async () => {
      await clickLabel("pin"); await isRoute("Start");
      const data = await state();
      const link = data.app_links.find(item => item.app === "phone.calendar" && item.path === `task/${taskId}`);
      assert(link, "task pin uses the SDK app-link contract");
      const tile = data.tiles.find(item => item.target?.AppLink === link.id); assert(tile);
      await page.wheel(...centre(await control(element => element.name === "StartScroll", "Start scroll viewer")), 0, -10000);
      const target = await reveal(element => element.name === `Tile-${tile.id}`, "pinned task tile", element => element.name === "StartScroll", "Start scroll viewer");
      await page.click(...centre(target)); await isRoute({App:"Calendar"}); await nativeTaskTitle(savedFields.title);
      await capture("calendar-task-pinned-activation");
      await clickLabel("more"); await clickLabel("delete");
      await control(element => element.name === "DialogAction" && element.automationName === "delete", "task delete confirmation");
      await clickName("DialogCancel"); assert.equal((await library()).tasks.length, 1);
      await clickLabel("more"); await clickLabel("delete"); await clickName("DialogAction");
      await until(async () => (await library()).tasks.length === 0, "confirmed task deletion");
      assert.equal((await library()).next_id, taskId + 1, "task identity is not reused after deletion");
      await capture("calendar-task-deleted");
    });

    const clock = await page.evaluate(`(() => {
      const seconds=Math.floor(Date.now()/60000)*60+10;
      globalThis.__phoneTaskClock={original:Date.now,seconds};
      Date.now=()=>globalThis.__phoneTaskClock.seconds*1000;
      phoneApp.setUtcOffset(-new Date(seconds*1000).getTimezoneOffset()*60);phoneApp.tick(seconds);
      const date=new Date((seconds+60)*1000);
      return {base:seconds,date:[date.getFullYear(),String(date.getMonth()+1).padStart(2,'0'),String(date.getDate()).padStart(2,'0')].join('-'),
        time:[String(date.getHours()).padStart(2,'0'),String(date.getMinutes()).padStart(2,'0')].join(':')};
    })()`);
    frozen = true;
    const tick = async seconds => {
      await page.evaluate(`globalThis.__phoneTaskClock.seconds=${seconds};phoneApp.tick(${seconds})`);
      await page.evaluate("new Promise(resolve=>requestAnimationFrame(()=>requestAnimationFrame(()=>resolve(true))))");
    };
    const reminderFixture = structuredClone(fixture);
    reminderFixture.app_data["phone.calendar"] = {version:1, next_id:3, tasks:[1,2].map(id => ({
      id, revision:1, created:clock.base, updated:clock.base, completed_at:null,
      fields:{title:`Foreground task ${id}`, notes:"Persisted independent of Calendar's active view.",
        due:clock.date, priority:"normal", reminder:{date:clock.date,time:clock.time}},
    }))};
    await load(reminderFixture);
    await check("foreground task reminders queue outside Calendar, snooze, activate the exact task and cancel on completion", async () => {
      await page.evaluate('phoneApp.openApp("Calculator")'); await isRoute({App:"Calculator"});
      await tick(clock.base + 60);
      await control(element => element.name === "PhoneAlarm" && element.automationName === "App reminder", "first task reminder");
      assert((await snapshot()).elements.some(element => element.text === "Foreground task 1"));
      await capture("calendar-task-foreground-reminder");
      await clickName("AlarmSnooze");
      await until(async () => (await snapshot()).elements.some(element => element.text === "Foreground task 2"), "queued second task reminder");
      await clickName("AlarmDismiss"); await until(noAlert, "second task dismissed");
      await tick(clock.base + 359); assert(await noAlert(), "task snooze never fires early");
      await tick(clock.base + 360);
      await control(element => element.name === "PhoneAlarm", "snoozed task reminder");
      await clickLabel("Open reminder item"); await isRoute({App:"Calendar"});
      await nativeTaskTitle("Foreground task 1"); await until(noAlert, "opening task dismisses its reminder");
      await clickLabel("complete");
      await tick(clock.base + 361); assert(await noAlert());
      assert((await library()).tasks.find(item => item.id === 1).completed_at !== null);
      assert.equal((await library()).tasks.find(item => item.id === 2).completed_at, null,
        "dismissing another reminder never completes its task");
      await capture("calendar-task-reminder-opened-and-completed");
    });

    await check("locked task and appointment reminders hide private content until explicit opening unlocks them", async () => {
      const privateTasks = structuredClone(reminderFixture);
      privateTasks.settings.notifications_on_lock = false;
      privateTasks.app_data["phone.calendar"].tasks = [privateTasks.app_data["phone.calendar"].tasks[0]];
      const privateTitle = "Private locked task subject";
      privateTasks.app_data["phone.calendar"].tasks[0].fields.title = privateTitle;
      const locked = async (fixture, label, privateText) => {
        await tick(clock.base); await load(fixture);
        await page.evaluate("phoneApp.lockPhone()");
        await control(element => element.automationName === "Unlock", "locked phone before reminder");
        await tick(clock.base + 60);
        await control(element => element.name === "PhoneAlarm" && element.automationName === label, "private locked reminder");
        const view = await snapshot();
        for (const text of privateText) assert(!view.elements.some(element => element.text?.includes(text)),
          `locked reminder does not mount private text: ${text}`);
        assert(!view.elements.some(element => element.name === "ReminderTime"), "private occurrence/reminder time is not mounted");
        assert(view.elements.some(element => element.text === "New reminder"));
        await control(element => element.name === "AlarmSnooze", "private reminder snooze");
        await control(element => element.name === "AlarmDismiss", "private reminder dismiss");
      };
      await locked(privateTasks, "App reminder", [privateTitle, "normal priority"]);
      await capture("calendar-task-locked-reminder-private");
      await clickLabel("Open reminder item"); await isRoute({App:"Calendar"});
      await nativeTaskTitle(privateTitle); await until(noAlert, "private task explicitly opened");
      assert(!(await snapshot()).elements.some(element => element.name === "LockScreen"));
      await capture("calendar-task-unlocked-reminder-details");

      const privateCalendar = structuredClone(fixture);
      privateCalendar.settings.notifications_on_lock = false;
      const subject = "Private locked appointment", location = "Private appointment location";
      privateCalendar.events = [{id:privateCalendar.next_id++, date:clock.date, time:clock.time,
        title:subject, location, details:"Private appointment notes", duration_minutes:30, all_day:false,
        repeat:"never", repeat_until:null, excluded_dates:[], reminder_minutes:0, status:"busy"}];
      await locked(privateCalendar, "Calendar reminder", [subject, location]);
      await capture("calendar-appointment-locked-reminder-private");
      await clickLabel("Open reminder appointment"); await isRoute({App:"Calendar"});
      await control(element => button(element) && element.automationName === "edit", "unlocked appointment details");
      const opened = await snapshot();
      assert(opened.elements.some(element => element.text === subject));
      assert(opened.elements.some(element => element.text === location));
      assert(!opened.elements.some(element => element.name === "LockScreen"));
      await capture("calendar-appointment-unlocked-reminder-details");
    });

    await check("replacing the phone document discards a snoozed task reminder from its old epoch", async () => {
      await tick(clock.base); await load(reminderFixture); await tick(clock.base + 60);
      await control(element => element.name === "PhoneAlarm", "old document task reminder");
      await clickName("AlarmSnooze"); await clickName("AlarmDismiss"); await until(noAlert, "old reminder queue snoozed and dismissed");
      await load(fixture); await tick(clock.base + 360); assert(await noAlert());
      assert.equal(await library(), undefined, "replacement does not inherit task storage or old reminder activation");
      await capture("calendar-task-reminder-epoch-reset");
    });
  } catch (error) {
    await capture("calendar-tasks-failure").catch(() => {});
    throw error;
  } finally {
    if (frozen) await page.evaluate(`(() => {
      Date.now=globalThis.__phoneTaskClock.original;delete globalThis.__phoneTaskClock;
      phoneApp.setUtcOffset(-new Date().getTimezoneOffset()*60);phoneApp.tick(Date.now()/1000);
    })()`);
    await load(original);
  }
}
