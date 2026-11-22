use phone_domain::*;
use phone_state::{HostRequest, HostServices, PhoneViewModel};
use std::{cell::RefCell, rc::Rc};

fn model() -> Rc<PhoneViewModel> {
    PhoneViewModel::new(PhoneData::demo(), Rc::new(|_| {}))
}
fn target_id(vm: &PhoneViewModel, target: TileTarget) -> Id {
    vm.data()
        .all_tiles()
        .find(|tile| tile.target == Some(target))
        .unwrap()
        .id
}
#[test]
fn folder_resize_reorder_extract_and_roundtrip_preserve_original_records() {
    let vm = model();
    vm.set_tile_size(100, TileSize::Wide);
    let original = vm.data().tile(100).unwrap().clone();
    vm.create_folder(100, 101, "communication".into());
    let folder = vm.data().parent_folder(100).unwrap();
    assert_eq!(vm.data().tile(100), Some(&original));
    vm.create_folder(102, folder, "ignored".into());
    vm.move_folder_tile(folder, 100, Some(101));
    assert_eq!(
        vm.data()
            .tile(folder)
            .unwrap()
            .folder_tiles
            .iter()
            .map(|tile| tile.id)
            .collect::<Vec<_>>(),
        [100, 101, 102]
    );
    vm.set_tile_size(101, TileSize::Small);
    let restored = PhoneData::from_json(&vm.export_json().unwrap()).unwrap();
    assert_eq!(*vm.data(), restored);
    assert_eq!(restored.tile(100), Some(&original));
    assert_eq!(restored.tile(101).unwrap().size, TileSize::Small);
    vm.extract_folder_tile(folder, 100);
    assert_eq!(vm.data().parent_folder(100), None);
    assert_eq!(vm.data().tile(100), Some(&original));
    vm.unpin(101);
    assert!(vm.data().tile(folder).is_none());
    assert_eq!(vm.data().parent_folder(102), None);
}
#[test]
fn relative_drop_resolves_destination_including_end_of_another_folder() {
    let vm = model();
    vm.create_folder(100, 101, "one".into());
    let one = vm.data().parent_folder(100).unwrap();
    vm.create_folder(102, 103, "two".into());
    let two = vm.data().parent_folder(102).unwrap();
    vm.move_tile_relative(100, 102, true);
    assert!(vm.data().tile(one).is_none());
    assert_eq!(
        vm.data()
            .tile(two)
            .unwrap()
            .folder_tiles
            .iter()
            .map(|tile| tile.id)
            .collect::<Vec<_>>(),
        [103, 102, 100]
    );
    let last = vm.data().tiles.last().unwrap().id;
    vm.move_tile_relative(100, last, true);
    assert_eq!(vm.data().tiles.last().unwrap().id, 100);
    assert_eq!(vm.data().parent_folder(100), None);
    let before = vm.data().clone();
    vm.move_tile_relative(two, 102, false);
    assert_eq!(*vm.data(), before);
}
#[test]
fn secondary_tiles_activate_exact_items_and_clean_deleted_targets() {
    let vm = model();
    let requests = Rc::new(RefCell::new(Vec::new()));
    let capture = requests.clone();
    vm.set_services(HostServices {
        request: Rc::new(move |request| capture.borrow_mut().push(request)),
    });
    vm.pin_contact(1);
    vm.pin_note(501);
    vm.pin_note(502);
    vm.pin_bookmark(951);
    let contact = target_id(&vm, TileTarget::Contact(1));
    let note = target_id(&vm, TileTarget::Note(501));
    let second = target_id(&vm, TileTarget::Note(502));
    let bookmark = target_id(&vm, TileTarget::Bookmark(951));
    assert_ne!(note, second);
    let count = vm.data().all_tiles().count();
    vm.pin_note(501);
    assert_eq!(vm.data().all_tiles().count(), count);
    vm.open_tile(contact);
    assert_eq!(vm.route(), Route::Contact(1));
    vm.open_tile(note);
    assert_eq!(vm.route(), Route::Note(501));
    vm.open_tile(bookmark);
    assert_eq!(vm.route(), Route::App(AppId::InternetExplorer));
    assert!(
        matches!(requests.borrow().last(),Some(HostRequest::Browse{url}) if url=="https://github.com/wieslawsoltes/FerroUI")
    );
    vm.create_folder(note, contact, "personal".into());
    let folder = vm.data().parent_folder(note).unwrap();
    let mut renamed = vm.data().note(501).unwrap().clone();
    renamed.title = "Renamed note".into();
    vm.save_note(renamed);
    assert_eq!(vm.data().tile(note).unwrap().label, "Renamed note");
    vm.delete_contact(1);
    assert!(vm.data().tile(contact).is_none());
    assert!(vm.data().tile(folder).is_none());
    assert_eq!(vm.data().parent_folder(note), None);
    vm.delete_note(501);
    assert!(vm.data().tile(note).is_none());
    assert!(vm.data().tile(second).is_some());
    vm.delete_bookmark(951);
    assert!(vm.data().tile(bookmark).is_none());
    let exported = PhoneData::from_json(&vm.export_json().unwrap()).unwrap();
    assert_eq!(*vm.data(), exported);
}
#[test]
fn mixed_folder_edits_always_remain_importable_and_unique() {
    let vm = model();
    let mut random = 33_u64;
    for turn in 0..120 {
        random = random.wrapping_mul(6364136223846793005).wrapping_add(1);
        let ids = vm
            .data()
            .all_tiles()
            .map(|tile| tile.id)
            .collect::<Vec<_>>();
        let source = ids[random as usize % ids.len()];
        let target = ids[(random >> 16) as usize % ids.len()];
        match turn % 5 {
            0 => vm.create_folder(source, target, "group".into()),
            1 => vm.move_tile_relative(source, target, random & 1 == 0),
            2 => vm.resize_tile(source),
            3 => {
                let parent = vm.data().parent_folder(source);
                if let Some(parent) = parent {
                    vm.extract_folder_tile(parent, source);
                }
            }
            _ => vm.move_tile(source, Some(target)),
        }
        let doc = vm.data().clone();
        assert_eq!(PhoneData::from_json(&doc.to_json().unwrap()).unwrap(), doc);
        assert_eq!(
            doc.all_tiles()
                .map(|tile| tile.id)
                .collect::<std::collections::HashSet<_>>()
                .len(),
            doc.all_tiles().count()
        );
    }
}
