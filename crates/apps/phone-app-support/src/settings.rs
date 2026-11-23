pub fn update(vm: &phone_state::PhoneViewModel, change: impl FnOnce(&mut phone_domain::Settings)) {
    let mut settings = vm.data().settings.clone();
    change(&mut settings);
    vm.set_settings(settings);
}
pub fn on_off(value: bool) -> &'static str {
    if value {
        "on"
    } else {
        "off"
    }
}
