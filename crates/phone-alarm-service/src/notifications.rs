use super::Alert;
use phone_domain::{AppId, Settings};
use phone_state::{HostPermission, HostRequest, PhoneViewModel};

fn allowed(settings: &Settings, permission: HostPermission, app: AppId, locked: bool) -> bool {
    permission == HostPermission::Granted
        && settings.notifications
        && settings.notification_preference(app).banners
        && (!locked || settings.notifications_on_lock)
}

pub(super) fn deliver(vm: &PhoneViewModel, alert: &Alert) {
    let (app, title, body) = match alert {
        Alert::Alarm(alarm) => (
            AppId::Alarms,
            "alarm".to_owned(),
            format!("{} · {}", alarm.time_text(), alarm.label),
        ),
        Alert::Calendar(reminder) => (
            AppId::Calendar,
            reminder.title.clone(),
            if reminder.location.is_empty() {
                "Calendar reminder".into()
            } else {
                reminder.location.clone()
            },
        ),
    };
    let locked = phone_sdk::current_session().is_some_and(|ui| ui.locked.get_untracked());
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
            AppId::Calendar,
            false
        ));
        assert!(allowed(
            &settings,
            HostPermission::Granted,
            AppId::Calendar,
            false
        ));
        settings.notifications_on_lock = false;
        assert!(!allowed(
            &settings,
            HostPermission::Granted,
            AppId::Calendar,
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
            AppId::Calendar,
            false
        ));
    }
}
