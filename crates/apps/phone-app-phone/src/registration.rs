use phone_sdk::AppRegistration;

pub fn registration() -> AppRegistration {
    AppRegistration::builtin(phone_domain::AppId::Phone, crate::view)
        .with_pivot_count(|ui| if ui.pivot.get_untracked() < 2 { 2 } else { 0 })
        .with_reset(|ui| ui.state::<crate::PhoneSession>().dial.set(String::new()))
}
