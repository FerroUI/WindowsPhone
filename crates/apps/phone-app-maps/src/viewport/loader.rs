use super::{model::TileKey, Scope, State};
use ferroui_base::threading::Dispatcher;
use phone_sdk::{session::Session, AppContext};
use phone_state::PhoneViewModel;
use std::{
    cell::Cell,
    collections::{BTreeMap, BTreeSet},
    rc::{Rc, Weak},
};

const PARALLEL: usize = 2;
ferroui_base::ferro_thread_local! {static NEXT_SOURCE:Cell<Option<u64>>=const{Cell::new(Some(1))};}

pub(super) struct Flight {
    source: String,
    abort: Box<dyn Fn()>,
}
pub(super) struct Tiles<Image = phone_sdk::images::TransientImage> {
    pub visible: BTreeSet<TileKey>,
    pub ready: BTreeMap<TileKey, Image>,
    pub failed: BTreeMap<TileKey, String>,
    pending: BTreeMap<TileKey, Flight>,
}
impl<Image> Default for Tiles<Image> {
    fn default() -> Self {
        Self {
            visible: Default::default(),
            ready: Default::default(),
            failed: Default::default(),
            pending: Default::default(),
        }
    }
}
impl<Image> Tiles<Image> {
    fn reconcile(&mut self, visible: BTreeSet<TileKey>) -> Vec<Flight> {
        self.visible = visible;
        self.failed.retain(|key, _| self.visible.contains(key));
        // Retain decoded image ownership only for the visible working set.
        // Returning to a tile uses the host's persistent visited-tile cache.
        self.ready.retain(|key, _| self.visible.contains(key));
        let stale = self
            .pending
            .keys()
            .filter(|key| !self.visible.contains(key))
            .copied()
            .collect::<Vec<_>>();
        let cancelled = stale
            .into_iter()
            .filter_map(|key| self.pending.remove(&key))
            .collect();
        cancelled
    }
    fn next(&self) -> Option<TileKey> {
        if self.pending.len() >= PARALLEL {
            return None;
        }
        self.visible
            .iter()
            .find(|key| {
                !self.ready.contains_key(key)
                    && !self.failed.contains_key(key)
                    && !self.pending.contains_key(key)
            })
            .copied()
    }
    fn accepts(&self, key: TileKey, source: &str) -> bool {
        self.visible.contains(&key)
            && self
                .pending
                .get(&key)
                .is_some_and(|flight| flight.source == source)
    }
    pub fn loading(&self) -> bool {
        self.visible
            .iter()
            .any(|key| !self.ready.contains_key(key) && !self.failed.contains_key(key))
    }
    fn take_pending(&mut self) -> Vec<Flight> {
        std::mem::take(&mut self.pending).into_values().collect()
    }
}
impl<Image> Drop for Tiles<Image> {
    fn drop(&mut self) {
        for flight in self.take_pending() {
            (flight.abort)();
        }
    }
}
#[derive(Clone)]
struct Context {
    vm: Weak<PhoneViewModel>,
    ui: Weak<Session>,
    epoch: u64,
}
impl Context {
    fn new(cx: &AppContext) -> Self {
        Self {
            vm: Rc::downgrade(&cx.vm),
            ui: Rc::downgrade(&cx.session),
            epoch: cx.vm.document_epoch(),
        }
    }
    fn upgrade(&self) -> Option<AppContext> {
        let vm = self.vm.upgrade()?;
        let session = self.ui.upgrade()?;
        if vm.document_epoch() != self.epoch
            || vm.route() != phone_domain::Route::App(phone_domain::AppId::Maps)
        {
            return None;
        }
        Some(AppContext {
            vm,
            session,
            id: "phone.maps",
        })
    }
}
pub(super) fn cancel(state: &State) {
    let pending = state.tiles.borrow_mut().take_pending();
    for flight in pending {
        (flight.abort)();
    }
}
pub(super) fn synchronize(state: &Rc<State>, cx: &AppContext, scope: &Rc<Scope>) {
    let visible = state
        .camera
        .get_untracked()
        .visible(state.size.get_untracked())
        .into_iter()
        .map(|tile| tile.key)
        .collect();
    let changed = state.tiles.borrow().visible != visible;
    let cancelled = state.tiles.borrow_mut().reconcile(visible);
    for flight in cancelled {
        (flight.abort)();
    }
    if changed {
        state.changed();
    }
    pump(state, &Context::new(cx), scope);
}
fn pump(state: &Rc<State>, context: &Context, scope: &Rc<Scope>) {
    if !state.current(scope)
        || state.paused.get_untracked()
        || state.dragging.get_untracked()
        || state.deferred.get_untracked()
    {
        return;
    }
    let Some(cx) = context.upgrade() else {
        return;
    };
    loop {
        let Some(key) = state.tiles.borrow().next() else {
            break;
        };
        let source = NEXT_SOURCE.with(|next| {
            let id = next.get()?;
            next.set(id.checked_add(1));
            Some(format!("map-tile:{id}"))
        });
        let Some(source) = source else {
            state.tiles.borrow_mut().failed.insert(
                key,
                "The map image identity space is exhausted. Restart Maps.".into(),
            );
            state.changed();
            break;
        };
        let task = phone_sdk::tasks::map_tile(&cx, key.zoom, key.x, key.y);
        let weak_state = Rc::downgrade(state);
        let weak_scope = Rc::downgrade(scope);
        let completion = context.clone();
        let result_source = source.clone();
        let task = Dispatcher::current_dispatcher().spawn_local(async move {
            let result = task.await.and_then(|bytes| {
                phone_sdk::host::maps::validate_png(&bytes)?;
                Ok(bytes)
            });
            let (Some(state), Some(scope), Some(_cx)) = (
                weak_state.upgrade(),
                weak_scope.upgrade(),
                completion.upgrade(),
            ) else {
                return;
            };
            if !state.current(&scope) || !state.tiles.borrow().accepts(key, &result_source) {
                return;
            }
            state.tiles.borrow_mut().pending.remove(&key);
            let result = result.and_then(|bytes| phone_sdk::images::TransientImage::decode(&bytes));
            match result {
                Ok(image) => {
                    state.tiles.borrow_mut().ready.insert(key, image);
                }
                Err(error) => {
                    state
                        .tiles
                        .borrow_mut()
                        .failed
                        .insert(key, error.chars().take(240).collect());
                }
            }
            state.changed();
            pump(&state, &completion, &scope);
        });
        let abort = task.abort_handle();
        drop(task);
        state.tiles.borrow_mut().pending.insert(
            key,
            Flight {
                source,
                abort: Box::new(move || abort.abort()),
            },
        );
    }
}
pub(super) fn retry(state: &Rc<State>, cx: &AppContext) {
    state.tiles.borrow_mut().failed.clear();
    state.changed();
    if let Some(scope) = state.scope.borrow().upgrade() {
        synchronize(state, cx, &scope);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn completion_context_is_weak_and_document_and_mounted_scope_bound() {
        let vm = PhoneViewModel::new(phone_domain::PhoneData::default(), Rc::new(|_| {}));
        let registry = Rc::new(phone_sdk::AppRegistry::new(vec![crate::registration()]).unwrap());
        registry.bind(&vm);
        let session = Rc::new(Session::new(vm.now(), registry));
        vm.navigate(phone_domain::Route::App(phone_domain::AppId::Maps));
        let cx = crate::commands::context(vm.clone(), session.clone());
        let context = Context::new(&cx);
        assert!(context.upgrade().is_some());
        let state = State::default();
        let first = Rc::new(Scope {
            alive: Cell::new(true),
        });
        let replacement = Rc::new(Scope {
            alive: Cell::new(true),
        });
        *state.scope.borrow_mut() = Rc::downgrade(&first);
        assert!(state.current(&first));
        *state.scope.borrow_mut() = Rc::downgrade(&replacement);
        assert!(
            !state.current(&first),
            "Returning to Maps cannot revive an old mounted viewport"
        );
        replacement.alive.set(false);
        assert!(!state.current(&replacement));
        vm.home();
        assert!(context.upgrade().is_none());
        vm.import_json(&vm.export_json().unwrap()).unwrap();
        vm.navigate(phone_domain::Route::App(phone_domain::AppId::Maps));
        assert!(
            context.upgrade().is_none(),
            "A restored document rejects the old completion even on the same route"
        );
        let weak_vm = Rc::downgrade(&vm);
        let weak_session = Rc::downgrade(&session);
        drop(cx);
        drop(vm);
        drop(session);
        assert!(weak_vm.upgrade().is_none());
        assert!(weak_session.upgrade().is_none());
    }
    fn key(x: u32) -> TileKey {
        TileKey { zoom: 10, x, y: 5 }
    }
    fn flight(source: &str, cancelled: Rc<Cell<u32>>) -> Flight {
        Flight {
            source: source.into(),
            abort: Box::new(move || cancelled.set(cancelled.get() + 1)),
        }
    }
    #[test]
    fn viewport_admission_limits_requests_cancels_outside_tiles_and_refuses_old_results() {
        let cancelled = Rc::new(Cell::new(0));
        let mut tiles = Tiles::<()>::default();
        tiles.visible = BTreeSet::from([key(1), key(2), key(3)]);
        tiles
            .pending
            .insert(key(1), flight("first", cancelled.clone()));
        tiles
            .pending
            .insert(key(2), flight("second", cancelled.clone()));
        assert_eq!(tiles.next(), None);
        for old in tiles.reconcile(BTreeSet::from([key(2), key(3)])) {
            (old.abort)();
        }
        assert_eq!(cancelled.get(), 1);
        assert!(!tiles.accepts(key(1), "first"));
        assert!(tiles.accepts(key(2), "second"));
        assert_eq!(tiles.next(), Some(key(3)));
        tiles
            .pending
            .insert(key(3), flight("replacement", cancelled.clone()));
        assert!(!tiles.accepts(key(3), "previous"));
        drop(tiles);
        assert_eq!(
            cancelled.get(),
            3,
            "All retained work is aborted when its owner drops"
        );
    }
    #[test]
    fn retained_images_are_bounded_without_prefetch_and_failures_need_explicit_retry() {
        let mut tiles = Tiles::<()>::default();
        for x in 0..40 {
            tiles.ready.insert(key(x), ());
        }
        tiles.reconcile(BTreeSet::from([key(39), key(50)]));
        assert_eq!(tiles.ready.len(), 1);
        assert!(tiles.ready.contains_key(&key(39)));
        assert_eq!(tiles.next(), Some(key(50)));
        tiles.failed.insert(key(50), "offline".into());
        assert!(!tiles.loading());
        assert_eq!(tiles.next(), None);
        tiles.failed.clear();
        assert_eq!(tiles.next(), Some(key(50)));
    }
}
