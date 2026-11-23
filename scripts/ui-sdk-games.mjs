import assert from 'node:assert/strict';

// Native input enters independently registered game crates. All data is an
// isolated local fixture; no account, network game service or host UI is used.
export async function verifySdkGames(ctx) {
  const {page, check, state, snapshot, until, capture, clickName, clickLabel,
    textPresent, home, isRoute, reload, button, centre, control, reveal, edit} = ctx;
  const original = await state();
  const fixture = structuredClone(original);
  Object.assign(fixture.settings, {lock_timeout: 0, reduced_motion: true, dark: true,
    high_contrast: false, accent: '#1BA1E2', brightness: 0.8});
  fixture.alarms = []; fixture.events = [];
  fixture.app_data ||= {};
  const tic = 'org.ferroui.tic-tac-toe', mines = 'org.ferroui.minesweeper';
  fixture.disabled_extensions = (fixture.disabled_extensions || []).filter(id => id !== tic && id !== mines);
  const oldLinks = new Set((fixture.app_links || []).filter(link => [tic, mines].includes(link.app)).map(link => link.id));
  fixture.app_links = (fixture.app_links || []).filter(link => !oldLinks.has(link.id));
  const pruneTiles = tiles => tiles.flatMap(tile => {
    if (oldLinks.has(tile.target?.AppLink)) return [];
    if (tile.folder_tiles?.length) {
      tile.folder_tiles = pruneTiles(tile.folder_tiles);
      tile.children = tile.folder_tiles.map(child => child.app);
      if (!tile.folder_tiles.length) return [];
    }
    return [tile];
  });
  fixture.tiles = pruneTiles(fixture.tiles);
  delete fixture.app_data[tic]; delete fixture.app_data[mines];
  const load = async data => {
    await page.evaluate(`phoneApp.importData(${JSON.stringify(JSON.stringify(data))})`);
    await isRoute('Start');
  };
  const launch = async id => {
    await page.evaluate('phoneApp.openApp("Games")');
    await isRoute({App: 'Games'});
    await clickLabel('collection');
    await clickName(`GameLaunch-${id}`);
    await isRoute({Extension: {app: id, path: ''}});
  };
  const saved = async id => (await state()).app_data?.[id];
  const localSearch = async query => {
    await clickName('HardwareSearch');
    await isRoute('Search');
    await edit('CortanaInput', query);
  };
  const cellColor = async (image, name, rgb, label) => {
    const view = await snapshot();
    const cell = view.elements.find(element => element.name === name);
    assert(cell, `${label}: native cell geometry is available`);
    const sx = image.width / view.width, sy = image.height / view.height;
    const left = Math.ceil((cell.x + 2) * sx), right = Math.floor((cell.x + cell.width - 2) * sx);
    const top = Math.ceil((cell.y + 2) * sy), bottom = Math.floor((cell.y + cell.height - 2) * sy);
    assert(left >= 0 && top >= 0 && right <= image.width && bottom <= image.height,
      `${label}: sample stays inside the rendered cell`);
    let count = 0, minX = right, maxX = left, minY = bottom, maxY = top;
    for (let y = top; y < bottom; y++) for (let x = left; x < right; x++) {
      if (image.pixel(x, y).every((channel, index) => Math.abs(channel - rgb[index]) <= 24)) {
        count++; minX = Math.min(minX, x); maxX = Math.max(maxX, x);
        minY = Math.min(minY, y); maxY = Math.max(maxY, y);
      }
    }
    const minimum = Math.ceil(Math.max(20, cell.width * cell.height * 0.02) * sx * sy);
    assert(count >= minimum, `${label}: expected at least ${minimum} full-contrast symbol pixels, found ${count}`);
    assert(maxX - minX >= cell.width * sx * 0.3 && maxY - minY >= cell.height * sy * 0.35,
      `${label}: the visible symbol must occupy the cell, not just isolated pixels`);
  };
  await load(fixture);
  try {
    await check('SDK rejects unregistered and malformed application activation without mutation', async () => {
      const before = await state();
      assert.equal(await page.evaluate('phoneApp.launchApp("org.ferroui.not-installed", "")'), false);
      assert.equal(await page.evaluate(`phoneApp.launchApp(${JSON.stringify(tic)}, "\\u0000")`), false);
      await isRoute('Start');
      assert.deepEqual(await state(), before);
    });

    await check('registered Tic tac toe supports local search, native play, exact score persistence and turn undo', async () => {
      await launch(tic);
      const beforeSearch = await state();
      await localSearch('find Tic tac toe');
      await clickLabel('Tic tac toe');
      await isRoute({Extension: {app: tic, path: ''}});
      await localSearch('open Tic tac toe');
      await clickLabel('search');
      await isRoute({Extension: {app: tic, path: ''}});
      assert.deepEqual(await state(), beforeSearch, 'local discovery and open commands do not change app data');
      await clickLabel('more'); await clickLabel('two players');
      await until(async () => (await saved(tic))?.game.mode === 'TwoPlayers', 'two-player mode saved');
      for (const index of [0, 3, 1, 4, 2]) await clickName(`TicTacToeCell${index}`);
      await until(async () => (await saved(tic))?.local_scores[0] === 1, 'single completed local win');
      await textPresent('X wins');
      const completed = await saved(tic);
      assert.deepEqual(completed.game.moves, [0, 3, 1, 4, 2]);
      assert.deepEqual(completed.local_scores, [1, 0, 0]);
      const completedImage = await capture('sdk-tic-tac-toe-completed-game');
      await cellColor(completedImage, 'TicTacToeCell0', [27, 161, 226], 'completed X remains accent colored');
      await cellColor(completedImage, 'TicTacToeCell3', [255, 255, 255], 'completed O remains white');
      await reload(); await launch(tic);
      assert.deepEqual(await saved(tic), completed, 'board and scores survive a real reload');
      await textPresent('X wins');
      await clickLabel('more'); await clickLabel('against phone');
      await clickName('TicTacToeCell0');
      await until(async () => (await saved(tic))?.game.moves.length === 2, 'complete phone reply');
      await clickLabel('undo');
      await until(async () => (await saved(tic))?.game.moves.length === 0, 'undo removes both plies');
      assert.deepEqual((await saved(tic)).local_scores, [1, 0, 0], 'mode change and undo preserve completed scores');
      await home();
    });

    await check('registered Minesweeper provides safe first reveal, flags, transient clock and resumable state', async () => {
      await launch(mines);
      await clickName('MineCell0');
      await until(async () => (await saved(mines))?.game.first === 0, 'first reveal persisted');
      let data = await saved(mines);
      assert.equal(data.game.cells[0].mine, false);
      assert.equal(data.game.cells[0].adjacent, 0);
      assert.equal(data.game.cells.filter(cell => cell.mine).length, 10);
      assert(data.game.cells.filter(cell => cell.revealed).length > 1, 'zero-region flood fill');
      const hidden = data.game.cells.findIndex(cell => !cell.revealed);
      assert(hidden >= 0);
      await clickLabel('flag'); await clickName(`MineCell${hidden}`);
      await until(async () => (await saved(mines))?.game.cells[hidden].flagged, 'native flag action persisted');
      await clickLabel('reveal');
      data = await saved(mines);
      const bytes = await page.evaluate("localStorage.getItem('ferroui.windows-phone.v1')");
      const seconds = async () => (await snapshot()).elements.find(element => element.name === 'MinesweeperSeconds')?.text;
      const beforeTick = await seconds();
      assert.match(beforeTick, /^\d+$/, 'visible game clock is mounted');
      await until(async () => Number(await seconds()) > Number(beforeTick), 'visible game clock advances without input');
      assert.deepEqual(await saved(mines), data, 'clock does not mutate the game model');
      assert.equal(await page.evaluate("localStorage.getItem('ferroui.windows-phone.v1')"), bytes,
        'clock does not persist phone data');
      const playingImage = await capture('sdk-minesweeper-playing');
      await cellColor(playingImage, `MineCell${hidden}`, [255, 189, 56], 'native flag is visibly amber');
      await reload(); await launch(mines);
      assert.deepEqual(await saved(mines), data, 'minefield and flags survive reload');
      const restoredImage = await capture('sdk-minesweeper-restored');
      await cellColor(restoredImage, `MineCell${hidden}`, [255, 189, 56], 'restored flag remains visibly amber');
      const roundtrip = await state();
      await load(roundtrip); await launch(mines);
      assert.deepEqual(await saved(mines), data, 'phone restore retains isolated app data');
    });

    await check('SDK app Start links, local search, and Store disable/enable preserve game progress', async () => {
      await launch(tic);
      const progress = await saved(tic);
      await clickLabel('more'); await clickLabel('pin to start');
      await isRoute('Start');
      const link = await until(async () => (await state()).app_links.find(link => link.app === tic && link.path === ''),
        'canonical game app link');
      const pinned = await state();
      assert.equal(pinned.app_links.filter(item => item.app === tic && item.path === '').length, 1);
      assert.equal(link.title, 'Tic tac toe');
      const tile = pinned.tiles.find(tile => tile.target?.AppLink === link.id);
      assert(tile, 'Start tile targets the canonical app link');
      assert.deepEqual(await saved(tic), progress, 'pinning preserves the board and scores');
      await page.wheel(...centre(await control(element => element.name === 'StartScroll', 'Start scroll viewer')), 0, -100000);
      const target = await reveal(element => element.name === `Tile-${tile.id}`, 'pinned game tile',
        element => element.name === 'StartScroll', 'Start scroll viewer');
      await page.click(...centre(target));
      await isRoute({Extension: {app: tic, path: ''}});

      await page.evaluate('phoneApp.openApp("Store")');
      await isRoute({App: 'Store'});
      await edit('StoreSearch', 'Tic tac toe');
      await clickLabel('Tic tac toe');
      const scroll = element => /ScrollViewer$/.test(element.type) && element.width > 220 && element.height > 250;
      const disable = await reveal(element => button(element) && element.automationName === 'disable',
        'disable game', scroll, 'Store detail scroll viewer');
      await page.click(...centre(disable)); await clickLabel('save');
      await until(async () => (await state()).disabled_extensions.includes(tic), 'game disabled through Store');
      await until(async () => !(await snapshot()).elements.some(element => element.hit && button(element) && element.automationName === 'save'),
        'disable confirmation dismissed');
      const disabled = await state();
      assert(!disabled.app_links.some(item => item.app === tic), 'disable removes app links');
      assert(!disabled.tiles.some(item => item.id === tile.id || item.target?.AppLink === link.id), 'disable removes the pinned tile');
      assert.deepEqual(disabled.app_data[tic], progress, 'disable retains saved progress');
      assert.equal(await page.evaluate(`phoneApp.launchApp(${JSON.stringify(tic)}, "")`), false);
      await isRoute({App: 'Store'});
      assert.deepEqual(await state(), disabled, 'unavailable activation cannot mutate data');
      await page.wheel(...centre(await control(scroll, 'Store detail scroll viewer')), 0, -100000);
      await capture('sdk-game-store-disabled');

      await localSearch('find Tic tac toe');
      await control(element => button(element) && element.automationName === 'search the web', 'local search results rendered');
      assert(!(await snapshot()).elements.some(element => button(element) && element.automationName === 'Tic tac toe'),
        'disabled game is absent from local search results');
      await edit('CortanaInput', 'open Tic tac toe');
      await clickLabel('search');
      await textPresent('0 matches on your phone. Choose a result, or search the web.');
      await isRoute('Search');
      assert(!(await snapshot()).elements.some(element => button(element) && element.automationName === 'Tic tac toe'),
        'disabled local open cannot reveal an app result');
      assert.deepEqual(await state(), disabled, 'disabled local discovery and open cannot mutate the phone');

      await page.evaluate('phoneApp.openApp("Store")');
      await isRoute({App: 'Store'});
      await clickLabel('enable');
      await until(async () => !(await state()).disabled_extensions.includes(tic), 'game re-enabled through Store');
      assert.deepEqual(await saved(tic), progress, 'enable restores access to the same progress');
      assert(!(await state()).app_links.some(item => item.app === tic), 'enable does not recreate removed Start links');
      await clickLabel('open');
      await isRoute({Extension: {app: tic, path: ''}});
      assert.deepEqual(await saved(tic), progress);
    });
  } catch (error) {
    await capture('sdk-games-failure').catch(() => {});
    throw error;
  } finally { await load(original); }
}
