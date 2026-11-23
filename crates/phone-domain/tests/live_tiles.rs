use phone_domain::*;

fn card(tag: &str, starts_at: i64, expires_at: Option<i64>) -> TileNotification {
    let mut value = TileNotification::new(
        tag,
        TileTemplate::Text {
            title: tag.into(),
            body: "content".into(),
        },
        starts_at,
    );
    value.expires_at = expires_at;
    value
}

#[test]
fn old_phone_backups_get_default_tile_preferences_and_no_notifications() {
    let data = PhoneData::demo();
    let mut json = serde_json::to_value(&data).unwrap();
    for tile in json["tiles"].as_array_mut().unwrap() {
        tile.as_object_mut().unwrap().remove("appearance");
        tile.as_object_mut().unwrap().remove("notifications");
    }
    let restored = PhoneData::from_json(&json.to_string()).unwrap();
    assert!(restored
        .all_tiles()
        .all(|tile| tile.appearance == TileAppearance::default() && tile.notifications.is_empty()));
    assert_eq!(
        PhoneData::from_json(&restored.to_json().unwrap()).unwrap(),
        restored
    );
}

#[test]
fn schedule_expiry_and_rotation_use_current_time_without_changing_data() {
    let mut tile = Tile::new(1, AppId::Phone, TileSize::Wide);
    tile.notifications = vec![card("first", 100, Some(140)), card("later", 120, Some(130))];
    let before = tile.clone();
    assert!(tile.active_notification(99, true).is_none());
    assert_eq!(tile.active_notification(110, true).unwrap().tag, "first");
    assert_eq!(tile.active_notification(120, true).unwrap().tag, "later");
    assert_eq!(tile.active_notification(128, true).unwrap().tag, "first");
    assert_eq!(tile.active_notification(128, false).unwrap().tag, "later");
    assert_eq!(tile.active_notification(130, false).unwrap().tag, "first");
    assert!(tile.active_notification(140, false).is_none());
    assert_eq!(
        tile, before,
        "Clock selection must not serialize or modify queue state"
    );
    tile.appearance.queue = false;
    assert_eq!(tile.active_notification(128, true).unwrap().tag, "later");
    tile.appearance.live = false;
    assert!(tile.active_notification(128, true).is_none());
}

#[test]
fn queued_backup_admission_is_bounded_and_rejects_missing_photo_ids() {
    let mut data = PhoneData::demo();
    data.tiles[0].notifications = (0..6)
        .map(|i| card(&format!("card{i}"), 100, None))
        .collect();
    assert!(data.clone().normalize().is_err());
    data.tiles[0].notifications.truncate(2);
    data.tiles[0].notifications[1].tag = "card0".into();
    assert!(data.clone().normalize().is_err());
    data.tiles[0].notifications.truncate(1);
    data.tiles[0].notifications[0].template = TileTemplate::Photo {
        photo: Id::MAX,
        title: "photo".into(),
        body: String::new(),
    };
    assert!(data.clone().normalize().is_err());
    data.tiles[0].notifications[0].template = TileTemplate::Text {
        title: "ok".into(),
        body: String::new(),
    };
    data.normalize().unwrap();
}

#[test]
fn publication_rejects_expired_far_future_and_invalid_template_inputs() {
    assert!(card("good", 100, Some(101))
        .validate_publication(101)
        .is_err());
    assert!(card("good", 31 * 86_400, None)
        .validate_publication(0)
        .is_err());
    assert!(card("../bad", 100, None).validate().is_err());
    assert!(card("bad", 100, Some(100)).validate().is_err());
    assert!(TileTemplate::Cycle {
        photos: vec![1, 1],
        title: String::new()
    }
    .validate()
    .is_err());
    assert!(TileTemplate::List {
        title: String::new(),
        lines: vec!["x".into(); 4]
    }
    .validate()
    .is_err());
    assert!(TileTemplate::Progress {
        title: String::new(),
        body: String::new(),
        percent: 101
    }
    .validate()
    .is_err());
    assert!(TileTemplate::Text {
        title: "x".repeat(81),
        body: String::new()
    }
    .validate()
    .is_err());
}

#[test]
fn preferences_and_badges_preserve_accessibility_without_unbounded_visual_counts() {
    let mut appearance = TileAppearance::default();
    for color in ["red", "#00000000", "#zzzzzz", "#fff"] {
        appearance.background = TileBackground::Color(color.into());
        assert!(appearance.validate().is_err());
    }
    appearance.background = TileBackground::Color("#0088ff".into());
    appearance.validate().unwrap();
    appearance.label = Some("one\ntwo".into());
    assert!(appearance.validate().is_err());
    assert_eq!(TileBadge::count_text(1000), "99+");
    assert_eq!(TileBadge::Count(1000).accessible_text(), "1000 new items");
}
