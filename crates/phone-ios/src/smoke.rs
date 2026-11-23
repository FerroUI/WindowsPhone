//! Explicit simulator-only observation hook; production launches create no timer.
use ferroui_base::{
    Point,
    threading::{DispatcherPriority, DispatcherTimer},
};
use ferroui_controls::{Control, TextBox, TopLevel};
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

fn native_metadata() -> Option<serde_json::Value> {
    unsafe extern "C" {
        fn wp_ios_smoke_metadata(buffer: *mut std::ffi::c_char, capacity: usize) -> usize;
    }
    let mut buffer = [0_u8; 16 * 1024];
    // UIKit copies bounded JSON on its own thread; no native pointer escapes.
    let length = unsafe { wp_ios_smoke_metadata(buffer.as_mut_ptr().cast(), buffer.len()) };
    (length > 0 && length < buffer.len())
        .then(|| serde_json::from_slice(&buffer[..length]).ok())
        .flatten()
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
    let Some(top) = TopLevel::get_top_level(Some(&view)) else {
        return;
    };
    let wrapper = top.content().and_then(|value| Control::from_boxed(&value));
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
                    | "DevicePhoneNumber"
                    | "DevicePeopleSearch"
                    | "PeopleRoot"
                    | "NativeMessageSearch"
                    | "DeviceMessageRecipients"
                    | "DeviceMessageBody"
                    | "DeviceMessageSubject"
                    | "DeviceSettingsRoot"
                    | "DeviceCommunicationsSettings"
            )
        })
        .take(16)
        .collect::<Vec<_>>();
    let fields = view
        .get_visual_descendants()
        .filter_map(|visual| visual.cast::<TextBox>())
        .filter_map(|control| {
            let name = control.name()?;
            matches!(
                name.as_str(),
                "DevicePhoneNumber" | "DeviceMessageRecipients" | "DeviceMessageBody"
            )
            .then(|| (name, control.text().unwrap_or_default()))
        })
        .collect::<std::collections::BTreeMap<_, _>>();
    let rectangle = |control: &Control| {
        let bounds = control.bounds();
        control.translate_point(Point::new(0.0, 0.0), &top).map(|point| {
            serde_json::json!({"x": point.x, "y": point.y, "width": bounds.width, "height": bounds.height})
        })
    };
    let bounds = view.bounds();
    let client = top.client_size();
    let viewport = host.vm().viewport();
    let sequence = SEQUENCE.with(|value| {
        let next = value.get().saturating_add(1);
        value.set(next);
        next
    });
    let proof = serde_json::json!({
        "schema": 1,
        "ready": view.is_attached_to_visual_tree() && view.is_measure_valid() && view.is_arrange_valid()
            && wrapper.as_ref().is_some_and(|control| control.is_attached_to_visual_tree()),
        "width": bounds.width, "height": bounds.height,
        "platform": "ios", "route": host.vm().route(),
        "native": native_metadata(),
        "status": host.vm().status(), "sequence": sequence,
        "observed_at_unix_ms": SystemTime::now().duration_since(UNIX_EPOCH)
            .map(|elapsed| elapsed.as_millis()).unwrap_or_default(),
        "closed": host.runtime().is_closed(),
        "activation": ACTIVATION.with(|value| value.borrow().clone()),
        "client": {"width": client.width, "height": client.height},
        "render_scaling": top.render_scaling(),
        "safe_area": top.insets_manager().map(|manager| {
            let padding = manager.safe_area_padding();
            serde_json::json!({"left": padding.left, "top": padding.top, "right": padding.right, "bottom": padding.bottom})
        }),
        "wrapper": wrapper.as_ref().and_then(|control| rectangle(control)),
        "inner": rectangle(&view),
        "screen": screen.as_ref().and_then(|control| rectangle(control)),
        "viewport": {"width": viewport.0, "height": viewport.1},
        "mounted_controls": controls,
        "device_fields": fields,
        "device": host.vm().device_state().snapshot,
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
    crate::smoke_devices::install(host);
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
