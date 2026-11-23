use phone_sdk::AppRegistration;

pub fn registration() -> AppRegistration {
    AppRegistration::builtin(phone_domain::AppId::Camera, crate::view)
}
