use phone_domain::{AppId, DateOrder, Settings};

#[test]
fn old_settings_preserve_media_level_and_receive_system_defaults() {
    let old: Settings =
        serde_json::from_str(r##"{"volume":21,"sounds":false,"accent":"#0050EF"}"##).unwrap();
    assert_eq!(old.volume, 21);
    assert_eq!(old.ringer_volume, 8);
    assert!(!old.media_muted);
    assert_eq!(old.media_gain(), 0.7);
    assert!(old.live_tiles_enabled());
    assert_eq!(
        old.lock_quick_apps,
        ["Phone", "Messaging", "Outlook", "", ""]
    );
    assert!(old.notifications_allowed(AppId::Messaging));
}

#[test]
fn imported_system_preferences_are_bounded_and_lock_slots_are_unique() {
    let mut settings: Settings = serde_json::from_str(
        r#"{
        "ringer_volume":255,"phone_name":"   ","lock_timeout":999999,
        "lock_detail_app":"unknown","lock_quick_apps":["Phone","Phone","Outlook","unknown",""] ,
        "app_notifications":{"Messaging":{"action_center":false},"unknown":{}},
        "region":"unsupported"
    }"#,
    )
    .unwrap();
    settings.normalize();
    assert_eq!(settings.ringer_volume, 10);
    assert_eq!(settings.lock_timeout, 60);
    assert_eq!(settings.phone_name, "Lumia");
    assert_eq!(settings.lock_detail_app, "Calendar");
    assert_eq!(settings.lock_quick_apps, ["Phone", "", "Outlook", "", ""]);
    assert_eq!(settings.app_notifications.len(), 1);
    assert!(!settings.notifications_allowed(AppId::Messaging));
    assert!(settings.notification_preference(AppId::Messaging).banners);
    assert_eq!(settings.region, "United Kingdom");
}

#[test]
fn preferences_round_trip_without_changing_independent_channels() {
    let mut settings = Settings::default();
    settings.ringer_volume = 2;
    settings.volume = 27;
    settings.media_muted = true;
    settings.reduced_motion = true;
    settings.high_contrast = true;
    settings.date_order = DateOrder::YearMonthDay;
    let restored: Settings =
        serde_json::from_str(&serde_json::to_string(&settings).unwrap()).unwrap();
    assert_eq!(restored, settings);
    assert_eq!(restored.media_gain(), 0.0);
    assert!(!restored.motion_enabled());
    assert_eq!(restored.foreground(), "#FFFFFF");
    assert_eq!(restored.background(), "#000000");
    assert_eq!(restored.date_order.format(2026, 11, 22), "2026-11-22");
}
