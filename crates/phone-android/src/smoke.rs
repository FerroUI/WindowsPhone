//! Debug-only observation of the real activity, mounted tree and activation path.
//! A private nonce file opts one fresh test process in; this module never navigates.
use ferroui_base::{
    threading::{DispatcherPriority, DispatcherTimer},
    Point, Ref,
};
use ferroui_controls::{Control, TopLevel};
use phone_native::NativeHost;
use serde_json::{json, Value};
use std::{
    cell::{Cell, RefCell},
    io::Read,
    path::PathBuf,
    rc::Rc,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

struct Observer {
    directory: PathBuf,
    nonce: String,
    sequence: Cell<u64>,
    activation: RefCell<Option<Value>>,
}

thread_local! {
    static OBSERVER: RefCell<Option<Rc<Observer>>> = const { RefCell::new(None) };
}

/// Records only the outcome returned by the application's normal URI handler.
pub fn activation(uri: &str, result: &Result<(), String>) {
    OBSERVER.with(|slot| {
        if let Some(observer) = slot.borrow().as_ref() {
            *observer.activation.borrow_mut() = Some(json!({
                "uri": uri.chars().take(4096).collect::<String>(),
                "outcome": result.as_ref().map(|_| "accepted").unwrap_or("rejected"),
                "error": result.as_ref().err().map(|error| error.chars().take(512).collect::<String>()),
            }));
        }
    });
}

fn rectangle(view: &Control, top: &TopLevel) -> Option<Value> {
    let bounds = view.bounds();
    let point = view.translate_point(Point::new(0.0, 0.0), top)?;
    Some(json!({"x": point.x, "y": point.y, "width": bounds.width, "height": bounds.height}))
}

impl Observer {
    fn write(&self, host: &NativeHost, wrapper: &Control) {
        let view = host.view();
        let Some(top) = TopLevel::get_top_level(Some(&view)) else {
            return;
        };
        let screen = view
            .get_visual_descendants()
            .filter_map(|visual| visual.cast::<Control>())
            .find(|control| control.name().as_deref() == Some("PhoneScreen"));
        let controls = view
            .get_visual_descendants()
            .filter(|visual| {
                let bounds = visual.bounds();
                visual.is_effectively_visible() && bounds.width > 0.0 && bounds.height > 0.0
            })
            .filter_map(|visual| visual.name())
            .filter(|name| {
                matches!(
                    name.as_str(),
                    "PhoneScreen" | "PhonePage" | "StartTiles" | "StartScroll" | "WalletSearch"
                )
            })
            .take(16)
            .collect::<Vec<_>>();
        let client = top.client_size();
        let viewport = host.vm().viewport();
        let sequence = self.sequence.get().saturating_add(1);
        self.sequence.set(sequence);
        let proof = json!({
            "schema": 1, "platform": "android", "nonce": self.nonce,
            "sequence": sequence, "process_id": std::process::id(),
            "observed_at_unix_ms": SystemTime::now().duration_since(UNIX_EPOCH)
                .map(|elapsed| elapsed.as_millis()).unwrap_or_default(),
            "ready": view.is_attached_to_visual_tree() && wrapper.is_attached_to_visual_tree()
                && view.is_measure_valid() && view.is_arrange_valid(),
            "closed": host.runtime().is_closed(), "route": host.vm().route(),
            "status": host.vm().status(), "host": host.vm().platform_state(),
            "activation": self.activation.borrow().clone(),
            "client": {"width": client.width, "height": client.height},
            "render_scaling": top.render_scaling(),
            "safe_area": top.insets_manager().map(|manager| {
                let padding = manager.safe_area_padding();
                json!({"left": padding.left, "top": padding.top, "right": padding.right, "bottom": padding.bottom})
            }),
            "wrapper": rectangle(wrapper, &top), "inner": rectangle(&view, &top),
            "screen": screen.as_ref().and_then(|screen| rectangle(screen, &top)),
            "viewport": {"width": viewport.0, "height": viewport.1},
            "mounted_controls": controls,
        });
        let temporary = self.directory.join("phone-android-smoke.tmp");
        if std::fs::write(&temporary, proof.to_string()).is_ok() {
            let _ = std::fs::rename(temporary, self.directory.join("phone-android-smoke.json"));
        }
    }
}

pub fn install(host: &Rc<NativeHost>, wrapper: &Ref<Control>) {
    let Ok(path) = crate::bridge::data_path() else {
        return;
    };
    let Some(directory) = path.parent() else {
        return;
    };
    let marker = directory.join("phone-android-smoke.enabled");
    let Ok(file) = std::fs::File::open(&marker) else {
        return;
    };
    let mut nonce = String::new();
    if file.take(65).read_to_string(&mut nonce).is_err() {
        return;
    }
    let nonce = nonce.trim();
    if nonce.len() != 32 || !nonce.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return;
    }
    let observer = Rc::new(Observer {
        directory: directory.to_path_buf(),
        nonce: nonce.into(),
        sequence: Cell::new(0),
        activation: RefCell::new(None),
    });
    OBSERVER.with(|slot| *slot.borrow_mut() = Some(observer.clone()));
    let weak = Rc::downgrade(host);
    let wrapper = wrapper.downgrade();
    let deadline = Instant::now() + Duration::from_secs(600);
    DispatcherTimer::run(
        move || {
            let host = weak.upgrade().filter(|host| !host.runtime().is_closed());
            let wrapper = wrapper.upgrade();
            if Instant::now() >= deadline
                || !marker.is_file()
                || host.is_none()
                || wrapper.is_none()
            {
                OBSERVER.with(|slot| slot.borrow_mut().take());
                return false;
            }
            if let (Some(host), Some(wrapper)) = (host, wrapper) {
                observer.write(&host, &wrapper);
            }
            true
        },
        Duration::from_millis(500),
        DispatcherPriority::BACKGROUND,
    )
    .detach();
}
