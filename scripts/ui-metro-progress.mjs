import assert from "node:assert/strict";
import { writeFile } from "node:fs/promises";
import path from "node:path";
import { inputGeometryReady } from "./ui-input-geometry.mjs";

// These HTTP fixtures hold the real SDK task response. All loading/selection
// state comes from normal application commands; inspectView only observes it.
export async function verifyMetroProgress(ctx) {
  const { page, check, state, snapshot, until, clickLabel, clickName, edit, isRoute, capture, output } = ctx;
  const original = await state();
  const fixture = structuredClone(original);
  const podcasts = "org.ferroui.podcasts";
  Object.assign(fixture.settings, {
    lock_timeout: 0, reduced_motion: false, battery_saver: false, live_tiles: false,
    dark: true, high_contrast: false, text_scale: 1, sounds: false, accent: "#1BA1E2",
  });
  fixture.events = [];
  fixture.alarms = [];
  fixture.app_data ||= {};
  delete fixture.app_data["phone.weather"];
  delete fixture.app_data[podcasts];
  fixture.installed = [...new Set([...fixture.installed, "Weather"])];
  fixture.disabled_extensions = (fixture.disabled_extensions || []).filter(id => id !== podcasts);
  const geocoding = { results: [{ id: 3089424, name: "Raciborz", country: "Poland", latitude: 50.091, longitude: 18.219 }] };
  const weather = {
    current: { temperature_2m: 10, apparent_temperature: 8, relative_humidity_2m: 75, wind_speed_10m: 12, weather_code: 3 },
    hourly: { time: ["2026-11-23T10:00"], temperature_2m: [10], precipitation_probability: [20], weather_code: [3] },
    daily: { time: ["2026-11-23"], temperature_2m_min: [2], temperature_2m_max: [12], precipitation_probability_max: [20], weather_code: [3] },
  };
  const feed = '<rss version="2.0"><channel><title>Metro progress podcast</title><description>Controlled loading fixture</description><item><title>Progress episode</title><enclosure url="https://example.org/progress-episode.mp3" type="audio/mpeg"/></item></channel></rss>';
  const load = async data => {
    await page.evaluate(`phoneApp.importData(${JSON.stringify(JSON.stringify(data))})`);
    await isRoute("Start");
  };
  const mock = () => page.evaluate(`(() => {
    if (globalThis.phoneProgressFixture) throw new Error("Progress fixture already installed");
    const fixture = { fetch: globalThis.fetch, pending: [] };
    globalThis.phoneProgressFixture = fixture;
    globalThis.fetch = async (input, options) => {
      const url = new URL(typeof input === "string" ? input : input.url);
      if (url.hostname === "geocoding-api.open-meteo.com")
        return new Response(${JSON.stringify(JSON.stringify(geocoding))}, {headers: {"content-type": "application/json"}});
      const kind = url.hostname === "api.open-meteo.com" ? "weather"
        : url.href === "https://example.org/metro-progress.xml" ? "podcast" : null;
      if (!kind) return fixture.fetch(input, options);
      return new Promise((resolve, reject) => {
        const signal = options?.signal || (typeof input === "object" ? input.signal : undefined);
        if (signal?.aborted) { reject(new DOMException("Aborted", "AbortError")); return; }
        const pending = { kind, resolve, reject, signal };
        pending.abort = () => {
          fixture.pending = fixture.pending.filter(value => value !== pending);
          reject(new DOMException("Aborted", "AbortError"));
        };
        signal?.addEventListener("abort", pending.abort, {once: true});
        fixture.pending.push(pending);
      });
    };
    return true;
  })()`);
  const release = kind => page.evaluate(`(() => {
    const fixture = globalThis.phoneProgressFixture;
    const selected = fixture.pending.filter(value => value.kind === ${JSON.stringify(kind)});
    fixture.pending = fixture.pending.filter(value => value.kind !== ${JSON.stringify(kind)});
    for (const pending of selected) {
      pending.signal?.removeEventListener("abort", pending.abort);
      pending.resolve(new Response(${JSON.stringify(kind === "weather" ? JSON.stringify(weather) : feed)},
        {headers: {"content-type": ${JSON.stringify(kind === "weather" ? "application/json" : "application/rss+xml")}}}));
    }
    return selected.length;
  })()`);
  const unmock = async () => {
    await page.evaluate(`(() => {
      const fixture = globalThis.phoneProgressFixture;
      if (!fixture) return;
      globalThis.fetch = fixture.fetch;
      for (const pending of fixture.pending) {
        pending.signal?.removeEventListener("abort", pending.abort);
        pending.reject(new DOMException("Fixture disposed", "AbortError"));
      }
      delete globalThis.phoneProgressFixture;
    })()`);
  };
  const pending = kind => until(() => page.evaluate(
    `globalThis.phoneProgressFixture.pending.some(value => value.kind === ${JSON.stringify(kind)})`),
  `${kind} operation waits for an actual host response`);
  const strip = async name => until(async () => {
    const density = await page.evaluate("devicePixelRatio");
    const view = await snapshot();
    if (!inputGeometryReady(view)) return;
    const bar = view.elements.find(element => element.name === name);
    const root = view.elements.find(element => element.name === "PhoneProgressRoot");
    const dots = view.elements.filter(element => /^PhoneProgressDot[0-4]$/.test(element.name));
    const phone = view.elements.find(element => element.name === "PhoneScreen");
    if (!bar || !root || dots.length !== 5 || !phone || root.width <= 0) return;
    assert(root.x >= phone.x - 0.1 && root.x + root.width <= phone.x + phone.width + 0.1,
      "the progress strip fits the phone surface");
    assert(root.y >= phone.y && root.y + root.height <= phone.y + phone.height,
      "the operation's progress strip is visible in its page");
    for (const dot of dots) {
      const expected = Math.ceil(4 * 400 / 480 * density) / density;
      assert(Math.abs(dot.width - expected) < 0.05 && Math.abs(dot.height - expected) < 0.05,
        "native activity dots pixel-snap the original scaled square geometry");
      assert.equal(dot.hit, false, "activity never takes page input");
    }
    return { root, dots };
  }, `${name} renders five phone activity dots`);
  const frames = duration => page.evaluate(`new Promise((resolve, reject) => {
    const start = performance.now(), frames = [];
    const sample = () => {
      try {
        const view = JSON.parse(phoneApp.inspectView());
        const root = view.elements.find(element => element.name === "PhoneProgressRoot");
        const dots = view.elements.filter(element => /^PhoneProgressDot[0-4]$/.test(element.name))
          .map(element => ({name: element.name, x: element.x, y: element.y, width: element.width, height: element.height, opacity: element.opacity}));
        frames.push({elapsed: performance.now() - start, root, dots});
        if (performance.now() - start >= ${duration}) resolve(frames);
        else requestAnimationFrame(sample);
      } catch (error) { reject(error); }
    };
    sample();
  })`);
  const inactive = name => until(async () => {
    const elements = (await snapshot()).elements;
    const root = elements.find(element => element.name === "PhoneProgressRoot");
    return elements.some(element => element.name === name) && root?.opacity === 0 &&
      !elements.some(element => /^PhoneProgressDot[0-4]$/.test(element.name));
  }, "finished activity retains its transparent slot and releases the visible dots");
  const unmounted = () => until(async () => !(await snapshot()).elements.some(element =>
    element.name === "PhoneProgressRoot" || /^PhoneProgressDot[0-4]$/.test(element.name)),
  "leaving the app detaches its progress visual tree");

  // Native Visual.Opacity is the base value, not compositor animation readback.
  // Inspect actual PNG pixels to establish the wrapper's intermediate opacity;
  // the strip is clipped, uniformly accent-coloured, and sits on the black page.
  const brightness = (image, root, viewport) => {
    const sx = image.width / viewport.width, sy = image.height / viewport.height;
    let peak = 0;
    for (let y = Math.ceil(root.y * sy); y < Math.floor((root.y + root.height) * sy); y++)
      for (let x = Math.ceil(root.x * sx); x < Math.floor((root.x + root.width) * sx); x++)
        peak = Math.max(peak, ...image.pixel(x, y));
    return peak;
  };

  await load(fixture);
  await mock();
  try {
    await check("Weather loading renders original five-dot motion and releases it after the real SDK response", async () => {
      await page.evaluate('phoneApp.openApp("Weather")');
      await isRoute({ App: "Weather" });
      await clickLabel("add city");
      await edit("WeatherCitySearch", "Raciborz");
      await clickLabel("search cities");
      await clickLabel("Raciborz, Poland");
      await pending("weather");
      const { root } = await strip("WeatherProgress");
      const observed = await frames(4600);
      assert(observed.length >= 8, "the browser must render multiple activity frames");
      assert(observed.every(frame => frame.root && frame.dots.length === 5),
        "all five retained dots remain mounted throughout the pending operation");
      for (let index = 0; index < 5; index++) {
        const positions = observed.map(frame => frame.dots.find(dot => dot.name === `PhoneProgressDot${index}`).x);
        assert(Math.max(...positions) - Math.min(...positions) > root.width * 0.65,
          `dot ${index} must actually move across the rendered strip`);
      }
      await until(async () => {
        const {root, dots} = await strip("WeatherProgress");
        return dots.filter(dot => dot.x > root.x + root.width * 0.2 && dot.x < root.x + root.width * 0.8).length >= 3;
      }, "moving progress dots occupy the visible strip");
      await capture("metro-progress-weather-moving");
      assert.equal(await release("weather"), 1);
      await until(async () => !!(await state()).app_data?.["phone.weather"]?.cities[0]?.forecast,
        "the held weather response commits a forecast");
      // The forecast replaces the welcome body and can move its footer below
      // the viewport. Scroll the real page to inspect that retained idle slot.
      await page.wheel(root.x + root.width / 2, root.y, 0, 10000);
      await inactive("WeatherProgress");
    });

    await check("Reduced-motion podcast activity stays static, survives app switching and clears on completion", async () => {
      const reduced = structuredClone(fixture);
      reduced.settings.reduced_motion = true;
      await load(reduced);
      const launch = async () => {
        assert.equal(await page.evaluate(`phoneApp.launchApp(${JSON.stringify(podcasts)}, "")`), true);
        await isRoute({ Extension: { app: podcasts, path: "" } });
      };
      await launch();
      await clickLabel("subscribe");
      await edit("FormField0", "https://example.org/metro-progress.xml");
      await clickLabel("subscribe");
      await pending("podcast");
      const { root } = await strip("PodcastProgress");
      // Leave the command that submitted the form so its hover tooltip cannot
      // cover the application bar in the activity capture.
      await page.mouseMove(root.x + root.width / 2, root.y + root.height / 2);
      const observed = await frames(400);
      assert(observed.length >= 3);
      for (let index = 0; index < 5; index++) {
        const dots = observed.map(frame => frame.dots.find(dot => dot.name === `PhoneProgressDot${index}`));
        assert(dots.every(dot => dot && dot.opacity === 1 && dot.x >= root.x && dot.x + dot.width <= root.x + root.width));
        assert(Math.max(...dots.map(dot => dot.x)) - Math.min(...dots.map(dot => dot.x)) < 0.1,
          "reduced motion must not advance dot translations");
      }
      await capture("metro-progress-podcast-reduced-motion");
      await clickName("HardwareStart");
      await isRoute("Start");
      await unmounted();
      await launch();
      await strip("PodcastProgress");
      assert.equal(await release("podcast"), 1);
      await until(async () => (await state()).app_data?.[podcasts]?.shows?.length === 1,
        "the held podcast response commits one show");
      await inactive("PodcastProgress");
      await clickLabel("refresh");
      await pending("podcast");
      await strip("PodcastProgress");
      assert.equal(await release("podcast"), 1);
      await inactive("PodcastProgress");
      assert.equal((await state()).app_data[podcasts].shows.length, 1,
        "refresh updates the existing podcast without adding another show");
    });

    await check("Podcast activity fades on the compositor, retains moving dots through exit and restarts before an old completion", async () => {
      const enabled = await state();
      enabled.settings.reduced_motion = false;
      await load(enabled);
      const show = enabled.app_data[podcasts].shows[0];
      const destination = { Extension: { app: podcasts, path: `show/${show.id}` } };
      assert.equal(await page.evaluate(`phoneApp.launchApp(${JSON.stringify(podcasts)}, ${JSON.stringify(destination.Extension.path)})`), true);
      await isRoute(destination);
      await inactive("PodcastProgress");
      await clickLabel("refresh");
      await pending("podcast");
      await until(async () => (await strip("PodcastProgress")).root.opacity === 1,
        "the activity's 250 ms entrance reaches its normal state");
      const active = await strip("PodcastProgress");
      await page.mouseMove(active.root.x + active.root.width / 2, active.root.y + active.root.height / 2);
      const full = await capture("metro-progress-podcast-active");
      const view = await snapshot();
      const peak = brightness(full, active.root, view);
      assert(peak > 100, "pending work actually paints full-intensity accent dots");

      const sampled = frames(1150);
      const action = async () => {
        assert.equal(await release("podcast"), 1);
        const faded = await capture("metro-progress-podcast-fading-out");
        const during = await strip("PodcastProgress");
        const fadedPeak = brightness(faded, during.root, view);
        assert(fadedPeak > peak * 0.1 && fadedPeak < peak * 0.95,
          `the compositor must visibly fade the whole strip: ${fadedPeak}/${peak}`);
        assert.equal(during.root.opacity, 1,
          "the native base opacity stays intact until compositor exit completion");
        // Starting another real fetch while the exit is still active must
        // cancel the old completion and preserve the moving strip's lifetime.
        await clickLabel("refresh");
        await pending("podcast");
        await strip("PodcastProgress");
      };
      const [recorded, acted] = await Promise.allSettled([sampled, action()]);
      if (output && recorded.status === "fulfilled")
        await writeFile(path.join(output, "metro-progress-podcast-restart-frames.json"), JSON.stringify(recorded.value, null, 2));
      if (acted.status === "rejected") throw acted.reason;
      if (recorded.status === "rejected") throw recorded.reason;
      assert(recorded.value.length >= 8);
      assert(recorded.value.every(frame => frame.root && frame.dots.length === 5),
        "the retained dots must survive response completion and a rapid restart without blinking out");
      assert.equal(recorded.value.at(-1).root.opacity, 1);
      await pending("podcast");
      await strip("PodcastProgress");
      assert.equal(await release("podcast"), 1);
      await inactive("PodcastProgress");
      const hidden = await capture("metro-progress-podcast-idle");
      assert.equal(brightness(hidden, active.root, view), 0,
        "the settled empty slot paints no track or activity dots");
    });
  } catch (error) {
    await capture("metro-progress-failure").catch(() => {});
    throw error;
  } finally {
    await unmock();
    await load(original);
    // Completion status is transient, so restoring the document does not
    // remove it. Dismiss its actual command before the next visual fixture.
    const status = element => /Button$/.test(element.type) && element.automationName === "×";
    if ((await snapshot()).elements.some(status)) await clickLabel("×");
    await until(async () => !(await snapshot()).elements.some(status),
      "progress fixture releases its completion message");
  }
}
