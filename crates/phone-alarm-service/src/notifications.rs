use super::Alert;
use phone_domain::{AppId, Settings};
use phone_state::{HostPermission, HostRequest, PhoneViewModel};

fn allowed(
    settings: &Settings,
    permission: HostPermission,
    app: Option<AppId>,
    locked: bool,
) -> bool {
    permission == HostPermission::Granted
        && settings.notifications
        && app
            .map(|app| settings.notification_preference(app))
            .unwrap_or_default()
            .banners
        && (!locked || settings.notifications_on_lock)
}

pub(super) fn deliver(vm: &PhoneViewModel, alert: &Alert) {
    let (app, title, body) = match alert {
        Alert::Alarm(alarm) => (
            Some(AppId::Alarms),
            "alarm".to_owned(),
            format!("{} · {}", alarm.time_text(), alarm.label),
        ),
        Alert::Calendar(reminder) => (
            Some(AppId::Calendar),
            reminder.title.clone(),
            if reminder.location.is_empty() {
                "Calendar reminder".into()
            } else {
                reminder.location.clone()
            },
        ),
        Alert::App(item) => (
            item.builtin,
            item.reminder.title.clone(),
            if item.reminder.detail.is_empty() {
                format!("{} reminder", item.app_title)
            } else {
                item.reminder.detail.clone()
            },
        ),
    };
    let locked = vm.route() == phone_domain::Route::Lock
        || phone_sdk::current_session().is_some_and(|ui| ui.locked.get_untracked());
    let permitted = allowed(
        &vm.data().settings,
        vm.environment().notification_permission,
        app,
        locked,
    );
    if permitted {
        vm.request(HostRequest::Notify { title, body });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn host_alerts_obey_permission_app_banners_and_lock_privacy() {
        let mut settings = Settings::default();
        assert!(!allowed(
            &settings,
            HostPermission::Prompt,
            Some(AppId::Calendar),
            false
        ));
        assert!(allowed(
            &settings,
            HostPermission::Granted,
            Some(AppId::Calendar),
            false
        ));
        settings.notifications_on_lock = false;
        assert!(!allowed(
            &settings,
            HostPermission::Granted,
            Some(AppId::Calendar),
            true
        ));
        settings
            .app_notifications
            .entry(AppId::Calendar.label().into())
            .or_default()
            .banners = false;
        assert!(!allowed(
            &settings,
            HostPermission::Granted,
            Some(AppId::Calendar),
            false
        ));
    }
}
