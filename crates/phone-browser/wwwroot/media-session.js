// A platform adapter for one active HTML media element. Event ownership stays
// here so replacing a player cannot leave hardware keys targeting old media.
export function createMediaSession({session, Metadata, control, report = () => {}} = {}) {
  let player, removers = [];
  const actions = ['play', 'pause', 'stop', 'nexttrack', 'previoustrack', 'seekbackward', 'seekforward', 'seekto'];
  const safe = action => { try { action(); } catch {} };
  const position = () => {
    if (!session?.setPositionState) return;
    const duration = player?.duration, rate = player?.playbackRate;
    if (!Number.isFinite(duration) || duration <= 0 || !Number.isFinite(rate) || rate <= 0) {
      safe(() => session.setPositionState());
      return;
    }
    safe(() => session.setPositionState({
      duration, playbackRate: rate,
      position: Math.min(duration, Math.max(0, Number(player.currentTime) || 0)),
    }));
  };
  const update = () => {
    if (session) safe(() => { session.playbackState = player ? player.paused || player.ended ? 'paused' : 'playing' : 'none'; });
    position();
  };
  const seek = (time, fast = false) => {
    if (!player || !Number.isFinite(time) || !Number.isFinite(player.duration) || player.duration <= 0) return;
    const bounded = Math.min(player.duration, Math.max(0, time));
    if (fast && typeof player.fastSeek === 'function') player.fastSeek(bounded);
    else player.currentTime = bounded;
    position();
  };
  const dispatch = (action, details = {}) => {
    if (!player) return;
    try {
      if (action === 'seekto') seek(details.seekTime, details.fastSeek);
      else if (action === 'seekbackward' || action === 'seekforward') {
        const offset = Number.isFinite(details.seekOffset) && details.seekOffset > 0 ? details.seekOffset : 10;
        seek(player.currentTime + (action === 'seekforward' ? offset : -offset));
      } else {
        const name = {nexttrack: 'next', previoustrack: 'previous'}[action] || action;
        Promise.resolve(control(name)).catch(error => report(error.message || String(error)));
      }
    } catch (error) { report(error.message || String(error)); }
  };
  const detach = () => { for (const remove of removers) remove(); removers = []; player = undefined; };
  const clear = () => {
    detach();
    if (!session) return;
    for (const action of actions) safe(() => session.setActionHandler(action, null));
    safe(() => { session.metadata = null; session.playbackState = 'none'; });
    safe(() => session.setPositionState?.());
  };
  const attach = (element, metadata) => {
    detach(); player = element;
    if (!session) return;
    safe(() => { session.metadata = Metadata ? new Metadata(metadata) : null; });
    for (const action of actions) safe(() => session.setActionHandler(action, details => dispatch(action, details)));
    for (const event of ['play', 'pause', 'ended', 'durationchange', 'loadedmetadata', 'timeupdate', 'ratechange', 'seeked']) {
      element.addEventListener(event, update);
      removers.push(() => element.removeEventListener(event, update));
    }
    update();
  };
  return {attach, clear, dispose: clear};
}
