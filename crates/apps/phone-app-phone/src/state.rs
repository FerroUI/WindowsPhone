use ferroui_view::prelude::*;
pub struct PhoneSession {
    pub readable: std::cell::Cell<(bool, bool)>,
    pub dial: Signal<String>,
    pub account: Signal<Option<String>>,
    pub clock: Signal<i64>,
}
impl Default for PhoneSession {
    fn default() -> Self {
        Self {
            readable: std::cell::Cell::new((false, false)),
            dial: signal(String::new()),
            account: signal(None),
            clock: signal(0),
        }
    }
}
