use phone_sdk::AppRegistration;

pub fn registration() -> AppRegistration {
    AppRegistration::builtin(phone_domain::AppId::Music, crate::view).with_pivot_count(|_| 2)
}
