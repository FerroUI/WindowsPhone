use ferroui_view::prelude::Element;
use phone_controls::Icon;
use phone_domain::{PhoneData, TileTarget};
use phone_sdk::{
    session::Session, AppContext, AppManifest, AppRegistration, AppRegistry, TileNotification,
    TileTemplate,
};
use phone_state::PhoneViewModel;
use std::rc::Rc;

fn registration(id: &'static str) -> AppRegistration {
    AppRegistration::new(AppManifest::new(id, id, Icon::Note), |_| Element::Empty)
}
fn fixture() -> (AppContext, u64, u64) {
    let registry = Rc::new(
        AppRegistry::new(vec![registration("tiles.one"), registration("tiles.two")]).unwrap(),
    );
    let vm = PhoneViewModel::new(PhoneData::default(), Rc::new(|_| {}));
    vm.set_registered_apps(["tiles.one".into(), "tiles.two".into()]);
    vm.pin_app("tiles.one", "one", "One").unwrap();
    vm.pin_app("tiles.two", "two", "Two").unwrap();
    let ids = vm
        .data()
        .tiles
        .iter()
        .map(|tile| tile.id)
        .collect::<Vec<_>>();
    let session = Rc::new(Session::new(vm.now(), registry));
    (
        AppContext {
            vm,
            session,
            id: "tiles.one",
        },
        ids[0],
        ids[1],
    )
}
fn card(context: &AppContext) -> TileNotification {
    TileNotification::new(
        "update",
        TileTemplate::Text {
            title: "Owned update".into(),
            body: String::new(),
        },
        context.vm.now(),
    )
}

#[test]
fn tile_updater_validates_app_and_exact_tile_ownership_including_folders() {
    let (context, own, other) = fixture();
    let updater = context.tile_updater().unwrap();
    assert_eq!(updater.tiles().unwrap(), [own]);
    assert!(updater.publish(other, card(&context)).is_err());
    updater.publish(own, card(&context)).unwrap();
    context.vm.create_folder(own, other, "Both".into());
    assert_eq!(updater.tiles().unwrap(), [own]);
    assert!(updater
        .publish(
            context.vm.data().parent_folder(own).unwrap(),
            card(&context)
        )
        .is_err());
    assert!(matches!(
        context.vm.data().tile(own).unwrap().target,
        Some(TileTarget::AppLink(_))
    ));
    updater.clear(own, Some("update")).unwrap();
    assert!(context
        .vm
        .data()
        .tile(own)
        .unwrap()
        .notifications
        .is_empty());
}

#[test]
fn restore_disable_close_and_reset_refuse_stale_publishers() {
    let (context, own, _) = fixture();
    let updater = context.tile_updater().unwrap();
    context
        .session
        .registry()
        .reset_app(&context.vm, &context.session, context.id);
    assert!(updater.publish(own, card(&context)).is_err());
    let updater = context.tile_updater().unwrap();
    context
        .session
        .registry()
        .close(&context.vm, &context.session, context.id);
    assert!(updater.clear(own, None).is_err());
    let updater = context.tile_updater().unwrap();
    context
        .vm
        .import_json(&context.vm.export_json().unwrap())
        .unwrap();
    assert!(updater.publish(own, card(&context)).is_err());
    let updater = context.tile_updater().unwrap();
    context.vm.set_extension_enabled(context.id, false).unwrap();
    assert!(updater.tiles().is_err());
}

#[test]
fn shutdown_revokes_even_retained_contexts_and_updaters_do_not_keep_hosts_alive() {
    let (context, own, _) = fixture();
    let updater = context.tile_updater().unwrap();
    context
        .session
        .registry()
        .shutdown(&context.vm, &context.session);
    assert!(updater.publish(own, card(&context)).is_err());
    assert!(context.tile_updater().is_err());
    let weak = Rc::downgrade(&context.vm);
    drop(context);
    assert!(weak.upgrade().is_none());
    assert!(updater.tiles().is_err());
}
