use phone_domain::*;
use phone_state::PhoneViewModel;
use std::rc::Rc;

fn model() -> Rc<PhoneViewModel> {
    PhoneViewModel::new(PhoneData::demo(), Rc::new(|_| {}))
}
fn card(tag: &str, now: i64) -> TileNotification {
    TileNotification::new(
        tag,
        TileTemplate::Text {
            title: tag.into(),
            body: "body".into(),
        },
        now,
    )
}

#[test]
fn publishing_replaces_tags_evicts_oldest_and_clears_exactly_one_queue() {
    let vm = model();
    for index in 0..7 {
        vm.publish_tile_notification(100, card(&format!("card{index}"), vm.now()))
            .unwrap();
    }
    assert_eq!(
        vm.data()
            .tile(100)
            .unwrap()
            .notifications
            .iter()
            .map(|n| n.tag.as_str())
            .collect::<Vec<_>>(),
        ["card2", "card3", "card4", "card5", "card6"]
    );
    vm.publish_tile_notification(100, card("card3", vm.now()))
        .unwrap();
    assert_eq!(
        vm.data()
            .tile(100)
            .unwrap()
            .notifications
            .last()
            .unwrap()
            .tag,
        "card3"
    );
    assert_eq!(vm.data().tile(100).unwrap().notifications.len(), 5);
    vm.publish_tile_notification(101, card("other", vm.now()))
        .unwrap();
    vm.clear_tile_notifications(100, Some("card4")).unwrap();
    assert_eq!(vm.data().tile(100).unwrap().notifications.len(), 4);
    vm.clear_tile_notifications(100, None).unwrap();
    assert!(vm.data().tile(100).unwrap().notifications.is_empty());
    assert_eq!(vm.data().tile(101).unwrap().notifications.len(), 1);
}

#[test]
fn folders_and_roundtrip_preserve_child_customization_and_notifications() {
    let vm = model();
    let appearance = TileAppearance {
        label: Some("My phone".into()),
        background: TileBackground::Color("#004488".into()),
        show_label: false,
        ..Default::default()
    };
    vm.set_tile_appearance(100, appearance).unwrap();
    vm.publish_tile_notification(100, card("call", vm.now()))
        .unwrap();
    let saved = vm.data().tile(100).unwrap().clone();
    vm.create_folder(100, 101, "Favorites".into());
    let folder = vm.data().parent_folder(100).unwrap();
    vm.move_into_folder(102, folder);
    vm.set_tile_size(102, TileSize::Small);
    assert_eq!(vm.data().tile(100).unwrap(), &saved);
    vm.dissolve_folder(folder);
    assert_eq!(vm.data().tile(100).unwrap(), &saved);
    assert!(vm.data().tile(folder).is_none());
    assert_eq!(vm.data().parent_folder(102), None);
    assert_eq!(
        *vm.data(),
        PhoneData::from_json(&vm.export_json().unwrap()).unwrap()
    );
}

#[test]
fn invalid_customization_or_card_never_partially_mutates_the_document() {
    let vm = model();
    let before = vm.data().clone();
    assert!(vm
        .set_tile_appearance(
            100,
            TileAppearance {
                background: TileBackground::Color("red".into()),
                ..Default::default()
            }
        )
        .is_err());
    assert!(vm
        .publish_tile_notification(100, card("bad/tag", vm.now()))
        .is_err());
    assert_eq!(*vm.data(), before);
    vm.create_folder(100, 101, "Folder".into());
    let folder = vm.data().parent_folder(100).unwrap();
    assert!(vm
        .publish_tile_notification(folder, card("wrong", vm.now()))
        .is_err());
}

#[test]
fn deleting_a_photo_repairs_tile_backgrounds_and_cycle_references() {
    let vm = model();
    let photos = vm
        .data()
        .photos
        .iter()
        .take(2)
        .map(|photo| photo.id)
        .collect::<Vec<_>>();
    assert_eq!(photos.len(), 2);
    vm.set_tile_appearance(
        100,
        TileAppearance {
            background: TileBackground::Photo(photos[0]),
            ..Default::default()
        },
    )
    .unwrap();
    vm.publish_tile_notification(
        100,
        TileNotification::new(
            "photos",
            TileTemplate::Cycle {
                photos: photos.clone(),
                title: "Trip".into(),
            },
            vm.now(),
        ),
    )
    .unwrap();
    vm.dispatch(Action::DeletePhoto(photos[0]));
    let data = vm.data();
    let tile = data.tile(100).unwrap();
    assert_eq!(tile.appearance.background, TileBackground::App);
    assert_eq!(tile.notifications[0].template.photo_ids(), [photos[1]]);
    drop(data);
    vm.dispatch(Action::DeletePhoto(photos[1]));
    assert!(vm.data().tile(100).unwrap().notifications.is_empty());
}
