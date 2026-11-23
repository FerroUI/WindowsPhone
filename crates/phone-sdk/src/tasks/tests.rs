use super::*;
use crate::{session::Session, AppContext, AppManifest, AppRegistration, AppRegistry, Capability};
use ferroui_view::prelude::Element;
use phone_controls::Icon;
use phone_domain::PhoneData;
use phone_state::{HostRequest, HostServices, PhoneViewModel};
use std::{
    cell::RefCell,
    future::Future,
    pin::Pin,
    rc::Rc,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
    task::{Context, Poll, Wake, Waker},
};

fn fixture(files: bool) -> (AppContext, Rc<RefCell<Vec<HostRequest>>>) {
    let capabilities: &'static [Capability] = if files { &[Capability::Files] } else { &[] };
    let app = AppRegistration::new(
        AppManifest::new("tests.picker", "Picker", Icon::Folder).capabilities(capabilities),
        |_| Element::Empty,
    );
    let registry = Rc::new(AppRegistry::new(vec![app]).unwrap());
    let vm = PhoneViewModel::new(PhoneData::default(), Rc::new(|_| {}));
    vm.set_registered_apps(["tests.picker".into()]);
    let calls = Rc::new(RefCell::new(Vec::new()));
    let recorded = calls.clone();
    vm.set_services(HostServices {
        request: Rc::new(move |request| recorded.borrow_mut().push(request)),
    });
    let session = Rc::new(Session::new(0, registry));
    (
        AppContext {
            vm,
            session,
            id: "tests.picker",
        },
        calls,
    )
}
fn options() -> FilePickerOptions {
    FilePickerOptions {
        extensions: vec![".txt".into()],
        max_bytes: 32,
    }
}
fn selected() -> PickedFile {
    PickedFile {
        name: "note.txt".into(),
        mime: "text/plain".into(),
        bytes: b"hello".to_vec(),
    }
}
fn last_id(calls: &RefCell<Vec<HostRequest>>) -> u32 {
    match calls.borrow().last().unwrap() {
        HostRequest::ChooseFile { request_id, .. } => *request_id,
        _ => unreachable!(),
    }
}
fn poll(task: &mut FileTask) -> Poll<FileResult> {
    Pin::new(task).poll(&mut Context::from_waker(Waker::noop()))
}
struct WakeCount(AtomicUsize);
impl Wake for WakeCount {
    fn wake(self: Arc<Self>) {
        self.0.fetch_add(1, Ordering::SeqCst);
    }
}

#[test]
fn request_is_synchronous_and_completion_wakes_once_with_owned_bytes() {
    let (context, calls) = fixture(true);
    let mut task = choose_file(&context, options());
    assert_eq!(
        calls.borrow().len(),
        1,
        "request dispatch precedes the first future poll"
    );
    let id = last_id(&calls);
    let wake = Arc::new(WakeCount(AtomicUsize::new(0)));
    let waker = Waker::from(wake.clone());
    assert!(Pin::new(&mut task)
        .poll(&mut Context::from_waker(&waker))
        .is_pending());
    assert!(complete_file(id, Ok(Some(selected()))));
    assert_eq!(wake.0.load(Ordering::SeqCst), 1);
    assert_eq!(poll(&mut task), Poll::Ready(Ok(Some(selected()))));
    assert!(!complete_file(id, Ok(Some(selected()))));
    assert!(!is_file_task_pending(id));
}

#[test]
fn capability_and_option_errors_never_reach_the_host() {
    let (context, calls) = fixture(false);
    let mut task = choose_file(&context, options());
    assert!(matches!(poll(&mut task), Poll::Ready(Err(error)) if error.contains("Files")));
    assert!(calls.borrow().is_empty());
    let mut invalid = options();
    invalid.max_bytes = 0;
    let mut task = choose_file(&context, invalid);
    assert!(matches!(poll(&mut task), Poll::Ready(Err(_))));
    assert!(calls.borrow().is_empty());
}

#[test]
fn disabling_an_app_blocks_held_contexts_and_pending_or_ready_file_delivery() {
    for ready in [false, true] {
        let (context, calls) = fixture(true);
        let mut task = choose_file(&context, options());
        let id = last_id(&calls);
        if ready {
            assert!(complete_file(id, Ok(Some(selected()))));
        }
        context.vm.set_extension_enabled(context.id, false).unwrap();
        let mut blocked = context.choose_file(options());
        assert!(
            matches!(poll(&mut blocked), Poll::Ready(Err(error)) if error.contains("disabled"))
        );
        assert_eq!(calls.borrow().len(), 1);
        assert!(!is_file_task_pending(id));
        assert!(matches!(poll(&mut task), Poll::Ready(Err(error)) if error.contains("disabled")));
        context.vm.set_extension_enabled(context.id, true).unwrap();
        assert!(!complete_file(id, Ok(Some(selected()))));
    }
}

#[test]
fn user_cancel_resolves_none_but_dropped_and_obsolete_requests_never_deliver() {
    let (context, calls) = fixture(true);
    let mut cancelled = choose_file(&context, options());
    let id = last_id(&calls);
    assert!(complete_file(id, Ok(None)));
    assert_eq!(poll(&mut cancelled), Poll::Ready(Ok(None)));
    let dropped = choose_file(&context, options());
    let old = last_id(&calls);
    drop(dropped);
    assert!(!complete_file(old, Ok(Some(selected()))));
    let mut replacement = choose_file(&context, options());
    let next = last_id(&calls);
    assert!(next > old);
    let data = context.vm.data().clone();
    context
        .vm
        .replace_data_after_save(data, |_| Ok(()))
        .unwrap();
    assert!(complete_file(next, Ok(Some(selected()))));
    assert!(
        matches!(poll(&mut replacement), Poll::Ready(Err(error)) if error.contains("phone data changed"))
    );
}

#[test]
fn pending_queue_is_bounded_and_closing_cancels_the_owning_app() {
    let (context, calls) = fixture(true);
    let mut tasks: Vec<_> = (0..MAX_PENDING_FILE_TASKS)
        .map(|_| choose_file(&context, options()))
        .collect();
    let mut overflow = choose_file(&context, options());
    assert!(matches!(poll(&mut overflow), Poll::Ready(Err(error)) if error.contains("16 pending")));
    assert_eq!(calls.borrow().len(), MAX_PENDING_FILE_TASKS);
    cancel_for_app(&context.session, "tests.someone-else");
    assert!(poll(&mut tasks[0]).is_pending());
    cancel_for_app(&context.session, context.id);
    for task in &mut tasks {
        assert!(matches!(poll(task), Poll::Ready(Err(error)) if error.contains("app closed")));
    }
    let mut next = choose_file(&context, options());
    cancel_for_session(&context.session);
    assert!(matches!(poll(&mut next), Poll::Ready(Err(_))));
}

#[test]
fn malformed_host_results_fail_with_no_partial_application_delivery() {
    let (context, calls) = fixture(true);
    for file in [
        PickedFile {
            name: "../note.txt".into(),
            ..selected()
        },
        PickedFile {
            name: "note.exe".into(),
            ..selected()
        },
        PickedFile {
            mime: "text/plain\nunsafe".into(),
            ..selected()
        },
        PickedFile {
            bytes: vec![0; 33],
            ..selected()
        },
    ] {
        let mut task = choose_file(&context, options());
        assert!(complete_file(last_id(&calls), Ok(Some(file))));
        assert!(matches!(poll(&mut task), Poll::Ready(Err(_))));
    }
    assert!(context.vm.data().app_data.is_empty());
}

#[test]
fn already_ready_file_cannot_cross_restore_or_app_close_before_polling() {
    let (context, calls) = fixture(true);
    let mut task = choose_file(&context, options());
    let id = last_id(&calls);
    assert!(complete_file(id, Ok(Some(selected()))));
    assert!(!is_file_task_pending(id));
    assert!(!complete_file(id, Ok(Some(selected()))));
    let data = context.vm.data().clone();
    context
        .vm
        .replace_data_after_save(data, |_| Ok(()))
        .unwrap();
    assert!(
        matches!(poll(&mut task), Poll::Ready(Err(error)) if error.contains("phone data changed"))
    );

    let mut task = choose_file(&context, options());
    assert!(complete_file(last_id(&calls), Ok(Some(selected()))));
    cancel_for_app(&context.session, context.id);
    assert!(matches!(poll(&mut task), Poll::Ready(Err(error)) if error.contains("app closed")));
}
