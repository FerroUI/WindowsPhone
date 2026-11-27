import assert from 'node:assert/strict';

// Native Settings rendering of controlled host observations. This fixture uses
// the production observer with an injected platform; it never requests actual
// permissions, displays system notifications, or measures the CI host's disk.
export async function verifyHostRefinements(ctx) {
  const {page, check, state, snapshot, capture, button, centre, control,
    clickLabel, textPresent, reveal, isRoute} = ctx;
  const original = await state();
  const environment = await page.evaluate('JSON.parse(phoneApp.hostState()).environment');
  assert(environment, 'hostState exposes the observed environment for exact fixture restoration');
  const fixture = structuredClone(original);
  Object.assign(fixture.settings, {lock_timeout: 0, reduced_motion: true, notifications: true});
  fixture.alarms = [];
  fixture.events = [];
  const load = async data => {
    await page.evaluate(`phoneApp.importData(${JSON.stringify(JSON.stringify(data))})`);
    await isRoute('Start');
  };
  const scrollMatch = element => /ScrollViewer$/.test(element.type) && element.width > 220 && element.height > 160;
  const setting = async name => {
    await page.evaluate('phoneApp.openApp("Settings")');
    await isRoute({App: 'Settings'});
    await clickLabel('system');
    await page.wheel(...centre(await control(element => element.name === 'SettingsSystemScroll', 'Settings system pane')), 0, -100000);
    const row = await reveal(element => button(element) && element.automationName === name,
      `${name} settings entry`, element => element.name === 'SettingsSystemScroll', 'Settings system pane');
    await page.click(...centre(row));
    await isRoute({Setting: name});
    await page.wheel(...centre(await control(scrollMatch, `${name} scroll viewer`)), 0, 100000);
  };
  await load(fixture);
  try {
    await page.evaluate(`(async()=>{
      const observer=await import('./environment.js');
      const platform=new EventTarget(); platform.document=new EventTarget(); platform.document.hidden=false;
      let persisted=false;
      platform.Notification={permission:'default'};
      platform.navigator={onLine:true,mediaSession:{},storage:{
        estimate:async()=>({usage:13107200,quota:67108864}),
        persisted:async()=>persisted,
        persist:()=>{throw new Error('The observation fixture must not request storage permission.');}
      }};
      const dispose=observer.observeEnvironment(phoneApp,platform);
      globalThis.__phoneHostSmoke={dispose,observer,change:(permission,value)=>{
        platform.Notification.permission=permission;persisted=value;observer.refreshEnvironment();
      }};
    })()`);
    const committed = await state();
    const saved = await page.evaluate("localStorage.getItem('ferroui.windows-phone.v1')");

    await check('native Storage Sense distinguishes observed origin quota and persistent storage', async () => {
      await setting('storage sense');
      await textPresent('Device app storage');
      await textPresent('12.50 MB');
      await textPresent('Available site quota: 64.00 MB');
      await control(element => button(element) && element.automationName === 'keep app data on this device',
        'explicit persistence command');
      await control(element => button(element) && element.automationName === 'refresh storage information',
        'read-only storage refresh command');
      await capture('host-native-storage-observations');
      await page.evaluate("__phoneHostSmoke.change('default',true)");
      await textPresent('App data is stored persistently. Keep a separate backup to recover deleted data.');
      assert(!(await snapshot()).elements.some(element => button(element) &&
        element.automationName === 'keep app data on this device'), 'persistent storage does not offer another request');
      assert.equal(await page.evaluate("localStorage.getItem('ferroui.windows-phone.v1')"), saved,
        'host observations must not write phone preferences');
    });

    await check('native notification settings distinguish unrequested and blocked host permissions', async () => {
      await setting('notifications+actions');
      await textPresent('Device notifications');
      await textPresent('permission not requested');
      await control(element => button(element) && element.automationName === 'allow device notifications',
        'explicit notification permission command');
      assert(!(await snapshot()).elements.some(element => button(element) &&
        element.automationName === 'send a test notification'));
      await capture('host-native-notification-permission');
      await page.evaluate("__phoneHostSmoke.change('denied',true)");
      await textPresent('blocked in host settings');
      assert(!(await snapshot()).elements.some(element => button(element) &&
        ['allow device notifications', 'send a test notification'].includes(element.automationName)),
        'blocked permission offers neither a repeated prompt nor a test notification');
      assert.deepEqual(await state(), committed, 'observing host permissions must not change the phone document');
      assert.equal(await page.evaluate("localStorage.getItem('ferroui.windows-phone.v1')"), saved);
    });
  } catch (error) {
    await capture('host-refinements-failure').catch(() => {});
    throw error;
  } finally {
    try { await load(original); }
    finally {
      await page.evaluate(`(async()=>{
        globalThis.__phoneHostSmoke?.dispose();
        phoneApp.setEnvironment(${JSON.stringify(JSON.stringify(environment))});
        const observer=await import('./environment.js');observer.observeEnvironment(phoneApp);
        delete globalThis.__phoneHostSmoke;
      })()`);
    }
  }
}
