use phone_sdk::AppRegistration;

pub fn registration() -> AppRegistration {
    AppRegistration::builtin(phone_domain::AppId::Music, crate::view)
        .with_pivot_count(|_| 2)
        .with_back(|vm, ui| phone_media_support::back(vm, ui, phone_domain::AppId::Music))
        .with_reset(|ui| phone_media_support::reset(ui, phone_domain::AppId::Music))
}
