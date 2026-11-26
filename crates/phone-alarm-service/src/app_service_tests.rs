use super::*;
use phone_domain::{PhoneData, Route};
use phone_sdk::{AppManifest, AppRegistration, AppRegistry, Capability};
use phone_state::{HostEnvironment, HostPermission, HostServices};

const APP: &str = "test.tasks";
fn provider(vm: &PhoneViewModel) -> Result<Vec<phone_sdk::AppReminder>, String> {
    let value = vm.app_data(APP).unwrap_or_else(|| serde_json::json!([]));
    value
        .as_array()
        .ok_or_else(|| "Invalid fixture".to_owned())?
        .iter()
        .map(|item| {
            Ok(phone_sdk::AppReminder {
                key: item["key"].as_str().unwrap().into(),
                revision: item["revision"].as_u64().unwrap(),
                scheduled_local: item["time"].as_i64().unwrap(),
                title: item["title"].as_str().unwrap().into(),
                detail: "Task fixture".into(),
                path: "task/1".into(),
            })
        })
        .collect()
}
fn item(key: &str, title: &str, time: i64, revision: u64) -> serde_json::Value {
    serde_json::json!({"key":key,"title":title,"time":time,"revision":revision})
}
struct Fixture {
    vm: Rc<PhoneViewModel>,
    registry: Rc<AppRegistry>,
    state: AlarmState,
    requests: Rc<RefCell<Vec<HostRequest>>>,
    _owner: Owner,
    now: i64,
}
impl Fixture {
    fn new() -> Self {
        let vm = PhoneViewModel::new(PhoneData::default(), Rc::new(|_| {}));
        let now = phone_domain::days_from_date(2026, 11, 26) * 86_400 + 9 * 3600;
        vm.set_clock(now);
        vm.set_utc_offset(0);
        vm.set_environment(HostEnvironment {
            notification_permission: HostPermission::Granted,
            ..Default::default()
        });
        let requests = Rc::new(RefCell::new(Vec::new()));
        let record = requests.clone();
        vm.set_services(HostServices {
            request: Rc::new(move |request| record.borrow_mut().push(request)),
        });
        let registry = Rc::new(
            AppRegistry::new(vec![AppRegistration::new(
                AppManifest::new(APP, "Tasks", phone_controls::Icon::Calendar)
                    .capabilities(&[Capability::Notifications]),
                |_| Element::Empty,
            )
            .with_reminders(provider)])
            .unwrap(),
        );
        registry.bind(&vm);
        let owner = Owner::new();
        let state = owner.with(AlarmState::new);
        Self {
            vm,
            registry,
            state,
            requests,
            _owner: owner,
            now,
        }
    }
    fn put(&self, items: Vec<serde_json::Value>) {
        self.vm
            .set_app_data(APP, Some(serde_json::Value::Array(items)))
            .unwrap();
    }
    fn tick(&self, time: i64) {
        self.vm.set_clock(time);
        self.state.tick_with_registry(&self.vm, &self.registry);
    }
    fn active(&self) -> Option<phone_sdk::ScopedAppReminder> {
        match self.state.active.get_untracked() {
            Some(Alert::App(item)) => Some(item),
            None => None,
            _ => panic!("unexpected core alarm"),
        }
    }
    fn notices(&self) -> usize {
        self.requests
            .borrow()
            .iter()
            .filter(|request| matches!(request, HostRequest::Notify { .. }))
            .count()
    }
}

#[test]
fn app_reminders_queue_outside_the_active_app_refresh_edits_and_cancel_disabled_snoozes() {
    let f = Fixture::new();
    assert_eq!(f.vm.route(), Route::Start);
    f.put(vec![
        item("b", "Second", f.now, 1),
        item("a", "First", f.now, 1),
    ]);
    f.tick(f.now);
    assert_eq!(f.active().unwrap().reminder.key, "a");
    assert_eq!(f.state.queue.borrow().len(), 1);
    assert_eq!(f.notices(), 1);
    f.put(vec![
        item("b", "Second", f.now, 1),
        item("a", "Edited first", f.now, 2),
    ]);
    f.tick(f.now + 1);
    assert_eq!(f.active().unwrap().reminder.title, "Edited first");
    assert_eq!(f.notices(), 1);
    f.put(vec![item("b", "Second", f.now, 1)]);
    f.tick(f.now + 2);
    assert_eq!(f.active().unwrap().reminder.key, "b");
    assert_eq!(f.notices(), 2);
    f.state.snooze(&f.vm, 5);
    assert!(f.active().is_none());
    f.vm.set_extension_enabled(APP, false).unwrap();
    f.tick(f.now + 3);
    f.vm.set_extension_enabled(APP, true).unwrap();
    f.tick(f.now + 4);
    assert!(
        f.active().is_none(),
        "rapid disable/enable cannot replay the fired current minute"
    );
    f.tick(f.now + 302);
    assert!(f.active().is_none(), "disabling removed the old snooze");
    assert_eq!(f.notices(), 2);
}

#[test]
fn epoch_replacement_clears_active_queued_and_snoozed_tasks_and_suspension_defers_reconciliation() {
    let f = Fixture::new();
    f.put(vec![
        item("a", "First", f.now, 1),
        item("b", "Second", f.now, 1),
        item("c", "Third", f.now, 1),
    ]);
    f.tick(f.now);
    f.state.snooze(&f.vm, 1);
    assert_eq!(f.active().unwrap().reminder.key, "b");
    assert_eq!(f.state.queue.borrow().len(), 1);
    f.state.host_visibility(&f.vm, false);
    let before = f.requests.borrow().len();
    let replacement = PhoneData::default();
    f.vm.replace_data(replacement).unwrap();
    f.tick(f.now + 60);
    assert_eq!(
        f.requests.borrow().len(),
        before,
        "hidden service cannot deliver stale queued/snoozed tasks"
    );
    f.state.host_visibility(&f.vm, true);
    f.tick(f.now + 60);
    assert!(f.active().is_none());
    assert!(f.state.queue.borrow().is_empty());
    f.tick(f.now + 120);
    assert!(f.active().is_none());
    f.put(vec![item("a", "New document task", f.now + 180, 1)]);
    f.tick(f.now + 180);
    assert_eq!(f.active().unwrap().reminder.title, "New document task");
    f.state.host_closing(&f.vm);
    let before = f.requests.borrow().len();
    f.put(vec![item("a", "After close", f.now + 240, 2)]);
    f.state.host_visibility(&f.vm, true);
    f.tick(f.now + 240);
    assert!(f.active().is_none());
    assert_eq!(f.requests.borrow().len(), before);
}
