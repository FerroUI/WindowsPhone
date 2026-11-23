use super::*;
use crate::{AppManifest, AppRegistration, AppRegistry, Capability};
use ferroui_view::prelude::Element;
use phone_controls::Icon;
use phone_state::HostServices;
fn fixture(network: bool) -> (AppContext, Rc<RefCell<Vec<HostRequest>>>) {
    let app = AppRegistration::new(
        AppManifest::new("test.network", "Network", Icon::Explorer).capabilities(if network {
            &[Capability::Network]
        } else {
            &[]
        }),
        |_| Element::Empty,
    );
    let registry = Rc::new(AppRegistry::new(vec![app]).unwrap());
    let vm = PhoneViewModel::new(phone_domain::PhoneData::default(), Rc::new(|_| {}));
    vm.set_registered_apps(["test.network".into()]);
    let calls = Rc::new(RefCell::new(Vec::new()));
    let recorded = calls.clone();
    vm.set_services(HostServices {
        request: Rc::new(move |request| recorded.borrow_mut().push(request)),
    });
    (
        AppContext {
            session: Rc::new(Session::new(0, registry)),
            vm,
            id: "test.network",
        },
        calls,
    )
}
fn poll(task: &mut HttpTask) -> Poll<HttpResult> {
    Pin::new(task).poll(&mut Context::from_waker(Waker::noop()))
}
fn id(calls: &RefCell<Vec<HostRequest>>) -> u32 {
    match calls.borrow().last().unwrap() {
        HostRequest::HttpGet { request_id, .. } => *request_id,
        _ => unreachable!(),
    }
}
#[test]
fn completion_is_bounded_once_only_and_capability_checked() {
    let (cx, calls) = fixture(false);
    let mut task = get(&cx, "https://example.org/feed", 32);
    assert!(matches!(poll(&mut task),Poll::Ready(Err(e)) if e.contains("Network")));
    assert!(calls.borrow().is_empty());
    let (cx, calls) = fixture(true);
    let mut task = get(&cx, "https://example.org/feed", 32);
    let request = id(&calls);
    assert!(poll(&mut task).is_pending());
    assert!(complete_http(request, Ok(b"hello".to_vec())));
    assert!(!complete_http(request, Ok(Vec::new())));
    assert_eq!(poll(&mut task), Poll::Ready(Ok(b"hello".to_vec())));
    let mut task = get(&cx, "https://example.org/feed", 2);
    complete_http(id(&calls), Ok(b"too big".to_vec()));
    assert!(matches!(poll(&mut task), Poll::Ready(Err(_))));
}
#[test]
fn restore_close_disable_and_drop_cannot_publish_stale_responses() {
    for ready in [false, true] {
        let (cx, calls) = fixture(true);
        let mut task = get(&cx, "https://example.org/feed", 32);
        let request = id(&calls);
        if ready {
            complete_http(request, Ok(Vec::new()));
        }
        let data = cx.vm.data().clone();
        cx.vm.replace_data_after_save(data, |_| Ok(())).unwrap();
        complete_http(request, Ok(Vec::new()));
        assert!(matches!(poll(&mut task),Poll::Ready(Err(e)) if e.contains("data changed")));
    }
    let (cx, calls) = fixture(true);
    let task = get(&cx, "https://example.org/feed", 32);
    let request = id(&calls);
    drop(task);
    assert!(!complete_http(request, Ok(Vec::new())));
    let mut task = get(&cx, "https://example.org/feed", 32);
    complete_http(id(&calls), Ok(Vec::new()));
    cancel(&cx.session, Some(cx.id));
    assert!(matches!(poll(&mut task), Poll::Ready(Err(_))));
    let mut task = get(&cx, "https://example.org/feed", 32);
    cx.vm.set_extension_enabled(cx.id, false).unwrap();
    assert!(matches!(poll(&mut task), Poll::Ready(Err(_))));
}
#[test]
fn live_requests_have_a_strict_concurrency_limit() {
    let (cx, _) = fixture(true);
    let held = (0..MAX_PENDING_HTTP_TASKS)
        .map(|_| get(&cx, "https://example.org/feed", 32))
        .collect::<Vec<_>>();
    let mut overflow = get(&cx, "https://example.org/feed", 32);
    assert!(matches!(poll(&mut overflow), Poll::Ready(Err(_))));
    drop(held);
    assert!(poll(&mut get(&cx, "https://example.org/feed", 32)).is_pending());
}

#[test]
fn viewed_map_requests_share_network_admission_bounds_and_cancellation() {
    let (denied, calls) = fixture(false);
    assert!(matches!(
        poll(&mut map_tile(&denied, 1, 0, 0)),
        Poll::Ready(Err(_))
    ));
    assert!(calls.borrow().is_empty());
    let (cx, calls) = fixture(true);
    assert!(matches!(
        poll(&mut map_tile(&cx, 255, 0, 0)),
        Poll::Ready(Err(_))
    ));
    assert!(calls.borrow().is_empty());
    let mut task = map_tile(&cx, 4, 7, 8);
    let id = match calls.borrow().last().unwrap() {
        HostRequest::MapTile {
            request_id,
            zoom: 4,
            x: 7,
            y: 8,
        } => *request_id,
        _ => panic!("Unexpected tile request"),
    };
    assert!(is_http_task_pending(id));
    cancel(&cx.session, Some(cx.id));
    assert!(!is_http_task_pending(id));
    assert!(!complete_http(id, Ok(vec![1])));
    assert!(matches!(poll(&mut task), Poll::Ready(Err(_))));
}
