use ferroui_view::prelude::*;
pub struct WeatherState {
    pub current: Signal<Option<(String, f64, String)>>,
}
impl Default for WeatherState {
    fn default() -> Self {
        Self {
            current: signal(None),
        }
    }
}
