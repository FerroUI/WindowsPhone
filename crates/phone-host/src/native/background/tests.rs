use super::*;
use std::{
    sync::mpsc,
    task::Wake,
    time::{Duration, Instant},
};

struct ThreadWake(std::thread::Thread);
impl Wake for ThreadWake {
    fn wake(self: Arc<Self>) {
        self.0.unpark();
    }
    fn wake_by_ref(self: &Arc<Self>) {
        self.0.unpark();
    }
}

struct ObservedWake(std::thread::Thread, Arc<AtomicBool>);
impl Wake for ObservedWake {
    fn wake(self: Arc<Self>) {
        self.1.store(true, Ordering::Release);
        self.0.unpark();
    }
}

fn finish<T>(mut task: Task<T>) -> Result<T, String> {
    let waker = Waker::from(Arc::new(ThreadWake(std::thread::current())));
    let mut context = Context::from_waker(&waker);
    let deadline = Instant::now() + Duration::from_secs(3);
    loop {
        if let Poll::Ready(result) = Pin::new(&mut task).poll(&mut context) {
            return result;
        }
        assert!(
            Instant::now() < deadline,
            "Native worker did not wake its awaiting task."
        );
        std::thread::park_timeout(Duration::from_millis(20));
    }
}

#[test]
fn worker_wakes_pending_future_and_completes_on_another_thread() {
    let caller = std::thread::current().id();
    let (release, wait) = mpsc::channel();
    let task = run(move || {
        wait.recv().unwrap();
        Ok(std::thread::current().id())
    });
    let woke = Arc::new(AtomicBool::new(false));
    let waker = Waker::from(Arc::new(ObservedWake(std::thread::current(), woke.clone())));
    let mut context = Context::from_waker(&waker);
    let mut task = task;
    assert!(matches!(
        Pin::new(&mut task).poll(&mut context),
        Poll::Pending
    ));
    release.send(()).unwrap();
    let deadline = Instant::now() + Duration::from_secs(3);
    while !woke.load(Ordering::Acquire) {
        assert!(
            Instant::now() < deadline,
            "The registered waker was not signalled."
        );
        std::thread::park_timeout(Duration::from_millis(20));
    }
    assert_ne!(finish(task).unwrap(), caller);
}

#[test]
fn dropping_queued_task_cancels_work_without_poisoning_worker() {
    let (started, running) = mpsc::channel();
    let (release, wait) = mpsc::channel();
    let blocker = run(move || {
        started.send(()).unwrap();
        wait.recv().unwrap();
        Ok(())
    });
    running.recv_timeout(Duration::from_secs(3)).unwrap();
    let called = Arc::new(AtomicBool::new(false));
    let observed = called.clone();
    drop(run(move || {
        observed.store(true, Ordering::Release);
        Ok(())
    }));
    release.send(()).unwrap();
    finish(blocker).unwrap();
    finish(run(|| Ok(()))).unwrap();
    assert!(!called.load(Ordering::Acquire));
}
