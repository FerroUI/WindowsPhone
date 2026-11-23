use phone_sdk::AppRegistration;

pub fn registration() -> AppRegistration {
    AppRegistration::builtin(phone_domain::AppId::Alarms, crate::view)
        .with_back(|_, ui| {
            ui.state::<phone_alarm_service::AlarmState>()
                .editor
                .dismiss()
        })
        .with_route_changed(|_, ui, _, _| {
            ui.state::<phone_alarm_service::AlarmState>().editor.close()
        })
        .with_deactivate(|ui| ui.state::<phone_alarm_service::AlarmState>().editor.close())
        .with_overlay(|ui| {
            ui.state::<phone_alarm_service::AlarmState>()
                .editor
                .draft
                .get_untracked()
                .is_some()
        })
}
