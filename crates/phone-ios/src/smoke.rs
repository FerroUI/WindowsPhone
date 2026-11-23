//! Explicit simulator-only observation hook; production launches create no timer.
use ferroui_base::threading::{DispatcherPriority, DispatcherTimer};
use ferroui_controls::TopLevel;
use phone_native::NativeHost;
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

thread_local! {
    static ACTIVATION: RefCell<Option<serde_json::Value>> = const { RefCell::new(None) };
    static SEQUENCE: Cell<u64> = const { Cell::new(0) };
}

fn enabled() -> bool {
    std::env::var("PHONE_IOS_SMOKE").as_deref() == Ok("1")
}

/// Observe the real delegate path without injecting an activation or changing it.
pub fn activation(host: Option<&Rc<NativeHost>>, uri: &str, outcome: &str) {
    if !enabled() {
        return;
    }
    ACTIVATION.with(|value| {
        *value.borrow_mut() = Some(serde_json::json!({"uri": uri, "outcome": outcome}));
    });
    eprintln!("[ios-smoke] Activation {outcome}: {uri}");
    if let Some(host) = host {
        write_status(host);
    }
}

fn write_status(host: &NativeHost) {
    let view = host.view();
    let bounds = view.bounds();
    let attached = TopLevel::get_top_level(Some(&view)).is_some();
    let sequence = SEQUENCE.with(|value| {
        let next = value.get().saturating_add(1);
        value.set(next);
        next
    });
    let proof = serde_json::json!({
        "ready": attached && bounds.width > 0.0 && bounds.height > 0.0,
        "width": bounds.width, "height": bounds.height,
        "platform": "ios", "route": host.vm().route(),
        "status": host.vm().status(), "sequence": sequence,
        "observed_at_unix_ms": SystemTime::now().duration_since(UNIX_EPOCH)
            .map(|elapsed| elapsed.as_millis()).unwrap_or_default(),
        "closed": host.runtime().is_closed(),
        "activation": ACTIVATION.with(|value| value.borrow().clone()),
    });
    // HOME is supplied by iOS for its own sandbox; it is never reassigned.
    if let Some(directory) = std::env::var_os("HOME").map(std::path::PathBuf::from) {
        let directory = directory.join("Documents");
        if std::fs::create_dir_all(&directory).is_ok() {
            let temporary = directory.join("phone-ios-smoke.tmp");
            if std::fs::write(&temporary, proof.to_string()).is_ok() {
                let _ = std::fs::rename(temporary, directory.join("phone-ios-smoke.json"));
            }
        }
    }
}

pub fn install(host: &Rc<NativeHost>) {
    if !enabled() {
        return;
    }
    let weak = Rc::downgrade(host);
    DispatcherTimer::run(
        move || {
            let Some(host) = weak.upgrade() else {
                return false;
            };
            write_status(&host);
            true
        },
        Duration::from_millis(500),
        DispatcherPriority::BACKGROUND,
    )
    .detach();
}
