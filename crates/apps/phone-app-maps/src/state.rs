use crate::model::Place;
use ferroui_view::prelude::*;
use phone_app_support::geography::PlaceMatch;
use std::cell::Cell;
pub struct MapSession {
    pub selection: Signal<Option<Place>>,
    pub results: Signal<Vec<PlaceMatch>>,
    pub busy: Signal<bool>,
    pub generation: Cell<u64>,
    pub directions: Signal<bool>,
    pub origin: Signal<String>,
    pub destination: Signal<String>,
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
        }
    }
}
