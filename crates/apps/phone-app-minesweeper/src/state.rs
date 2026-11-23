use ferroui_view::prelude::*;

pub struct ViewState {
    pub flag_mode: Signal<bool>,
    pub clock: Signal<i64>,
}

impl Default for ViewState {
    fn default() -> Self {
        Self {
            flag_mode: signal(false),
            clock: signal(0),
        }
    }
}
