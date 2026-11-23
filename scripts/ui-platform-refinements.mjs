import assert from 'node:assert/strict';

// Verify the rendered Device app against this browser's actual capability
// description. This gate never asks for an OS permission or opens a share UI.
export async function verifyPlatformRefinements(ctx) {
  const {page, check, state, snapshot, capture, clickLabel, textPresent, isRoute, button} = ctx;
  const original = await state();
  const fixture = structuredClone(original);
  Object.assign(fixture.settings, {lock_timeout: 0, reduced_motion: true});
  fixture.alarms = []; fixture.events = [];
  fixture.disabled_extensions = (fixture.disabled_extensions || []).filter(id => id !== 'org.ferroui.device');
  const load = async data => {
    await page.evaluate(`phoneApp.importData(${JSON.stringify(JSON.stringify(data))})`);
    await isRoute('Start');
  };
  await load(fixture);
  try {
    await check('Device app renders observed browser services without offering a launcher or device catalog', async () => {
      const observed = await page.evaluate('JSON.parse(phoneApp.platformState())');
      assert.equal(observed.platform, 'browser');
      assert.equal(observed.mode, 'app');
      assert.equal(observed.launcher, 'unsupported');
      assert.equal(observed.shell, 'unsupported');
      assert(!observed.features.some(feature => ['installedApps', 'launcherRole', 'shellSession', 'systemSettings'].includes(feature)));
      await page.evaluate('phoneHost.activate("ferroui-phone://app/org.ferroui.device")');
      await isRoute({Extension: {app: 'org.ferroui.device', path: ''}});
      await textPresent('web browser · application');
      await textPresent('Home role: unavailable\nShell session: unavailable');
      const view = await snapshot();
      assert(!view.elements.some(element => element.name === 'DeviceApplicationList'));
      assert(!view.elements.some(element => button(element) &&
        ['choose as home app', 'application settings', 'network settings'].includes(element.automationName)));
      for (const [feature, label] of [['notifications', 'allow notifications'], ['haptics', 'try haptic feedback']]) {
        assert.equal(view.elements.some(element => button(element) && element.automationName === label),
          observed.features.includes(feature), `${label} follows the provider's observed API support`);
      }
      await clickLabel('refresh device apps');
      await textPresent('Home role: unavailable\nShell session: unavailable');
      await capture('platform-browser-device-services');
    });
    await check('browser embedding API validates platform requests and activation routes before mutation', async () => {
      const committed = await state();
      const results = await page.evaluate(`(() => {
        const rejected = operation => {try {operation(); return false;} catch {return true;}};
        return [
          rejected(() => phoneHost.request({kind:'requestLauncher'})),
          rejected(() => phoneHost.request({kind:'launchApplication',id:'arbitrary-command'})),
          rejected(() => phoneHost.activate('file:///not-an-app')),
          rejected(() => phoneHost.activate('ferroui-phone://app/org.ferroui.device?command=anything')),
        ];
      })()`);
      assert.deepEqual(results, [true, true, true, true]);
      assert.deepEqual(await state(), committed);
      await isRoute({Extension: {app: 'org.ferroui.device', path: ''}});
    });
  } catch (error) {
    await capture('platform-refinements-failure').catch(() => {});
    throw error;
  } finally { await load(original); }
}
