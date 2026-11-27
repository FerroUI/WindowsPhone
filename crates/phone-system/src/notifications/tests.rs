use super::*;
use phone_domain::{Notification, PhoneData};
use phone_host::platform::{
    DeviceNotification, DeviceOperation, DeviceRequest, DeviceResponse, DeviceSnapshot,
    PlatformRequest,
};
use phone_state::{HostRequest, HostServices};
use std::cell::RefCell;

pub(crate) fn model() -> Rc<PhoneViewModel> {
    let mut data = PhoneData::default();
    data.settings.lock_timeout = 0;
    data.settings.reduced_motion = true;
    data.settings.live_tiles = false;
    data.settings.notifications = true;
    data.settings.notifications_on_lock = true;
    data.settings.app_notifications.clear();
    data.alarms.clear();
    data.events.clear();
    data.notifications.clear();
    let id = data.allocate_id();
    data.notifications.push(Notification {
        id,
        app: AppId::Outlook,
        title: "Local mail".into(),
        body: "Visible local record".into(),
        timestamp: 3,
        route: Route::App(AppId::Outlook),
    });
    let vm = PhoneViewModel::new(data, Rc::new(|_| {}));
    vm.set_device_response(DeviceResponse::Snapshot(DeviceSnapshot {
        operations: vec![
            DeviceOperation::ReadNotifications,
            DeviceOperation::NotificationActions,
        ],
        ..Default::default()
    }))
    .unwrap();
    vm
}

pub(crate) fn notice(
    key: &str,
    package: &str,
    timestamp: i64,
    clearable: bool,
) -> DeviceNotification {
    DeviceNotification {
        key: key.into(),
        package_id: package.into(),
        application: package.into(),
        title: format!("Notice {key}"),
        body: "Native provider record".into(),
        timestamp,
        clearable,
        ongoing: !clearable,
        ..Default::default()
    }
}

pub(crate) fn record(vm: &PhoneViewModel) -> Rc<RefCell<Vec<Vec<String>>>> {
    let recorded = Rc::new(RefCell::new(Vec::new()));
    let output = recorded.clone();
    vm.set_services(HostServices {
        request: Rc::new(move |request| {
            if let HostRequest::Platform {
                request:
                    PlatformRequest::Device {
                        request: DeviceRequest::DismissNotifications { keys },
                    },
            } = request
            {
                output.borrow_mut().push(keys);
            }
        }),
    });
    recorded
}

#[test]
fn native_group_dismissal_is_bound_to_captured_clearable_keys_and_live_authority() {
    let vm = model();
    let recorded = record(&vm);
    let original = vec![
        notice("one", "z.mail", 4_000, true),
        notice("service", "z.mail", 3_000, false),
        notice("other", "a.music", 2_000, true),
    ];
    vm.set_device_response(DeviceResponse::Notifications(original.clone()))
        .unwrap();
    let target = Dismissal::from_name(&vm, "DeviceNotifications-z.mail").unwrap();
    let mut later = original.clone();
    later.push(notice("arrival", "z.mail", 5_000, true));
    vm.set_device_response(DeviceResponse::Notifications(later.clone()))
        .unwrap();
    target.dismiss(&vm);
    assert_eq!(*recorded.borrow(), vec![vec!["one"]]);
    assert_eq!(
        vm.device_state().notifications.len(),
        4,
        "the provider owns actual removal"
    );
    assert_eq!(
        vm.data().notifications.len(),
        1,
        "native dismissal preserves local records"
    );

    // A provider update may make the captured key ongoing or change its owner.
    later[0].clearable = false;
    vm.set_device_response(DeviceResponse::Notifications(later.clone()))
        .unwrap();
    target.dismiss(&vm);
    later[0].clearable = true;
    later[0].package_id = "another.app".into();
    vm.set_device_response(DeviceResponse::Notifications(later))
        .unwrap();
    target.dismiss(&vm);
    assert_eq!(recorded.borrow().len(), 1);

    vm.set_device_response(DeviceResponse::Snapshot(DeviceSnapshot {
        operations: vec![DeviceOperation::ReadNotifications],
        ..Default::default()
    }))
    .unwrap();
    target.dismiss(&vm);
    assert!(Dismissal::from_name(&vm, "DeviceNotifications-z.mail").is_none());
    assert_eq!(
        recorded.borrow().len(),
        1,
        "read-only access cannot request dismissal"
    );
}

#[test]
fn restored_documents_cannot_reuse_old_local_or_native_dismissal_commands() {
    let vm = model();
    let recorded = record(&vm);
    let rows = vec![notice("one", "z.mail", 4_000, true)];
    vm.set_device_response(DeviceResponse::Notifications(rows.clone()))
        .unwrap();
    let id = vm.data().notifications[0].id;
    let local = Dismissal::from_name(&vm, &format!("Notification-{id}")).unwrap();
    let native = Dismissal::from_name(&vm, "DeviceNotifications-z.mail").unwrap();
    let replacement = vm.data().clone();
    vm.replace_data(replacement.clone()).unwrap();
    local.dismiss(&vm);
    native.dismiss(&vm);
    assert_eq!(vm.data().notifications, replacement.notifications);
    assert!(recorded.borrow().is_empty());
}

#[test]
fn clear_all_preserves_provider_owned_rows_and_a_document_restored_by_the_host_callback() {
    let vm = model();
    vm.set_device_response(DeviceResponse::Notifications(vec![
        notice("one", "z.mail", 4_000, true),
        notice("service", "z.mail", 3_000, false),
        notice("other", "a.music", 2_000, true),
    ]))
    .unwrap();
    let replacement = vm.data().clone();
    let expected = replacement.notifications.clone();
    let model = Rc::downgrade(&vm);
    let recorded = Rc::new(RefCell::new(Vec::new()));
    let output = recorded.clone();
    vm.set_services(HostServices {
        request: Rc::new(move |request| {
            if let HostRequest::Platform {
                request:
                    PlatformRequest::Device {
                        request: DeviceRequest::DismissNotifications { keys },
                    },
            } = request
            {
                output.borrow_mut().push(keys);
                let vm = model.upgrade().unwrap();
                assert!(
                    vm.data().notifications.is_empty(),
                    "local clear completes before the host callback"
                );
                vm.replace_data(replacement.clone()).unwrap();
            }
        }),
    });
    crate::device_notifications::clear(&vm);
    assert_eq!(*recorded.borrow(), vec![vec!["one", "other"]]);
    assert_eq!(
        vm.data().notifications,
        expected,
        "a reentrant restore is not cleared afterward"
    );
}

#[test]
fn disabled_notifications_do_not_offer_or_execute_group_dismissal() {
    let vm = model();
    let recorded = record(&vm);
    vm.set_device_response(DeviceResponse::Notifications(vec![notice(
        "one", "z.mail", 4_000, true,
    )]))
    .unwrap();
    let target = Dismissal::from_name(&vm, "DeviceNotifications-z.mail").unwrap();
    let mut settings = vm.data().settings.clone();
    settings.notifications = false;
    vm.set_settings(settings);
    target.dismiss(&vm);
    crate::device_notifications::clear(&vm);
    assert!(Dismissal::from_name(&vm, "DeviceNotifications-z.mail").is_none());
    assert!(recorded.borrow().is_empty());
    assert_eq!(vm.data().notifications.len(), 1);
}

#[test]
fn clear_all_rechecks_policy_after_synchronous_local_observers() {
    use ferroui_reactive::ObservableExt;
    use ferroui_reactiveui::IReactiveNotifyPropertyChanged;
    use std::cell::Cell;

    let vm = model();
    let recorded = record(&vm);
    vm.set_device_response(DeviceResponse::Notifications(vec![notice(
        "one", "z.mail", 4_000, true,
    )]))
    .unwrap();
    let observed = Rc::downgrade(&vm);
    let once = Cell::new(false);
    let subscription = vm.changed().subscribe_next(move |_| {
        let vm = observed.upgrade().unwrap();
        if vm.data().notifications.is_empty() && !once.replace(true) {
            let mut settings = vm.data().settings.clone();
            settings.notifications = false;
            vm.set_settings(settings);
        }
    });
    crate::device_notifications::clear(&vm);
    assert!(!vm.data().settings.notifications);
    assert!(vm.data().notifications.is_empty());
    assert!(
        recorded.borrow().is_empty(),
        "policy withdrawn by a local observer prevents the later native request"
    );
    subscription.dispose();
}

#[test]
fn local_seconds_and_native_milliseconds_share_a_deterministic_group_order() {
    let mut groups = vec![
        Group {
            timestamp: 3_i64.saturating_mul(1000),
            key: "phone.outlook".into(),
            view: Element::Empty,
        },
        Group {
            timestamp: 2_000,
            key: "device.a.music".into(),
            view: Element::Empty,
        },
        Group {
            timestamp: 4_001,
            key: "device.z.mail".into(),
            view: Element::Empty,
        },
        Group {
            timestamp: 3_000,
            key: "phone.calendar".into(),
            view: Element::Empty,
        },
    ];
    chronological(&mut groups);
    assert_eq!(
        groups
            .iter()
            .map(|group| group.key.as_str())
            .collect::<Vec<_>>(),
        [
            "device.z.mail",
            "phone.calendar",
            "phone.outlook",
            "device.a.music"
        ]
    );
}

#[test]
fn mounted_action_center_orders_native_groups_cancels_swipes_and_clears_both_stores() {
    use ferroui_base::input::RawInputModifiers;
    use ferroui_base::threading::{CancellationToken, Dispatcher};
    use ferroui_base::{Point, Ref};
    use ferroui_controls::{AppBuilder, Application, Control, Window};
    use ferroui_headless::{
        FerroHeadlessPlatform, FerroHeadlessPlatformExtensions, FerroHeadlessPlatformOptions,
        HeadlessUnitTestSession, HeadlessWindowExtensions,
    };
    use phone_sdk::{AppRegistration, AppRegistry};
    use std::time::Duration;

    let session = HeadlessUnitTestSession::start_new(|| {
        AppBuilder::configure::<Application>().use_headless(FerroHeadlessPlatformOptions::default())
    });
    session.dispatch(|| {
        phone_controls::install_theme(&Application::current().unwrap());
        let vm = model();
        vm.set_viewport(400.0, 820.0);
        let rows = vec![
            notice("new", "z.mail", 4_000, true), notice("old", "a.music", 2_000, true),
            notice("service", "ongoing.app", 1_000, false),
        ];
        vm.set_device_response(DeviceResponse::Notifications(rows.clone())).unwrap();
        let recorded = record(&vm);
        let registry = Rc::new(AppRegistry::new(vec![
            AppRegistration::builtin(AppId::Outlook, |_, _| Element::Empty),
        ]).unwrap());
        let root = crate::create_view(vm.clone(), registry);
        let window = Window::new();
        window.set_width(400.0);
        window.set_height(820.0);
        window.set_content(Some(Control::boxed(root.clone())));
        window.show();
        let frame = || {
            Dispatcher::ui_thread().run_jobs(None);
            FerroHeadlessPlatform::force_render_timer_tick(1);
            Dispatcher::ui_thread().run_jobs(None);
        };
        frame();
        let ui = crate::SESSION.with(|slot| slot.borrow().upgrade().unwrap());
        ui.notification_open.set(true);
        frame();
        let named = |name: &str| -> Ref<Control> {
            root.find_descendant_of_type_where::<Control>(false, |control|control.has_name(name))
                .unwrap_or_else(||panic!("missing mounted Action Center control {name}"))
        };
        let origin = |control: &Control| control.translate_point(Point::new(0.0, 0.0), &window).unwrap();
        let newest = named("DeviceNotifications-z.mail");
        let local_id = vm.data().notifications[0].id;
        let local = named(&format!("Notification-{local_id}"));
        let older = named("DeviceNotifications-a.music");
        assert!(origin(&newest).y < origin(&local).y && origin(&local).y < origin(&older).y,
            "native package alphabetical order cannot displace timestamp order across stores");
        let point = origin(&newest);
        let from = Point::new(point.x + 36.0, point.y + 42.0);
        let to = Point::new(from.x + 100.0, from.y);
        let canceled = window.touch_begin(from, RawInputModifiers::NONE);
        window.touch_move(&*canceled, to, RawInputModifiers::NONE);
        frame();
        assert!(newest.translation().x > 20.0, "the actual native group tracks its touch contact");
        canceled.dispose();
        frame();
        assert_eq!(newest.translation().x, 0.0, "cancellation restores the acquired visual");
        assert_eq!(newest.opacity(), 1.0);
        assert!(recorded.borrow().is_empty());
        assert_eq!(vm.data().notifications.len(), 1);

        let swipe = window.touch_begin(from, RawInputModifiers::NONE);
        window.touch_move(&*swipe, to, RawInputModifiers::NONE);
        window.touch_end(&*swipe, to, RawInputModifiers::NONE);
        frame();
        assert_eq!(*recorded.borrow(), vec![vec!["new"]]);
        assert_eq!(vm.route(), Route::Start, "a horizontal row sequence cannot activate the row");
        assert_eq!(vm.device_state().notifications.len(), 3, "wait for a provider observation before removing rows");
        vm.set_device_response(DeviceResponse::Notifications(rows[1..].to_vec())).unwrap();
        frame();
        assert!(root.find_descendant_of_type_where::<Control>(false, |control|control.has_name("DeviceNotifications-z.mail")).is_none());

        let local = named(&format!("Notification-{local_id}"));
        let p = origin(&local);
        let first = Point::new(p.x + 32.0, p.y + 42.0);
        let second = Point::new(first.x, first.y + 26.0);
        let a = window.touch_begin(first, RawInputModifiers::NONE);
        let b = window.touch_begin(second, RawInputModifiers::NONE);
        window.touch_move(&*a, Point::new(first.x + 100.0, first.y), RawInputModifiers::NONE);
        window.touch_end(&*a, Point::new(first.x + 100.0, first.y), RawInputModifiers::NONE);
        window.touch_end(&*b, second, RawInputModifiers::NONE);
        frame();
        assert_eq!(recorded.borrow().len(), 1, "a stationary second contact does not clear anything");
        assert_eq!(vm.data().notifications.len(), 1);

        let a = window.touch_begin(first, RawInputModifiers::NONE);
        let b = window.touch_begin(second, RawInputModifiers::NONE);
        let end_a = Point::new(first.x + 100.0, first.y);
        let end_b = Point::new(second.x + 100.0, second.y);
        window.touch_move(&*a, end_a, RawInputModifiers::NONE);
        window.touch_move(&*b, end_b, RawInputModifiers::NONE);
        window.touch_end(&*a, end_a, RawInputModifiers::NONE);
        window.touch_end(&*b, end_b, RawInputModifiers::NONE);
        frame();
        assert!(vm.data().notifications.is_empty(), "the parallel gesture clears the local store");
        assert_eq!(*recorded.borrow(), vec![vec!["new"], vec!["old"]],
            "the same gesture asks the native provider to clear its remaining clearable key, excluding ongoing rows");
        assert_eq!(vm.device_state().notifications.len(), 2);
        crate::host_closing(&vm);
        window.close();
        window.set_content(None);
        frame();
    }, CancellationToken::none())
        .wait_timeout(Duration::from_secs(30))
        .expect("mounted native Action Center gestures complete");
    session.dispose();
}
