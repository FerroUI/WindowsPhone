use ferroui_view::prelude::{signal, Signal};

pub(crate) struct StoreSession {
    pub selected: Signal<Option<String>>,
}
impl Default for StoreSession {
    fn default() -> Self {
        Self {
            selected: signal(None),
        }
    }
}
