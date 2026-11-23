use ferroui_view::prelude::*;
pub struct PhotosSession {
    pub zoom: Signal<f64>,
    pub album: Signal<Option<phone_domain::Id>>,
    pub selecting: Signal<bool>,
    pub selection: Signal<std::collections::BTreeSet<phone_domain::Id>>,
    pub choosing_album: Signal<bool>,
}
impl Default for PhotosSession {
    fn default() -> Self {
        Self {
            zoom: signal(1.0),
            album: signal(None),
            selecting: signal(false),
            selection: signal(Default::default()),
            choosing_album: signal(false),
        }
    }
}
