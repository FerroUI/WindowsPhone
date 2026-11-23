use ferroui_view::prelude::*;
use std::cell::{Cell, RefCell};
pub struct State {
    pub query: Signal<String>,
    pub episode: Signal<Option<String>>,
    pub busy: Signal<bool>,
    pub generation: Cell<u64>,
    pub pending_seek: RefCell<Option<(String, u32)>>,
    pub last_save: Cell<i64>,
}
impl Default for State {
    fn default() -> Self {
        Self {
            query: signal(String::new()),
            episode: signal(None),
            busy: signal(false),
            generation: Cell::new(0),
            pending_seek: RefCell::new(None),
            last_save: Cell::new(0),
        }
    }
}
impl State {
    pub fn reset(&self) {
        self.query.set(String::new());
        self.episode.set(None);
        self.busy.set(false);
        self.generation.set(self.generation.get().wrapping_add(1));
        self.pending_seek.borrow_mut().take();
        self.last_save.set(0);
    }
}
