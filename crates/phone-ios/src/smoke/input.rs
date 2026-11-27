//! Passive, bounded input receipts for the explicitly enabled simulator smoke.
//! These listeners never handle input, capture pointers, focus controls or run
//! commands. The normal observation timer writes their in-memory snapshot.
use ferroui_base::{
    Ref, WeakRef,
    input::{InputElement, InputManager, PointerEventArgs, raw::RawPointerEventArgs},
    interactivity::{RoutedEventArgs, RoutingStrategies},
    reactive::{Disposable, ObservableExt, Subscription},
};
use ferroui_controls::{Button, Control, TopLevel};
use serde_json::{Value, json};
use std::{
    cell::{Cell, RefCell},
    collections::VecDeque,
    rc::{Rc, Weak},
    time::{SystemTime, UNIX_EPOCH},
};

const CAPACITY: usize = 96;

thread_local! {
    static ACTIVE: RefCell<Weak<Receipts>> = const { RefCell::new(Weak::new()) };
}

pub(super) struct Receipts {
    view: WeakRef<Control>,
    route: Box<dyn Fn() -> Value>,
    next: Cell<u64>,
    events: RefCell<VecDeque<Value>>,
    subscriptions: RefCell<Vec<Subscription>>,
}

pub(super) fn install(view: &Ref<Control>, route: impl Fn() -> Value + 'static) -> Rc<Receipts> {
    let receipts = Rc::new(Receipts {
        view: view.downgrade(),
        route: Box::new(route),
        next: Cell::new(0),
        events: RefCell::new(VecDeque::with_capacity(CAPACITY)),
        subscriptions: RefCell::new(Vec::new()),
    });
    ACTIVE.with(|active| *active.borrow_mut() = Rc::downgrade(&receipts));
    let mut subscriptions = receipts.subscriptions.borrow_mut();
    if let Some(manager) = InputManager::instance() {
        for (stage, observable) in [
            ("raw_pre", manager.pre_process()),
            ("raw_post", manager.post_process()),
        ] {
            let weak = Rc::downgrade(&receipts);
            subscriptions.push(observable.subscribe_fn_guard(move |event| {
                let (Some(receipts), Some(pointer)) = (weak.upgrade(), event.downcast_ref::<RawPointerEventArgs>()) else {
                    return;
                };
                use ferroui_base::input::raw::RawPointerEventType as Kind;
                if !matches!(pointer.type_(), Kind::TouchBegin | Kind::TouchEnd | Kind::TouchCancel
                    | Kind::LeftButtonDown | Kind::LeftButtonUp | Kind::CancelCapture) {
                    return;
                }
                let Some(top) = receipts.top() else { return };
                if !event.root().try_root_element().is_some_and(|root| root.ptr_eq(&top)) {
                    return;
                }
                let point = pointer.position();
                let (hit, enabled_hit) = pointer.input_hit_test_result();
                receipts.record(json!({
                    "stage": stage, "kind": format!("{:?}", pointer.type_()),
                    "pointer_id": pointer.raw_pointer_id(), "input_timestamp_ms": pointer.timestamp(),
                    "position": {"x": point.x, "y": point.y}, "handled": pointer.handled(),
                    "hit": describe(hit.and_then(|control| control.cast::<Control>())),
                    "enabled_hit": describe(enabled_hit.and_then(|control| control.cast::<Control>())),
                }));
            }));
        }
    }
    // Root tunnel and bubble receipts also observe already-handled events.
    // Keeping native handler leases avoids an observer surviving its host.
    macro_rules! pointer {
        ($event:expr, $kind:literal) => {{
            let event = $event;
            let weak = Rc::downgrade(&receipts);
            let token = view.add_handler_with(
                event,
                move |_, args| {
                    if let Some(receipts) = weak.upgrade() {
                        receipts.pointer($kind, args);
                    }
                },
                RoutingStrategies::TUNNEL | RoutingStrategies::BUBBLE,
                true,
            );
            let weak_view = view.downgrade();
            subscriptions.push(Subscription::new(Disposable::create(move || {
                if let Some(view) = weak_view.upgrade() {
                    view.remove_handler(event, token);
                }
            })));
        }};
    }
    pointer!(InputElement::pointer_pressed_event(), "pressed");
    pointer!(InputElement::pointer_released_event(), "released");
    let weak = Rc::downgrade(&receipts);
    let event = Button::click_event();
    let token = view.add_handler_with(
        event,
        move |_, args| {
            if let Some(receipts) = weak.upgrade() {
                receipts.routed("button_click", args);
            }
        },
        RoutingStrategies::BUBBLE,
        true,
    );
    let weak_view = view.downgrade();
    subscriptions.push(Subscription::new(Disposable::create(move || {
        if let Some(view) = weak_view.upgrade() {
            view.remove_handler(event, token);
        }
    })));
    // Capture loss is direct, so observe its original source within this host
    // rather than relying on it to bubble through the view.
    let weak = Rc::downgrade(&receipts);
    subscriptions.push(
        InputElement::pointer_capture_lost_event()
            .route_finished()
            .subscribe(move |args| {
                if let Some(receipts) = weak.upgrade() {
                    receipts.routed("capture_lost", args);
                }
            }),
    );
    for (kind, event) in [
        ("got_focus", InputElement::got_focus_event()),
        ("lost_focus", InputElement::lost_focus_event()),
    ] {
        let weak = Rc::downgrade(&receipts);
        subscriptions.push(event.route_finished().subscribe(move |args| {
            if let Some(receipts) = weak.upgrade() {
                receipts.routed(kind, args);
            }
        }));
    }
    drop(subscriptions);
    receipts
}

fn describe(control: Option<Ref<Control>>) -> Value {
    control.map_or(Value::Null, |control| {
        json!({
            "name": control.name(), "instance": format!("{:p}", &*control),
            "attached": control.is_attached_to_visual_tree(),
            "enabled": control.is_effectively_enabled(), "focused": control.is_focused(),
            "pressed": control.cast::<Button>().map(|button| button.is_pressed()),
            "named_ancestors": control.get_self_and_visual_ancestors()
                .filter_map(|visual| visual.name()).take(6).collect::<Vec<_>>(),
        })
    })
}

impl Receipts {
    fn top(&self) -> Option<Ref<TopLevel>> {
        let view = self.view.upgrade()?;
        TopLevel::get_top_level(Some(&view))
    }

    fn record(&self, mut value: Value) {
        let sequence = self.next.get().saturating_add(1);
        self.next.set(sequence);
        value["sequence"] = sequence.into();
        value["observed_at_unix_ms"] = json!(
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|elapsed| elapsed.as_millis())
                .unwrap_or_default()
        );
        value["route"] = (self.route)();
        value["focused"] = describe(
            self.top()
                .and_then(|top| top.focus_manager().get_focused_element())
                .and_then(|control| control.cast::<Control>()),
        );
        let mut events = self.events.borrow_mut();
        if events.len() == CAPACITY {
            events.pop_front();
        }
        events.push_back(value);
    }

    fn pointer(&self, kind: &str, args: &PointerEventArgs) {
        let point = args.get_position(None);
        let source = args.source().and_then(|source| source.cast::<Control>());
        let button = source
            .as_ref()
            .and_then(|source| {
                source
                    .get_self_and_visual_ancestors()
                    .find_map(|visual| visual.cast::<Button>())
            })
            .map(|button| button.upcast::<Control>());
        self.record(json!({
            "stage": if args.route() == RoutingStrategies::TUNNEL { "root_tunnel" } else { "root_bubble" },
            "kind": kind, "handled": args.handled(),
            "pointer_id": args.pointer().id(), "pointer_type": format!("{:?}", args.pointer().type_()),
            "input_timestamp_ms": args.timestamp(), "position": {"x": point.x, "y": point.y},
            "source": describe(source), "button": describe(button),
            "captured": describe(args.pointer().captured().and_then(|control| control.cast::<Control>())),
        }));
    }

    fn routed(&self, kind: &str, args: &RoutedEventArgs) {
        let source = args.source().and_then(|source| source.cast::<Control>());
        let Some(view) = self.view.upgrade() else {
            return;
        };
        if !source
            .as_ref()
            .is_some_and(|source| source.ptr_eq(&view) || view.is_visual_ancestor_of(source))
        {
            return;
        }
        self.record(json!({
            "stage": "routed", "kind": kind, "handled": args.handled(),
            "routing": format!("{:?}", args.route()), "source": describe(source),
        }));
    }
}

pub(super) fn snapshot() -> Value {
    ACTIVE
        .with(|active| active.borrow().upgrade())
        .map_or(Value::Null, |receipts| {
            json!({
                "schema": 1, "capacity": CAPACITY, "total_events": receipts.next.get(),
                "dropped_events": receipts.next.get().saturating_sub(CAPACITY as u64),
                "events": *receipts.events.borrow(),
            })
        })
}
