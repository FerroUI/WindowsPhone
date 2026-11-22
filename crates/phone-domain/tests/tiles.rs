use phone_domain::*;

#[test]
fn legacy_folders_migrate_once_with_deterministic_unique_child_ids() {
    let mut legacy = PhoneData::demo();
    legacy.tiles[0].children = vec![AppId::Phone, AppId::Messaging, AppId::Phone];
    legacy.tiles.retain(|tile| tile.id != 101);
    legacy.next_id = 3;
    let json = legacy.to_json().unwrap();
    let first = PhoneData::from_json(&json).unwrap();
    let second = PhoneData::from_json(&json).unwrap();
    assert_eq!(first, second);
    let folder = &first.tiles[0];
    assert_eq!(folder.folder_tiles.len(), 2);
    assert_eq!(folder.children, [AppId::Phone, AppId::Messaging]);
    assert!(folder
        .folder_tiles
        .iter()
        .all(|tile| tile.size == TileSize::Small && tile.id >= 1000));
    assert_eq!(
        PhoneData::from_json(&first.to_json().unwrap()).unwrap(),
        first
    );
}
#[test]
fn nested_folders_duplicate_child_ids_and_missing_secondary_targets_are_rejected() {
    let mut data = PhoneData::demo();
    data.tiles[0]
        .folder_tiles
        .push(Tile::new(1100, AppId::Phone, TileSize::Small));
    data.tiles[0].folder_tiles[0].folder_tiles.push(Tile::new(
        1101,
        AppId::Photos,
        TileSize::Small,
    ));
    assert!(data.normalize().is_err());
    data.tiles[0].folder_tiles[0].folder_tiles.clear();
    data.tiles[0].folder_tiles[0].id = 101;
    assert!(data.normalize().is_err());
    data.tiles[0].folder_tiles[0].id = 1100;
    data.tiles[0].folder_tiles[0].target = Some(TileTarget::Contact(9876));
    assert!(data.normalize().is_err());
}
#[test]
fn quick_action_slots_repair_invalid_or_duplicate_values() {
    let mut data = PhoneData::demo();
    data.settings.quick_actions = ["camera", "camera", "missing", "brightness"].map(str::to_owned);
    data.normalize().unwrap();
    assert_eq!(
        data.settings.quick_actions,
        ["camera", "wi_fi", "bluetooth", "brightness"]
    );
}
