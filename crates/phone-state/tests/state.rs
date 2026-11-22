use ferroui_dynamic_data::IObservableCache;
use ferroui_reactive::ObservableExt;
use ferroui_reactiveui::IReactiveNotifyPropertyChanged;
use phone_domain::*;
use phone_state::{HostRequest, HostServices, PhoneViewModel};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

fn model() -> (Rc<PhoneViewModel>, Rc<Cell<usize>>) {
    let saves = Rc::new(Cell::new(0));
    let count = saves.clone();
    (
        PhoneViewModel::new(
            PhoneData::demo(),
            Rc::new(move |_| count.set(count.get() + 1)),
        ),
        saves,
    )
}
#[test]
fn navigation_clock_and_unchanged_settings_never_write_storage() {
    let (vm, saves) = model();
    vm.navigate(Route::Apps);
    vm.navigate(Route::App(AppId::Calculator));
    vm.back();
    vm.set_clock(vm.now() + 120);
    vm.set_viewport(360.0, 640.0);
    let unchanged = vm.data().settings.clone();
    vm.set_settings(unchanged);
    assert_eq!(saves.get(), 0);
    assert_eq!(vm.route(), Route::Apps);
    let mut settings = vm.data().settings.clone();
    settings.more_tiles = true;
    vm.set_settings(settings);
    assert_eq!(saves.get(), 1);
}
#[test]
fn back_dismisses_overlays_and_preserves_navigation() {
    let (vm, _) = model();
    vm.navigate(Route::Apps);
    vm.navigate(Route::App(AppId::OneNote));
    vm.navigate(Route::Note(501));
    vm.navigate(Route::Notifications);
    vm.back();
    assert_eq!(vm.route(), Route::Note(501));
    vm.back();
    assert_eq!(vm.route(), Route::App(AppId::OneNote));
    vm.back();
    assert_eq!(vm.route(), Route::Apps);
    assert!(vm.recent_apps().contains(&AppId::OneNote));
}
#[test]
fn tile_folder_operations_preserve_unique_apps_and_cache_identity() {
    let (vm, saves) = model();
    let untouched = vm.tiles.lookup(&104).unwrap();
    vm.create_folder(100, 101, "communication".into());
    assert!(vm.data().tile(100).is_none());
    assert_eq!(vm.data().tile(101).unwrap().children.len(), 2);
    assert!(Rc::ptr_eq(&untouched, &vm.tiles.lookup(&104).unwrap()));
    vm.remove_from_folder(101, AppId::Phone);
    assert!(!vm.data().tile(101).unwrap().is_folder());
    assert_eq!(
        vm.data()
            .tiles
            .iter()
            .filter(|t| t.app == AppId::Phone)
            .count(),
        1
    );
    assert_eq!(saves.get(), 2);
}
#[test]
fn notification_reentrancy_queues_mutations_without_borrow_conflicts() {
    let (vm, _) = model();
    let fired = Rc::new(Cell::new(false));
    let once = fired.clone();
    let weak = Rc::downgrade(&vm);
    let subscription = vm.changed().subscribe_next(move |_| {
        if !once.replace(true) {
            if let Some(vm) = weak.upgrade() {
                vm.toggle_favorite_contact(2);
            }
        }
    });
    vm.toggle_favorite_contact(1);
    assert!(!vm.contacts.lookup(&1).unwrap().favorite);
    assert!(!vm.contacts.lookup(&2).unwrap().favorite);
    subscription.dispose();
}
#[test]
fn invalid_import_leaves_existing_document_untouched() {
    let (vm, saves) = model();
    let before = vm.data().clone();
    assert!(vm.import_json("{\"version\":900}").is_err());
    assert_eq!(*vm.data(), before);
    assert_eq!(saves.get(), 0);
}
#[test]
fn outgoing_messages_are_drafts_and_open_the_host_composer() {
    let (vm, _) = model();
    let requests = Rc::new(RefCell::new(Vec::new()));
    let sink = requests.clone();
    vm.set_services(HostServices {
        request: Rc::new(move |request| sink.borrow_mut().push(request)),
    });
    vm.send_message(201, "Hello from the app".into());
    let data = vm.data();
    let last = data.conversation(201).unwrap().messages.last().unwrap();
    assert!(last.draft);
    assert!(!last.incoming);
    assert!(matches!(
        requests.borrow().last(),
        Some(HostRequest::Sms { .. })
    ));
}
#[test]
fn invalid_calendar_event_does_not_mutate_data() {
    let (vm, saves) = model();
    vm.save_event(CalendarEvent {
        id: 0,
        title: "Leap day".into(),
        date: "2026-02-29".into(),
        time: "08:00".into(),
        duration_minutes: 30,
        ..Default::default()
    });
    assert_eq!(vm.data().events.len(), 2);
    assert_eq!(saves.get(), 0);
    assert!(!vm.status().is_empty());
}
