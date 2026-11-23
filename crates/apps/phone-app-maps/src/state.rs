use crate::model::Place;
use ferroui_view::prelude::*;
use phone_app_support::geography::PlaceMatch;
use std::cell::{Cell, RefCell};
pub struct MapSession {
    pub selection: Signal<Option<Place>>,
    pub results: Signal<Vec<PlaceMatch>>,
    pub busy: Signal<bool>,
    pub generation: Cell<u64>,
    pub directions: Signal<bool>,
    pub origin: Signal<String>,
    pub destination: Signal<String>,
    pub search_abort: RefCell<Option<Box<dyn Fn()>>>,
    pub locate: RefCell<Option<Locate>>,
}
#[derive(Clone, Copy)]
pub struct Locate {
    pub revision: u64,
    pub started: i64,
    pub epoch: u64,
}
impl Default for MapSession {
    fn default() -> Self {
        Self {
            selection: signal(None),
            results: signal(Vec::new()),
            busy: signal(false),
            generation: Cell::new(0),
            directions: signal(false),
            origin: signal(String::new()),
            destination: signal(String::new()),
            search_abort: RefCell::new(None),
            locate: RefCell::new(None),
        }
    }
}
impl Drop for MapSession {
    fn drop(&mut self) {
        if let Some(abort) = self.search_abort.get_mut().take() {
            abort();
        }
    }
}
