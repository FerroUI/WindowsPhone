use ferroui_dynamic_data::IObservableCache;
use phone_domain::*;
use phone_state::{HostEnvironment, PhoneViewModel};
use std::{cell::Cell, rc::Rc};

fn fixture() -> (Rc<PhoneViewModel>, Rc<Cell<usize>>) {
    let mut data = PhoneData::default();
    data.mail = vec![
        Mail {
            id: 11,
            from: "A".into(),
            subject: "first".into(),
            folder: "inbox".into(),
            ..Default::default()
        },
        Mail {
            id: 12,
            from: "B".into(),
            subject: "second".into(),
            folder: "inbox".into(),
            ..Default::default()
        },
    ];
    data.conversations = vec![
        Conversation {
            id: 21,
            recipient: "+12025550111".into(),
            unread: true,
            messages: vec![
                Message {
                    id: 31,
                    text: "one".into(),
                    incoming: true,
                    ..Default::default()
                },
                Message {
                    id: 32,
                    text: "two".into(),
                    incoming: true,
                    ..Default::default()
                },
            ],
            ..Default::default()
        },
        Conversation {
            id: 22,
            recipient: "+12025550112".into(),
            unread: true,
            ..Default::default()
        },
    ];
    data.notifications = vec![
        notification(41, AppId::Outlook, Route::Mail(11)),
        notification(42, AppId::Outlook, Route::Mail(12)),
        notification(43, AppId::Messaging, Route::Conversation(21)),
        notification(44, AppId::Messaging, Route::Conversation(22)),
    ];
    let writes = Rc::new(Cell::new(0));
    let observed = writes.clone();
    (
        PhoneViewModel::new(data, Rc::new(move |_| observed.set(observed.get() + 1))),
        writes,
    )
}

fn notification(id: Id, app: AppId, route: Route) -> Notification {
    Notification {
        id,
        app,
        route,
        title: "local notice".into(),
        body: "body".into(),
        timestamp: 1_700_000_000,
    }
}

#[test]
fn missing_item_rejects_entire_bulk_action_before_any_change_or_write() {
    let (vm, writes) = fixture();
    let before = vm.data().clone();
    vm.dispatch(Action::MailBatch {
        ids: vec![11, 999],
        operation: MailOperation::Delete,
    });
    assert_eq!(*vm.data(), before);
    assert_eq!(writes.get(), 0);
    assert!(vm.status().contains("no longer exists"));
    vm.dispatch(Action::ConversationBatch {
        ids: vec![21, 999],
        delete: true,
    });
    assert_eq!(*vm.data(), before);
    assert_eq!(writes.get(), 0);
}

#[test]
fn batch_read_deduplicates_ids_removes_only_related_notices_and_writes_once() {
    let (vm, writes) = fixture();
    let untouched = vm.notifications.lookup(&43).unwrap();
    vm.dispatch(Action::MailBatch {
        ids: vec![11, 12, 11],
        operation: MailOperation::Read,
    });
    assert!(vm.data().mail.iter().all(|mail| mail.read));
    assert_eq!(writes.get(), 1);
    assert!(vm.notifications.lookup(&41).is_none());
    assert!(vm.notifications.lookup(&42).is_none());
    assert!(Rc::ptr_eq(
        &untouched,
        &vm.notifications.lookup(&43).unwrap()
    ));
    vm.dispatch(Action::MailBatch {
        ids: vec![11, 12],
        operation: MailOperation::Read,
    });
    assert_eq!(
        writes.get(),
        1,
        "an unchanged selection must not persist again"
    );
}

#[test]
fn selected_mail_can_be_restored_and_second_delete_is_permanent() {
    let (vm, writes) = fixture();
    vm.dispatch(Action::MailBatch {
        ids: vec![11, 12],
        operation: MailOperation::Delete,
    });
    assert_eq!(writes.get(), 1);
    assert!(vm.data().mail.iter().all(|mail| mail.folder == "deleted"));
    assert!(
        vm.data()
            .notifications
            .iter()
            .all(|notice| notice.app != AppId::Outlook)
    );
    vm.dispatch(Action::MailBatch {
        ids: vec![11],
        operation: MailOperation::Restore,
    });
    assert_eq!(vm.data().mail(11).unwrap().folder, "inbox");
    assert_eq!(vm.data().mail(12).unwrap().folder, "deleted");
    assert_eq!(writes.get(), 2);
    vm.dispatch(Action::MailBatch {
        ids: vec![12],
        operation: MailOperation::Delete,
    });
    assert!(vm.data().mail(12).is_none());
    assert!(vm.data().mail(11).is_some());
    assert_eq!(writes.get(), 3);
}

#[test]
fn thread_bulk_read_and_delete_are_atomic_and_clean_notification_projection() {
    let (vm, writes) = fixture();
    vm.dispatch(Action::ConversationBatch {
        ids: vec![21, 22, 21],
        delete: false,
    });
    assert_eq!(writes.get(), 1);
    assert!(vm.data().conversations.iter().all(|thread| !thread.unread));
    assert!(vm.notifications.lookup(&43).is_none());
    assert!(vm.notifications.lookup(&44).is_none());
    assert_eq!(vm.data().mail.len(), 2);
    vm.dispatch(Action::ConversationBatch {
        ids: vec![21, 22],
        delete: true,
    });
    assert!(vm.data().conversations.is_empty());
    assert_eq!(writes.get(), 2);
}

#[test]
fn deleting_one_message_keeps_other_messages_and_is_idempotent() {
    let (vm, writes) = fixture();
    vm.dispatch(Action::DeleteMessage {
        conversation: 21,
        message: 31,
    });
    assert_eq!(writes.get(), 1);
    let thread = vm.data().conversation(21).unwrap().clone();
    assert_eq!(thread.messages.len(), 1);
    assert_eq!(thread.messages[0].id, 32);
    vm.dispatch(Action::DeleteMessage {
        conversation: 21,
        message: 31,
    });
    assert_eq!(writes.get(), 1);
    assert_eq!(vm.data().conversation(21).unwrap(), &thread);
}

#[test]
fn notification_groups_and_per_app_delivery_do_not_affect_other_apps() {
    let (vm, writes) = fixture();
    vm.dismiss_notification_group(41);
    assert_eq!(writes.get(), 1);
    assert_eq!(vm.data().notifications.len(), 2);
    assert!(
        vm.data()
            .notifications
            .iter()
            .all(|notice| notice.app == AppId::Messaging)
    );
    let mut settings = vm.data().settings.clone();
    settings.app_notifications.insert(
        AppId::Outlook.label().into(),
        NotificationPreference {
            action_center: false,
            ..Default::default()
        },
    );
    vm.set_settings(settings);
    let before = writes.get();
    let next_id = vm.data().next_id;
    vm.add_notification(notification(0, AppId::Outlook, Route::App(AppId::Outlook)));
    assert_eq!(writes.get(), before);
    assert_eq!(vm.data().next_id, next_id);
    assert!(
        vm.data()
            .settings
            .notification_preference(AppId::Outlook)
            .banners,
        "banner policy is independent of action-center storage"
    );
    vm.add_notification(notification(
        0,
        AppId::Messaging,
        Route::App(AppId::Messaging),
    ));
    assert_eq!(writes.get(), before + 1);
    assert_eq!(vm.data().notifications.len(), 3);
}

#[test]
fn environment_changes_notify_without_persisting_or_fabricating_values() {
    let (vm, writes) = fixture();
    let document = vm.data().clone();
    let revision = vm.revision();
    assert_eq!(vm.environment(), HostEnvironment::default());
    let environment = HostEnvironment {
        battery_percent: Some(255),
        charging: Some(true),
        online: Some(false),
        ..Default::default()
    };
    vm.set_environment(environment);
    assert_eq!(vm.environment().battery_percent, Some(100));
    assert_eq!(vm.revision(), revision + 1);
    assert_eq!(writes.get(), 0);
    assert_eq!(*vm.data(), document);
    vm.set_environment(vm.environment());
    assert_eq!(vm.revision(), revision + 1);
    vm.set_environment(HostEnvironment::default());
    assert_eq!(vm.environment().battery_percent, None);
    assert_eq!(writes.get(), 0);
}
