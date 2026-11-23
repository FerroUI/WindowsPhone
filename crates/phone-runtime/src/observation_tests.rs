use super::*;
use phone_domain::{PhoneData, Route};
use phone_host::HostPlatform;
use std::{
    future::Future,
    pin::Pin,
    task::{Context, Poll, Waker},
};

#[derive(Default)]
struct Reply {
    result: RefCell<Option<Result<PlatformResponse, String>>>,
    waker: RefCell<Option<Waker>>,
    dropped: Cell<bool>,
}
struct Deferred(Rc<Reply>);
impl Future for Deferred {
    type Output = Result<PlatformResponse, String>;
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        if let Some(result) = self.0.result.borrow_mut().take() {
            Poll::Ready(result)
        } else {
            *self.0.waker.borrow_mut() = Some(cx.waker().clone());
            Poll::Pending
        }
    }
}
impl Drop for Deferred {
    fn drop(&mut self) {
        self.0.dropped.set(true);
    }
}
struct Provider {
    features: RefCell<Vec<HostFeature>>,
    requests: RefCell<Vec<(String, Rc<Reply>)>>,
    events: RefCell<Vec<PlatformResponse>>,
}
impl PlatformPlugin for Provider {
    fn id(&self) -> &'static str {
        "test.foreground.observations"
    }
    fn state(&self) -> PlatformState {
        PlatformState {
            platform: HostPlatform::Android,
            mode: HostMode::App,
            launcher: HostRole::Unsupported,
            shell: HostRole::Unsupported,
            features: self.features.borrow().clone(),
            detail: String::new(),
        }
    }
    fn dispatch(&self, request: PlatformRequest) -> PlatformTask {
        let key = match &request {
            PlatformRequest::Device {
                request: DeviceRequest::Snapshot,
            } => "snapshot".into(),
            PlatformRequest::Device {
                request: DeviceRequest::MessageThreads { limit },
            } => format!("threads:{limit}"),
            PlatformRequest::Device {
                request: DeviceRequest::Notifications,
            } => "notifications".into(),
            PlatformRequest::Device {
                request: DeviceRequest::Contacts { query, limit },
            } => format!("contacts:{query}:{limit}"),
            _ => format!("{request:?}"),
        };
        let reply = Rc::new(Reply::default());
        self.requests.borrow_mut().push((key, reply.clone()));
        Box::pin(Deferred(reply))
    }
    fn drain_events(&self) -> Vec<PlatformResponse> {
        std::mem::take(&mut *self.events.borrow_mut())
    }
}
impl Provider {
    fn keys(&self) -> Vec<String> {
        self.requests
            .borrow()
            .iter()
            .map(|(key, _)| key.clone())
            .collect()
    }
    fn reply(&self, index: usize, value: Result<PlatformResponse, String>) {
        let reply = self.requests.borrow()[index].1.clone();
        *reply.result.borrow_mut() = Some(value);
        let waker = reply.waker.borrow_mut().take();
        if let Some(waker) = waker {
            waker.wake();
        }
    }
    fn dropped(&self, index: usize) -> bool {
        self.requests.borrow()[index].1.dropped.get()
    }
}
fn pump() {
    Dispatcher::current_dispatcher().run_jobs(None);
}
fn fixture(features: Vec<HostFeature>) -> (Rc<PhoneRuntime>, Rc<Provider>) {
    let provider = Rc::new(Provider {
        features: RefCell::new(features),
        requests: RefCell::default(),
        events: RefCell::default(),
    });
    let vm = PhoneViewModel::new(PhoneData::demo(), Rc::new(|_| {}));
    let runtime = PhoneRuntime::attach(vm, provider.clone(), ServiceRegistry::default()).unwrap();
    (runtime, provider)
}
fn device(request: DeviceRequest) -> PlatformRequest {
    PlatformRequest::Device { request }
}
fn response(value: DeviceResponse) -> PlatformResponse {
    PlatformResponse::Device(value)
}
fn granted() -> DeviceSnapshot {
    DeviceSnapshot {
        operations: vec![
            DeviceOperation::ReadMessages,
            DeviceOperation::ReadNotifications,
        ],
        ..Default::default()
    }
}
fn threads(unread: u32) -> PlatformResponse {
    response(DeviceResponse::MessageThreads(vec![DeviceMessageThread {
        id: "thread:7".into(),
        title: "Sender".into(),
        snippet: "Observed private message".into(),
        unread,
        ..Default::default()
    }]))
}
fn notices() -> PlatformResponse {
    response(DeviceResponse::Notifications(vec![DeviceNotification {
        key: "notice:1".into(),
        package_id: "org.example".into(),
        application: "Example".into(),
        title: "Observed notice".into(),
        ..Default::default()
    }]))
}
fn boot() -> (Rc<PhoneRuntime>, Rc<Provider>) {
    let (runtime, provider) = fixture(vec![
        HostFeature::Messaging,
        HostFeature::NotificationCenter,
    ]);
    assert_eq!(provider.keys(), ["snapshot"]);
    provider.reply(0, Ok(response(DeviceResponse::Snapshot(granted()))));
    pump();
    assert_eq!(
        provider.keys(),
        ["snapshot", "threads:200", "notifications"]
    );
    (runtime, provider)
}
fn close(runtime: &PhoneRuntime) {
    runtime.lifecycle(HostLifecycle::Closing);
    pump();
}

#[test]
fn cold_start_initializes_authorized_private_rows_without_visiting_an_app() {
    let (runtime, provider) = boot();
    provider.reply(1, Ok(threads(2)));
    provider.reply(2, Ok(notices()));
    pump();
    assert_eq!(runtime.vm.route(), Route::Start);
    assert_eq!(runtime.vm.device_state().threads[0].unread, 2);
    assert_eq!(runtime.vm.device_state().notifications.len(), 1);
    for _ in 0..20 {
        runtime.observe();
    }
    runtime.vm.set_status("A user-owned status".into());
    assert_eq!(provider.keys().len(), 3);
    close(&runtime);
}

#[test]
fn display_notifications_alone_and_ungranted_capabilities_never_read_private_data() {
    let (runtime, provider) = fixture(vec![HostFeature::Notifications, HostFeature::Location]);
    for _ in 0..20 {
        runtime.observe();
    }
    assert!(provider.keys().is_empty());
    close(&runtime);
    let (runtime, provider) = fixture(vec![HostFeature::Contacts, HostFeature::Messaging]);
    provider.reply(
        0,
        Ok(response(
            DeviceResponse::Snapshot(DeviceSnapshot::default()),
        )),
    );
    pump();
    assert_eq!(provider.keys(), ["snapshot"]);
    close(&runtime);
}

#[test]
fn equivalent_reads_coalesce_but_page_limits_and_callbacks_keep_their_ownership() {
    let (runtime, provider) = boot();
    runtime
        .dispatch_platform(device(DeviceRequest::MessageThreads { limit: 200 }), None)
        .unwrap();
    assert_eq!(provider.keys().len(), 3);
    runtime
        .dispatch_platform(device(DeviceRequest::MessageThreads { limit: 300 }), None)
        .unwrap();
    pump();
    assert!(provider.dropped(1));
    assert_eq!(provider.keys()[3], "threads:300");
    provider.reply(3, Ok(threads(3)));
    provider.reply(2, Ok(notices()));
    pump();
    assert_eq!(runtime.vm.device_state().threads[0].unread, 3);
    close(&runtime);

    let (runtime, provider) = boot();
    let completed = Rc::new(Cell::new(false));
    let mark = completed.clone();
    runtime
        .dispatch_platform(
            device(DeviceRequest::MessageThreads { limit: 200 }),
            Some(Box::new(move |_, _| {
                mark.set(true);
                Ok(())
            })),
        )
        .unwrap();
    provider.reply(3, Ok(threads(99)));
    pump();
    assert!(completed.get());
    assert!(runtime.vm.device_state().threads.is_empty());
    assert_eq!(
        provider.keys()[4],
        "threads:200",
        "a consuming callback does not initialize Start"
    );
    provider.reply(4, Ok(threads(2)));
    pump();
    assert_eq!(runtime.vm.device_state().threads[0].unread, 2);
    close(&runtime);
}

#[test]
fn revoke_regrant_discards_old_queries_and_unsolicited_rows_until_fresh_initialization() {
    let (runtime, provider) = boot();
    let old = runtime.channels.borrow()[&6];
    let epoch = runtime.vm.document_epoch();
    provider.events.borrow_mut().extend([
        response(DeviceResponse::Snapshot(DeviceSnapshot::default())),
        response(DeviceResponse::Snapshot(granted())),
        threads(99),
        notices(),
    ]);
    runtime.observe();
    pump();
    assert!(provider.dropped(1) && provider.dropped(2));
    assert!(runtime.vm.device_state().threads.is_empty());
    assert!(runtime.vm.device_state().notifications.is_empty());
    runtime.deliver_completion(epoch, Some(6), old, Ok(threads(99)), None);
    assert!(runtime.vm.device_state().threads.is_empty());
    assert_eq!(provider.keys()[3..], ["threads:200", "notifications"]);
    provider.reply(3, Ok(threads(4)));
    provider.reply(4, Ok(notices()));
    pump();
    assert_eq!(runtime.vm.device_state().threads[0].unread, 4);
    provider
        .events
        .borrow_mut()
        .push(response(DeviceResponse::Notifications(Vec::new())));
    runtime.observe();
    assert!(runtime.vm.device_state().notifications.is_empty());
    close(&runtime);
}

fn contact_snapshot(status: PermissionState) -> DeviceSnapshot {
    DeviceSnapshot {
        operations: vec![DeviceOperation::ReadContacts],
        permissions: vec![DevicePermissionState {
            permission: DevicePermission::Contacts,
            status,
        }],
        ..Default::default()
    }
}
#[test]
fn contacts_cancel_across_regrant_and_retire_the_old_granted_subset_on_limited_access() {
    let (runtime, provider) = fixture(vec![HostFeature::Contacts]);
    provider.reply(
        0,
        Ok(response(DeviceResponse::Snapshot(contact_snapshot(
            PermissionState::Granted,
        )))),
    );
    pump();
    runtime
        .dispatch_platform(
            device(DeviceRequest::Contacts {
                query: "old".into(),
                limit: 200,
            }),
            None,
        )
        .unwrap();
    let serial = runtime.channels.borrow()[&3];
    let private = || {
        response(DeviceResponse::Contacts(vec![DeviceContact {
            id: "contact:1".into(),
            name: "Private".into(),
            ..Default::default()
        }]))
    };
    provider.events.borrow_mut().extend([
        response(DeviceResponse::Snapshot(contact_snapshot(
            PermissionState::Denied,
        ))),
        response(DeviceResponse::Snapshot(contact_snapshot(
            PermissionState::Granted,
        ))),
    ]);
    runtime.observe();
    pump();
    assert!(provider.dropped(1));
    runtime.deliver_completion(
        runtime.vm.document_epoch(),
        Some(3),
        serial,
        Ok(private()),
        None,
    );
    assert!(runtime.vm.device_state().contacts.is_empty());
    runtime.accept(private()).unwrap();
    runtime
        .dispatch_platform(
            device(DeviceRequest::Contacts {
                query: "subset".into(),
                limit: 200,
            }),
            None,
        )
        .unwrap();
    provider
        .events
        .borrow_mut()
        .push(response(DeviceResponse::Snapshot(contact_snapshot(
            PermissionState::Limited,
        ))));
    runtime.observe();
    pump();
    assert!(provider.dropped(2));
    assert!(runtime.vm.device_state().contacts.is_empty());
    close(&runtime);
}

#[test]
fn every_private_read_channel_is_cancelled_and_attachment_authority_is_independent() {
    let (runtime, provider) = fixture(vec![
        HostFeature::Contacts,
        HostFeature::Telephony,
        HostFeature::Messaging,
        HostFeature::NotificationCenter,
        HostFeature::AppShortcuts,
    ]);
    let mut snapshot = contact_snapshot(PermissionState::Granted);
    snapshot.operations.extend([
        DeviceOperation::CallHistory,
        DeviceOperation::ActiveCalls,
        DeviceOperation::ReadMessages,
        DeviceOperation::ReadNotifications,
        DeviceOperation::AppShortcuts,
        DeviceOperation::MessageAttachments,
    ]);
    provider.reply(0, Ok(response(DeviceResponse::Snapshot(snapshot.clone()))));
    pump();
    for request in [
        DeviceRequest::Contacts {
            query: String::new(),
            limit: 200,
        },
        DeviceRequest::CallHistory { limit: 200 },
        DeviceRequest::Calls,
        DeviceRequest::Messages {
            thread_id: "thread:7".into(),
            limit: 200,
        },
        DeviceRequest::MessageAttachments {
            message_id: "mms:7".into(),
        },
        DeviceRequest::Shortcuts {
            application: "org.example/.Main".into(),
        },
    ] {
        runtime.dispatch_platform(device(request), None).unwrap();
    }
    pump();
    assert_eq!(provider.keys().len(), 9);
    snapshot
        .operations
        .retain(|operation| *operation != DeviceOperation::MessageAttachments);
    provider
        .events
        .borrow_mut()
        .push(response(DeviceResponse::Snapshot(snapshot)));
    runtime.observe();
    pump();
    assert!(provider.dropped(7));
    for index in [1, 2, 3, 4, 5, 6, 8] {
        assert!(!provider.dropped(index));
    }
    provider.events.borrow_mut().push(response(
        DeviceResponse::Snapshot(DeviceSnapshot::default()),
    ));
    runtime.observe();
    pump();
    for index in 1..9 {
        assert!(
            provider.dropped(index),
            "private read {index} retained obsolete authority"
        );
    }
    close(&runtime);
}

#[test]
fn a_smaller_explicit_thread_page_does_not_complete_the_shells_initial_page() {
    let (runtime, provider) = boot();
    runtime
        .dispatch_platform(device(DeviceRequest::MessageThreads { limit: 1 }), None)
        .unwrap();
    provider.reply(3, Ok(threads(1)));
    pump();
    assert_eq!(runtime.vm.device_state().threads[0].unread, 1);
    assert_eq!(provider.keys()[4], "threads:200");
    provider.reply(4, Ok(threads(2)));
    pump();
    assert_eq!(runtime.vm.device_state().threads[0].unread, 2);
    close(&runtime);
}

#[test]
fn suspension_cancels_reads_rejects_delayed_reads_and_keeps_side_effect_tasks_independent() {
    let (runtime, provider) = boot();
    runtime
        .dispatch_platform(
            device(DeviceRequest::MarkMessagesRead {
                thread_id: "thread:7".into(),
            }),
            None,
        )
        .unwrap();
    runtime.lifecycle(HostLifecycle::Suspended);
    pump();
    assert!(provider.dropped(1) && provider.dropped(2));
    assert!(
        !provider.dropped(3),
        "an already requested side effect retains its own completion"
    );
    assert!(
        runtime
            .dispatch_platform(device(DeviceRequest::Notifications), None)
            .is_err()
    );
    for _ in 0..20 {
        runtime.observe();
    }
    assert_eq!(provider.keys().len(), 4);
    runtime.lifecycle(HostLifecycle::Resumed);
    assert_eq!(provider.keys()[4], "snapshot");
    runtime.lifecycle(HostLifecycle::Resumed);
    assert_eq!(provider.keys().len(), 5);
    provider.reply(4, Ok(response(DeviceResponse::Snapshot(granted()))));
    pump();
    assert_eq!(provider.keys()[5..], ["threads:200", "notifications"]);
    close(&runtime);
}

#[test]
fn restore_and_feature_withdrawal_require_fresh_capabilities_without_retaining_private_rows() {
    let (runtime, provider) = boot();
    provider.reply(1, Ok(threads(2)));
    provider.reply(2, Ok(notices()));
    pump();
    runtime
        .vm
        .replace_data_after_save(PhoneData::demo(), |_| Ok(()))
        .unwrap();
    runtime.observe();
    assert!(runtime.vm.device_state().threads.is_empty());
    assert_eq!(provider.keys()[3], "snapshot");
    provider.reply(3, Ok(response(DeviceResponse::Snapshot(granted()))));
    pump();
    provider.reply(4, Ok(threads(3)));
    provider.reply(5, Ok(notices()));
    pump();
    provider
        .features
        .borrow_mut()
        .retain(|feature| *feature != HostFeature::Messaging);
    runtime.observe();
    assert!(runtime.vm.device_state().threads.is_empty());
    assert!(
        !runtime
            .vm
            .device_state()
            .snapshot
            .supports(DeviceOperation::ReadMessages)
    );
    assert_eq!(provider.keys()[6], "snapshot");
    provider.reply(6, Ok(response(DeviceResponse::Snapshot(granted()))));
    pump();
    assert!(
        !provider.keys()[7..]
            .iter()
            .any(|key| key.starts_with("threads:"))
    );
    close(&runtime);
}

#[test]
fn failed_automatic_reads_back_off_and_preserve_user_status_then_recover() {
    let (runtime, provider) = fixture(vec![HostFeature::Messaging]);
    runtime.vm.set_status("Keep this user result".into());
    let now = runtime.vm.now();
    provider.reply(0, Err("temporary snapshot failure".into()));
    pump();
    assert_eq!(runtime.vm.status(), "Keep this user result");
    for _ in 0..20 {
        runtime.observe();
    }
    runtime.vm.set_clock(now + 3);
    runtime.observe();
    assert_eq!(provider.keys().len(), 1);
    runtime.vm.set_clock(now + 4);
    runtime.observe();
    assert_eq!(provider.keys(), ["snapshot", "snapshot"]);
    provider.reply(1, Ok(response(DeviceResponse::Snapshot(granted()))));
    pump();
    provider.reply(2, Err("temporary thread failure".into()));
    pump();
    assert_eq!(runtime.vm.status(), "Keep this user result");
    runtime.vm.set_clock(now + 7);
    runtime.observe();
    assert_eq!(provider.keys().len(), 3);
    runtime.vm.set_clock(now + 8);
    runtime.observe();
    provider.reply(3, Ok(threads(5)));
    pump();
    assert_eq!(runtime.vm.device_state().threads[0].unread, 5);
    close(&runtime);
}

#[test]
fn reentrant_restore_and_close_cannot_launch_or_admit_a_former_bootstrap() {
    let (runtime, provider) = fixture(vec![HostFeature::Messaging]);
    let weak = Rc::downgrade(&runtime);
    let changed = Rc::new(Cell::new(false));
    let once = changed.clone();
    let subscription = runtime.vm.changed().subscribe_next(move |_| {
        let runtime = weak.upgrade().unwrap();
        runtime.observe();
        if runtime
            .vm
            .device_state()
            .snapshot
            .supports(DeviceOperation::ReadMessages)
            && !once.replace(true)
        {
            runtime
                .vm
                .replace_data_after_save(PhoneData::demo(), |_| Ok(()))
                .unwrap();
        }
    });
    provider.reply(0, Ok(response(DeviceResponse::Snapshot(granted()))));
    pump();
    subscription.dispose();
    assert!(changed.get());
    assert!(runtime.vm.device_state().threads.is_empty());
    assert!(
        !provider
            .keys()
            .iter()
            .any(|key| key.starts_with("threads:"))
    );
    runtime.observe();
    assert_eq!(provider.keys().last().unwrap(), "snapshot");
    close(&runtime);
    assert!(provider.dropped(provider.keys().len() - 1));
    let count = provider.keys().len();
    runtime.observe();
    runtime.lifecycle(HostLifecycle::Resumed);
    assert_eq!(provider.keys().len(), count);
}
