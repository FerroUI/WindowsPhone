import assert from 'node:assert/strict';

// Fixture-only collection workflows. These checks do not play media, contact a
// service, open host file pickers or request device permissions.
export async function verifyMediaLibraries(ctx) {
  const {page, check, state, snapshot, until, capture, clickName, clickLabel,
    textPresent, isRoute, reload, edit} = ctx;
  const original = await state();
  const fixture = structuredClone(original);
  Object.assign(fixture.settings, {lock_timeout: 0, reduced_motion: true});
  fixture.events = []; fixture.alarms = [];
  for (const app of ['Music', 'Video', 'Photos']) if (!fixture.installed.includes(app)) fixture.installed.push(app);
  fixture.app_data ||= {};
  for (const id of ['phone.music', 'phone.video', 'phone.photos']) delete fixture.app_data[id];
  const base = fixture.next_id;
  fixture.next_id += 6;
  const songA = base, songB = base + 1, clip = base + 2, photoA = base + 3, photoB = base + 4;
  fixture.tracks = [
    {id: songA, title: 'Library song A', artist: 'Test artist', source: 'media:library-test-a', duration_seconds: 0, media_kind: 'audio'},
    {id: songB, title: 'Library song B', artist: 'Test artist', source: 'media:library-test-b', duration_seconds: 0, media_kind: 'audio'},
    {id: clip, title: 'Library video', artist: '', source: 'media:library-test-video', duration_seconds: 0, media_kind: 'video'},
  ];
  fixture.photos = [
    {id: photoA, name: 'Album photo A', source: 'forest', favorite: false},
    {id: photoB, name: 'Album photo B', source: 'forest', favorite: true},
  ];
  const load = async data => {
    await page.evaluate(`phoneApp.importData(${JSON.stringify(JSON.stringify(data))})`);
    await isRoute('Start');
  };
  const open = async app => {
    await page.evaluate(`phoneApp.openApp(${JSON.stringify(app)})`);
    await isRoute({App: app});
  };
  const saved = async id => (await state()).app_data?.[id];
  await load(fixture);
  try {
    await check('Music playlists select, reorder, rename and survive a real reload without changing media files', async () => {
      await open('Music');
      await clickLabel('playlists'); await clickLabel('new playlist');
      await edit('FormField0', 'Road trip'); await clickLabel('save');
      await clickLabel('Library song B'); await clickLabel('Library song A'); await clickLabel('done');
      await until(async () => (await saved('phone.music'))?.playlists.groups[0]?.items.length === 2, 'playlist selection committed');
      assert.deepEqual((await saved('phone.music')).playlists.groups[0].items, [songA, songB]);
      await clickLabel('move up: Library song B');
      await until(async () => (await saved('phone.music')).playlists.groups[0].items[0] === songB, 'playlist reordered');
      await clickLabel('rename'); await edit('FormField0', 'Commute'); await clickLabel('save');
      await capture('music-persisted-playlist');
      const expected = await saved('phone.music');
      await reload(); await open('Music'); await clickLabel('playlists'); await clickLabel('Commute');
      assert.deepEqual(await saved('phone.music'), expected);
      await textPresent('Commute');
      await clickLabel('delete'); await clickLabel('confirm');
      await until(async () => !(await saved('phone.music')).playlists.groups.length, 'playlist deleted');
      assert.deepEqual((await state()).tracks, fixture.tracks, 'deleting a playlist never removes or rewrites files');
    });
    await check('Video playlists use an independent namespace and expose only video selections', async () => {
      await open('Video'); await clickLabel('playlists'); await clickLabel('new playlist');
      await edit('FormField0', 'Watch later'); await clickLabel('save');
      assert(!(await snapshot()).elements.some(element => element.hit && element.automationName === 'Library song A'));
      await clickLabel('Library video'); await clickLabel('done');
      await until(async () => (await saved('phone.video'))?.playlists.groups[0]?.items[0] === clip, 'video playlist saved');
      assert.deepEqual((await saved('phone.video')).playlists.groups[0].items, [clip]);
      assert.equal((await saved('phone.music')).playlists.groups.length, 0);
      await capture('video-persisted-playlist');
    });
    await check('Photos albums select, rename photos, survive restore, and remove membership without deleting originals', async () => {
      await open('Photos'); await clickLabel('albums'); await clickLabel('more'); await clickLabel('new album');
      await edit('FormField0', 'Family'); await clickLabel('save');
      await clickLabel('Album photo A'); await clickLabel('Album photo B'); await clickLabel('done');
      await until(async () => (await saved('phone.photos'))?.albums.groups[0]?.items.length === 2, 'album saved');
      assert.deepEqual((await saved('phone.photos')).albums.groups[0].items, [photoA, photoB]);
      await capture('photos-persisted-album');
      await clickLabel('Album photo A'); await clickLabel('more'); await clickLabel('rename photo');
      await edit('FormField0', 'Family portrait'); await clickLabel('save');
      await until(async () => (await state()).photos.find(photo => photo.id === photoA)?.name === 'Family portrait', 'photo renamed');
      await clickName('HardwareBack');
      const roundtrip = await state(); await load(roundtrip); await open('Photos'); await clickLabel('albums'); await clickLabel('Family');
      assert.deepEqual(await saved('phone.photos'), roundtrip.app_data['phone.photos']);
      await clickLabel('Family portrait'); await clickLabel('more'); await clickLabel('remove from album');
      await until(async () => (await saved('phone.photos')).albums.groups[0].items.length === 1, 'only album membership removed');
      assert.deepEqual((await state()).photos, roundtrip.photos);
      await clickLabel('more'); await clickLabel('delete album'); await clickLabel('confirm');
      await until(async () => !(await saved('phone.photos')).albums.groups.length, 'album deleted');
      assert.deepEqual((await state()).photos, roundtrip.photos, 'album deletion preserves all original photos');
      await capture('photos-after-album-delete');
    });
  } finally {
    await load(original);
  }
}
