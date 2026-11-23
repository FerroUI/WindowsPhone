use phone_sdk::AppRegistration;

pub fn registration() -> AppRegistration {
    AppRegistration::builtin(phone_domain::AppId::Calculator, crate::view)
        .with_backspace_input(true)
        .with_reset(|ui| {
            let state = ui.state::<crate::CalculatorSession>();
            *state.calculator.borrow_mut() = Default::default();
            state.revision.update(|v| *v = v.wrapping_add(1));
        })
}
