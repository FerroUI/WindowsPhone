use ferroui_view::prelude::*;
pub struct PhoneSession {
    pub dial: Signal<String>,
}
impl Default for PhoneSession {
    fn default() -> Self {
        Self {
            dial: signal(String::new()),
        }
    }
}
