use ferroui_view::prelude::*;
pub struct PhotosSession {
    pub zoom: Signal<f64>,
}
impl Default for PhotosSession {
    fn default() -> Self {
        Self { zoom: signal(1.0) }
    }
}
