use ferroui_view::prelude::*;
pub struct WeatherState {
    pub current: Signal<Option<(String, f64, String)>>,
    pub query: Signal<String>,
    pub results: Signal<Vec<crate::model::Place>>,
    pub busy: Signal<bool>,
    pub generation: std::cell::Cell<u64>,
}
impl Default for WeatherState {
    fn default() -> Self {
        Self {
            current: signal(None),
            query: signal(String::new()),
            results: signal(Vec::new()),
            busy: signal(false),
            generation: std::cell::Cell::new(0),
        }
    }
}
