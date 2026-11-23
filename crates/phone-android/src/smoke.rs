//! Debug-only observation of the real activity, mounted tree and activation path.
//! A private nonce file opts one fresh test process in; this module never navigates.
use ferroui_base::{
    Point, Ref,
    threading::{DispatcherPriority, DispatcherTimer},
};
use ferroui_controls::{Control, TextBlock, TopLevel};
use phone_native::NativeHost;
use serde_json::{Value, json};
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

fn visible_rectangle(view: &Control, top: &TopLevel) -> Option<Value> {
    let bounds = view.bounds();
    if !view.is_effectively_visible() || bounds.width <= 0.0 || bounds.height <= 0.0 {
        return None;
    }
    let point = view.translate_point(Point::new(0.0, 0.0), top)?;
    let client = top.client_size();
    let (mut left, mut upper, mut right, mut bottom) = (
        point.x.max(0.0),
        point.y.max(0.0),
        (point.x + bounds.width).min(client.width),
        (point.y + bounds.height).min(client.height),
    );
    for ancestor in view
        .get_visual_ancestors()
        .filter(|visual| visual.clip_to_bounds())
    {
        let origin = ancestor.translate_point(Point::new(0.0, 0.0), top)?;
        let clip = ancestor.bounds();
        left = left.max(origin.x);
        upper = upper.max(origin.y);
        right = right.min(origin.x + clip.width);
        bottom = bottom.min(origin.y + clip.height);
    }
    if right <= left || bottom <= upper {
        return None;
    }
    Some(
        json!({"x": point.x, "y": point.y, "width": bounds.width, "height": bounds.height,
        "fully_visible": (left - point.x).abs() < 0.5 && (upper - point.y).abs() < 0.5
            && (right - point.x - bounds.width).abs() < 0.5 && (bottom - point.y - bounds.height).abs() < 0.5}),
    )
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
                    "PhoneScreen"
                        | "PhonePage"
                        | "StartTiles"
                        | "StartScroll"
                        | "WalletSearch"
                        | "NativeMessageSearch"
                        | "DeviceMessageRecipients"
                        | "DeviceMessageBody"
                )
            })
            .take(16)
            .collect::<Vec<_>>();
        let vm = host.vm();
        let device = vm.device_state();
        let messaging_tiles = {
            let data = vm.data();
            let mut pending = data.tiles.iter().collect::<Vec<_>>();
            let mut ids = Vec::new();
            while let Some(tile) = pending.pop() {
                pending.extend(tile.folder_tiles.iter());
                if !tile.is_folder()
                    && tile.target.is_none()
                    && serde_json::to_value(tile.app).ok().as_ref() == Some(&json!("Messaging"))
                {
                    ids.push(tile.id);
                }
            }
            ids
        };
        let messaging_badges = view
            .get_visual_descendants()
            .filter_map(|visual| {
                let label = visual.cast::<TextBlock>()?;
                let name = label.name()?;
                let id = name.strip_prefix("TileBadge-")?.parse::<u64>().ok()?;
                if !messaging_tiles.contains(&id) {
                    return None;
                }
                Some(json!({"tile_id": id, "name": name, "text": label.text(),
                "bounds": visible_rectangle(&label, &top)?}))
            })
            .take(16)
            .collect::<Vec<_>>();
        let title = format!("FerroUI-bootstrap-{}", self.nonce);
        let body = format!("Native-listener-{}", self.nonce);
        let action_center = view
            .get_visual_descendants()
            .filter_map(|visual| visual.cast::<Control>())
            .find(|control| {
                control.name().as_deref() == Some("ActionCenter")
                    && visible_rectangle(control, &top).is_some()
            });
        let notification_labels = action_center
            .as_ref()
            .map(|center| {
                center
                    .get_visual_descendants()
                    .filter_map(|visual| {
                        let label = visual.cast::<TextBlock>()?;
                        let text = label.text()?;
                        if text != title && text != body {
                            return None;
                        }
                        Some(json!({"text": text, "bounds": visible_rectangle(&label, &top)?}))
                    })
                    .take(8)
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        let status_bar = view
            .get_visual_descendants()
            .filter_map(|visual| visual.cast::<Control>())
            .find(|control| control.name().as_deref() == Some("StatusBar"))
            .and_then(|control| visible_rectangle(&control, &top));
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
            "device_snapshot": device.snapshot,
            "device_message_thread": device.message_thread,
            "device_message_ids": device.messages.iter().map(|message| &message.id).collect::<Vec<_>>(),
            // Observe already-admitted state only: no DeviceRequest, provider query or UI mutation.
            "device_threads": device.threads.iter().take(200).map(|thread| json!({"id":thread.id,"unread":thread.unread})).collect::<Vec<_>>(),
            "device_message_unread": device.threads.iter().map(|thread| u64::from(thread.unread)).sum::<u64>(),
            "messaging_tile_ids": messaging_tiles, "messaging_badges": messaging_badges,
            "device_notification_count": device.notifications.len(),
            "device_notification_fixture": device.notifications.iter().filter(|notice| notice.title == title && notice.body == body).take(4).collect::<Vec<_>>(),
            "notification_fixture_labels": notification_labels,
            "action_center": action_center.as_ref().and_then(|center| visible_rectangle(center, &top)),
            "status_bar": status_bar,
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
