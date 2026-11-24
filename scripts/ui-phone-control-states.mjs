import assert from "node:assert/strict";

// Production app surfaces, real browser pointer/key input, and observed native
// state. Read-only/disabled/mixed states that these app pages do not expose are
// covered by the mounted Rust regressions; this script adds no mutation hook.
export async function verifyPhoneControlStates(ctx) {
  const { page, check, state, snapshot, until, capture, control, centre,
    clickName, clickLabel, edit, isRoute } = ctx;
  const original = await state();
  const fixture = structuredClone(original);
  Object.assign(fixture.settings, { lock_timeout: 0, reduced_motion: true,
    battery_saver: false, high_contrast: false, text_scale: 1, time_24h: true,
    sounds: false, ringer_volume: 0, vibration: false, live_tiles: false });
  fixture.alarms = [];
  fixture.events = [];
  fixture.disabled_extensions = [];
  fixture.installed = [...new Set([...fixture.installed, "Alarms"])];
  const frame = () => page.evaluate(
    "new Promise(resolve=>requestAnimationFrame(()=>requestAnimationFrame(()=>resolve(true))))");
  const named = (view, name) => view.elements.find(element => element.name === name);
  const within = (part, parent) => {
    const [x, y] = centre(part);
    return x >= parent.x && x <= parent.x + parent.width &&
      y >= parent.y && y <= parent.y + parent.height;
  };
  const sameColor = (actual, expected, message) => {
    const normalize = value => value?.toLowerCase().replace(/^#ff(?=[0-9a-f]{6}$)/, "#");
    assert.equal(normalize(actual), normalize(expected), message);
  };
  const near = (actual, expected, message) =>
    assert(Math.abs(actual - expected) <= 0.8, `${message}: ${actual} versus ${expected}`);
  const sameBounds = (actual, expected, message) => {
    for (const axis of ["x", "y", "width", "height"])
      near(actual[axis], expected[axis], `${message} (${axis})`);
  };
  const onePart = (view, parent, name) => {
    const matches = view.elements.filter(element => element.name === name && within(element, parent));
    assert.equal(matches.length, 1, `${parent.automationName || parent.name} has one visible ${name}`);
    return matches[0];
  };
  const day = async (selected, focused) => control(element => /CheckBox$/.test(element.type) &&
    element.automationName === `Monday, ${selected ? "selected" : "not selected"}` &&
    (focused === undefined || element.focused === focused), "Monday native checkbox");
  const load = async document => {
    await page.evaluate(`phoneApp.importData(${JSON.stringify(JSON.stringify(document))})`);
    await isRoute("Start");
    await frame();
  };
  let heldPointer;
  try {
    for (const dark of [true, false]) {
      fixture.settings.dark = dark;
      await load(fixture);
      const palette = dark ? "dark" : "light";
      const foreground = dark ? "#FFFFFF" : "#000000";
      const background = dark ? "#000000" : "#FFFFFF";
      const label = `Metro ${palette} state fixture`;
      await check(`${palette} phone text input keeps native editing, undo and fixed focus bounds`, async () => {
        await page.evaluate("phoneApp.openApp('Alarms')");
        await isRoute({ App: "Alarms" });
        await clickLabel("add");
        const input = await control(element => element.name === "AlarmName", "Alarm name native text input");
        const initial = input.text;
        const normalView = await snapshot();
        const border = onePart(normalView, input, "BorderElement");
        sameColor(border.background, "#D6D6D6", "resting input uses the documented neutral adaptation");
        assert.equal(input.opacity, 1);
        await edit("AlarmName", label);
        await frame();
        const focusedView = await snapshot();
        const focused = named(focusedView, "AlarmName");
        assert(focused.focused, "the real TextBox receives pointer focus");
        sameBounds(focused, input, "focus retains input geometry");
        const focusedBorder = onePart(focusedView, focused, "BorderElement");
        sameBounds(focusedBorder, border, "focus retains the native border geometry");
        sameColor(focusedBorder.background, "#FFFFFF", "focus uses the editing surface role");
        sameColor(focused.foreground, "#000000", "editing text remains legible in either phone palette");
        assert(!focusedView.elements.some(element => element.name === "DeleteButton" && within(element, focused)),
          "the phone field does not expose the WinUI clear-button chrome");
        await page.press("z", ["ctrl"]);
        await until(async () => named(await snapshot(), "AlarmName")?.text === initial, "native TextBox undo");
        await page.press("y", ["ctrl"]);
        await until(async () => named(await snapshot(), "AlarmName")?.text === label, "native TextBox redo");
        await capture(`phone-input-${palette}-focused`);
      });

      await check(`${palette} Toolkit checkbox renders exact mark geometry and native press/focus states`, async () => {
        await clickLabel("Choose alarm repeat days");
        const unchecked = await day(false);
        assert(unchecked.height >= 44, "the visible choice remains a native touch target");
        await page.click(...centre(unchecked));
        const checked = await day(true);
        await frame();
        const normalView = await snapshot();
        const outline = onePart(normalView, checked, "PhoneChoiceOutline");
        const mark = onePart(normalView, checked, "PhoneChoiceCheck");
        near(outline.width, 32 * 400 / 480, "Toolkit checkbox width scales from the 480-pixel reference");
        near(outline.height, 32 * 400 / 480, "Toolkit checkbox height scales from the 480-pixel reference");
        near(mark.width, 23 * 400 / 480, "Toolkit check width");
        near(mark.height, 21 * 400 / 480, "Toolkit check height");
        sameColor(mark.foreground, foreground, "normal check ink follows the phone palette");

        heldPointer = centre(checked);
        await page.mouseMove(...heldPointer);
        await page.mouseDown(...heldPointer);
        const pressed = await until(async () => {
          const view = await snapshot();
          const row = view.elements.find(element => /CheckBox$/.test(element.type) &&
            element.automationName === "Monday, selected" && element.pressed);
          return row && { view, row };
        }, "actual native checkbox pressed state");
        sameBounds(pressed.row, checked, "press does not uniformly shrink the checkbox");
        assert.equal(pressed.row.opacity, 1, "press uses ink replacement instead of a subtree fade");
        sameColor(onePart(pressed.view, pressed.row, "PhoneChoiceOutline").background,
          foreground, "pressed fill uses the documented inverse-ink adaptation");
        sameColor(onePart(pressed.view, pressed.row, "PhoneChoiceCheck").foreground,
          background, "pressed check retains contrast against its fill");
        await capture(`phone-checkbox-${palette}-pressed`);

        // Releasing outside the native target cancels the pending activation.
        const screen = named(pressed.view, "PhoneScreen");
        heldPointer = [screen.x - 8, checked.y + checked.height / 2];
        await page.mouseMove(...heldPointer);
        await page.mouseUp(...heldPointer);
        heldPointer = undefined;
        await day(true);
        await page.press("Tab");
        await page.press("Tab", ["shift"]);
        const focused = await day(true, true);
        await frame();
        const focusView = await snapshot();
        const focus = onePart(focusView, focused, "PhoneChoiceFocus");
        assert.equal(focus.hitTestVisible, false, "focus decoration does not capture input");
        sameBounds(onePart(focusView, focused, "PhoneChoiceOutline"), outline,
          "keyboard focus does not move the indicator");
        await capture(`phone-checkbox-${palette}-keyboard-focus`);
        await page.press("Space");
        await day(false, true);
        await page.press("Space");
        await day(true, true);
        await clickLabel("done");
        const retained = await control(element => element.name === "AlarmName", "retained native alarm input");
        assert.equal(retained.text, label, "choice navigation preserves the edited draft");
        await clickLabel("save");
        const saved = await until(async () => (await state()).alarms.find(alarm => alarm.label === label),
          "native form committed the edited alarm");
        assert.deepEqual(saved.weekdays, [0], "pointer cancellation and keyboard toggles commit exactly one Monday selection");
      });
    }
  } catch (error) {
    await capture("phone-control-states-failure").catch(() => {});
    throw error;
  } finally {
    if (heldPointer) await page.mouseUp(...heldPointer).catch(() => {});
    await load(original);
  }
}
