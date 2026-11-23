use super::*;
use ferroui_reactive::ObservableExt;
use ferroui_reactiveui::IReactiveNotifyPropertyChanged;
use phone_domain::PhoneData;
use phone_host::HostPlatform;

struct Provider {
    battery: Cell<u8>,
    calls: Cell<usize>,
    closes: Cell<usize>,
    events: RefCell<Vec<PlatformResponse>>,
    policies: RefCell<Vec<DeviceNotificationPolicy>>,
}
impl PlatformPlugin for Provider {
    fn drain_events(&self) -> Vec<PlatformResponse> {
        std::mem::take(&mut *self.events.borrow_mut())
    }
    fn set_notification_policy(&self, policy: &DeviceNotificationPolicy) {
        self.policies.borrow_mut().push(*policy);
    }
    fn id(&self) -> &'static str {
        "test.runtime.observed-provider"
    }
    fn state(&self) -> PlatformState {
        PlatformState {
            platform: HostPlatform::Linux,
            mode: HostMode::Embedded,
            launcher: HostRole::Unsupported,
            shell: HostRole::Unsupported,
            features: vec![
                HostFeature::InstalledApps,
                HostFeature::Location,
                HostFeature::Haptics,
                HostFeature::Notifications,
            ],
            detail: String::new(),
        }
    }
    fn environment(&self) -> HostEnvironment {
        HostEnvironment {
            platform: HostPlatform::Linux,
            battery_percent: Some(self.battery.get()),
            notification_permission: HostPermission::Granted,
            ..Default::default()
        }
    }
    fn dispatch(&self, _: PlatformRequest) -> PlatformTask {
        self.calls.set(self.calls.get() + 1);
        ready(Ok(PlatformResponse::Accepted {
            message: "accepted".into(),
        }))
    }
    fn lifecycle(&self, event: HostLifecycle) {
        if event == HostLifecycle::Closing {
            self.closes.set(self.closes.get() + 1);
        }
    }
}

fn fixture(services: ServiceRegistry) -> (Rc<PhoneRuntime>, Rc<Provider>) {
    let provider = Rc::new(Provider {
        battery: Cell::new(80),
        calls: Cell::new(0),
        closes: Cell::new(0),
        events: RefCell::new(vec![]),
        policies: RefCell::new(vec![]),
    });
    let vm = PhoneViewModel::new(PhoneData::demo(), Rc::new(|_| {}));
    (
        PhoneRuntime::attach(vm, provider.clone(), services).unwrap(),
        provider,
    )
}

#[test]
fn notification_policy_follows_settings_and_restore_without_permission_requests() {
    let (runtime, provider) = fixture(ServiceRegistry::default());
    assert_eq!(provider.policies.borrow().len(), 1);
    let mut settings = runtime.vm.data().settings.clone();
    settings.notifications = false;
    runtime.vm.set_settings(settings);
    assert!(!provider.policies.borrow().last().unwrap().enabled);
    assert_eq!(provider.calls.get(), 0);
    runtime.vm.set_status("Unrelated UI status".into());
    assert_eq!(provider.policies.borrow().len(), 2);
    let mut restored = PhoneData::demo();
    restored.settings.notifications = true;
    restored
        .settings
        .app_notifications
        .entry("Messaging".into())
        .or_default()
        .sound = false;
    runtime
        .vm
        .replace_data_after_save(restored, |_| Ok(()))
        .unwrap();
    assert!(provider.policies.borrow().last().unwrap().enabled);
    assert!(!provider.policies.borrow().last().unwrap().message_sound);
    runtime.lifecycle(HostLifecycle::Closing);
    let count = provider.policies.borrow().len();
    let mut settings = runtime.vm.data().settings.clone();
    settings.notifications = false;
    runtime.vm.set_settings(settings);
    assert_eq!(provider.policies.borrow().len(), count);
}
#[test]
fn device_event_batches_stop_at_restore_and_reentrant_observation_is_bounded() {
    let (runtime, provider) = fixture(ServiceRegistry::default());
    provider.events.borrow_mut().extend([
        PlatformResponse::Device(DeviceResponse::Snapshot(DeviceSnapshot::default())),
        PlatformResponse::Device(DeviceResponse::ComposeDraft(NativeMessageDraft {
            id: "stale-intent".into(),
            recipients: vec!["123".into()],
            body: "old scope".into(),
        })),
    ]);
    let once = Rc::new(Cell::new(false));
    let flag = once.clone();
    let weak = Rc::downgrade(&runtime);
    let subscription = runtime.vm.changed().subscribe_next(move |_| {
        if !flag.replace(true) {
            let runtime = weak.upgrade().unwrap();
            runtime.observe();
            runtime
                .vm
                .replace_data_after_save(PhoneData::demo(), |_| Ok(()))
                .unwrap();
        }
    });
    runtime.observe();
    subscription.dispose();
    assert!(once.get());
    assert!(runtime.vm.device_state().compose_draft.is_none());
    assert!(!runtime.observing.get());
}
#[test]
fn admission_failures_complete_the_specific_send_or_attachment_selection() {
    let (runtime, provider) = fixture(ServiceRegistry::default());
    assert!(runtime
        .request(HostRequest::Platform {
            request: PlatformRequest::Device {
                request: DeviceRequest::SendMessage {
                    recipients: vec!["123".into()],
                    body: "draft".into(),
                    subscription: None,
                    client_id: "send-1".into()
                }
            }
        })
        .is_err());
    assert_eq!(
        runtime
            .vm
            .device_state()
            .send_results
            .get("send-1")
            .unwrap()
            .1,
        MessageDelivery::Failed
    );
    assert!(runtime
        .request(HostRequest::Platform {
            request: PlatformRequest::Device {
                request: DeviceRequest::PickMessageAttachments {
                    client_id: "picker-1".into()
                }
            }
        })
        .is_err());
    let state = runtime.vm.device_state();
    assert_eq!(state.draft_attachment_client.as_deref(), Some("picker-1"));
    assert!(state.draft_attachment_cancelled);
    assert_eq!(provider.calls.get(), 0);
}
fn catalog(title: &str) -> PlatformResponse {
    PlatformResponse::Applications(vec![DeviceApplication {
        id: "device.app".into(),
        title: title.into(),
    }])
}

#[derive(Clone, Copy, Debug)]
enum Invalidate {
    Close,
    Restore,
}
impl Invalidate {
    fn apply(self, runtime: &PhoneRuntime) {
        match self {
            Self::Close => runtime.lifecycle(HostLifecycle::Closing),
            Self::Restore => {
                let mut replacement = PhoneData::demo();
                replacement.settings.volume = 7;
                runtime
                    .vm
                    .replace_data_after_save(replacement, |_| Ok(()))
                    .unwrap();
            }
        }
        runtime
            .vm
            .set_status("The new scope owns this status.".into());
    }
}

#[test]
fn reactive_scope_changes_discard_old_results_errors_and_custom_completions() {
    for invalidation in [Invalidate::Close, Invalidate::Restore] {
        for response_kind in ["result", "error", "custom"] {
            let (runtime, provider) = fixture(ServiceRegistry::default());
            let epoch = runtime.vm.document_epoch();
            let observed = Rc::new(Cell::new(false));
            let once = observed.clone();
            let weak = Rc::downgrade(&runtime);
            let subscription = runtime.vm.changed().subscribe_next(move |_| {
                if !once.replace(true) {
                    invalidation.apply(&weak.upgrade().unwrap());
                }
            });
            // Force a real environment notification inside deliver_completion.
            provider.battery.set(79);
            let called = Rc::new(Cell::new(false));
            let record = called.clone();
            let complete: Option<Completion> = (response_kind == "custom").then(|| {
                Box::new(move |_: &Rc<PhoneViewModel>, _: PlatformResponse| {
                    record.set(true);
                    Err("Obsolete callback error".into())
                }) as Completion
            });
            runtime.deliver_completion(
                epoch,
                None,
                1,
                if response_kind == "error" {
                    Err("Obsolete provider error".into())
                } else {
                    Ok(catalog("Obsolete catalog"))
                },
                complete,
            );
            subscription.dispose();
            assert!(observed.get(), "{invalidation:?}: observer did not run");
            assert!(
                !called.get(),
                "{invalidation:?}: custom handler escaped admission"
            );
            assert!(
                runtime.vm.device_applications().is_empty(),
                "{invalidation:?}: stale catalog admitted"
            );
            assert_eq!(runtime.vm.status(), "The new scope owns this status.");
            match invalidation {
                Invalidate::Close => assert!(runtime.is_closed()),
                Invalidate::Restore => {
                    assert_ne!(runtime.vm.document_epoch(), epoch);
                    assert_eq!(runtime.vm.data().settings.volume, 7);
                }
            }
        }
    }
    // Positive control and last-response-wins admission use that same boundary.
    let (runtime, _) = fixture(ServiceRegistry::default());
    runtime.channels.borrow_mut().insert(1, 2);
    let epoch = runtime.vm.document_epoch();
    runtime.deliver_completion(epoch, Some(1), 1, Ok(catalog("Old")), None);
    assert!(runtime.vm.device_applications().is_empty());
    runtime.deliver_completion(epoch, Some(1), 2, Ok(catalog("Current")), None);
    assert_eq!(runtime.vm.device_applications()[0].title, "Current");
}

#[test]
fn a_custom_completion_cannot_report_errors_after_replacing_or_closing_its_scope() {
    for invalidation in [Invalidate::Close, Invalidate::Restore] {
        let (runtime, _) = fixture(ServiceRegistry::default());
        let called = Rc::new(Cell::new(false));
        let record = called.clone();
        let weak = Rc::downgrade(&runtime);
        runtime.deliver_completion(
            runtime.vm.document_epoch(),
            None,
            1,
            Ok(PlatformResponse::Accepted {
                message: String::new(),
            }),
            Some(Box::new(move |_, _| {
                record.set(true);
                invalidation.apply(&weak.upgrade().unwrap());
                Err("A failure from the former scope".into())
            })),
        );
        assert!(called.get());
        assert_eq!(runtime.vm.status(), "The new scope owns this status.");
    }
}

#[test]
fn disabled_phone_settings_block_sensitive_platform_actions_before_provider_dispatch() {
    let (runtime, provider) = fixture(ServiceRegistry::default());
    let mut settings = runtime.vm.data().settings.clone();
    settings.location = false;
    settings.vibration = false;
    settings.notifications = false;
    runtime.vm.set_settings(settings);
    for (request, expected) in [
        (
            PlatformRequest::Locate,
            "Turn on location in the phone's Settings first.",
        ),
        (
            PlatformRequest::Haptic {
                kind: HapticKind::Selection,
            },
            "Turn on vibration in the phone's Settings first.",
        ),
        (
            PlatformRequest::Notify {
                title: "Reminder".into(),
                body: "Body".into(),
            },
            "Turn on notifications in the phone's Settings first.",
        ),
    ] {
        assert!(provider.state().supports(request.feature().unwrap()));
        assert_eq!(
            runtime.dispatch_platform(request, None).unwrap_err(),
            expected
        );
        assert_eq!(
            provider.calls.get(),
            0,
            "denied setting reached the native provider"
        );
        assert_eq!(runtime.serial.get(), 0, "denied setting started a task");
    }
}

#[test]
fn closing_is_reentrant_once_and_still_allows_final_cleanup_requests() {
    let requests = Rc::new(Cell::new(0));
    let record = requests.clone();
    let mut services = ServiceRegistry::default();
    services
        .register(
            "test.cleanup",
            HostService::Clipboard,
            Rc::new(move |_| {
                record.set(record.get() + 1);
                Ok(())
            }),
        )
        .unwrap();
    let (runtime, provider) = fixture(services);
    let nested = Cell::new(false);
    runtime.close_with(|| {
        // This is the callback boundary used by phone_system::host_closing.
        runtime.lifecycle(HostLifecycle::Closing);
        runtime.close_with(|| nested.set(true));
        runtime
            .request(HostRequest::CopyText {
                text: "final cleanup".into(),
            })
            .unwrap();
    });
    runtime.lifecycle(HostLifecycle::Closing);
    assert!(!nested.get());
    assert_eq!(requests.get(), 1);
    assert_eq!(provider.closes.get(), 1);
    assert!(runtime.is_closed());
    assert!(runtime
        .request(HostRequest::CopyText {
            text: "too late".into()
        })
        .is_err());
    assert!(runtime.gate.reserve().is_err());
}
