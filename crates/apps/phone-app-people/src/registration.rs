use phone_sdk::AppRegistration;

pub fn registration() -> AppRegistration {
    AppRegistration::builtin(phone_domain::AppId::People, crate::view).with_pivot_count(|_| 2)
}
