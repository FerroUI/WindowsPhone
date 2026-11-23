use ferroui_view::prelude::*;
pub struct CalculatorSession {
    pub calculator: std::cell::RefCell<phone_domain::Calculator>,
    pub revision: Signal<u64>,
}
impl Default for CalculatorSession {
    fn default() -> Self {
        Self {
            calculator: std::cell::RefCell::new(phone_domain::Calculator::default()),
            revision: signal(0),
        }
    }
}
