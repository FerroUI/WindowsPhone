//! Explicit CI-only roundtrip against the real provider on our disposable iOS
//! simulator. Production launches and physical devices never execute this code.
use ferroui_base::threading::{Dispatcher, DispatcherPriority, DispatcherTimer};
use phone_host::platform::*;
use phone_native::NativeHost;
use serde_json::{Value, json};
use std::{
    rc::Rc,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

fn report(value: &Value) {
    let Some(directory) = std::env::var_os("HOME").map(std::path::PathBuf::from) else {
        return;
    };
    let directory = directory.join("Documents");
    if std::fs::create_dir_all(&directory).is_ok() {
        let temporary = directory.join("phone-ios-device-tests.tmp");
        if std::fs::write(&temporary, value.to_string()).is_ok() {
            let _ = std::fs::rename(temporary, directory.join("phone-ios-device-tests.json"));
        }
    }
}

async fn request(
    platform: &Rc<dyn PlatformPlugin>,
    request: DeviceRequest,
) -> Result<DeviceResponse, String> {
    let result = platform
        .dispatch(PlatformRequest::Device { request })
        .await?;
    let PlatformResponse::Device(value) = result else {
        return Err("Expected a typed native device response.".into());
    };
    value.validate()?;
    Ok(value)
}

async fn snapshot(platform: &Rc<dyn PlatformPlugin>) -> Result<DeviceSnapshot, String> {
    match request(platform, DeviceRequest::Snapshot).await? {
        DeviceResponse::Snapshot(snapshot) => Ok(snapshot),
        _ => Err("Expected the observed device snapshot.".into()),
    }
}

async fn contacts(
    platform: &Rc<dyn PlatformPlugin>,
    query: &str,
) -> Result<Vec<DeviceContact>, String> {
    match request(
        platform,
        DeviceRequest::Contacts {
            query: query.into(),
            limit: 20,
        },
    )
    .await?
    {
        DeviceResponse::Contacts(contacts) => Ok(contacts),
        _ => Err("Expected a native contacts result.".into()),
    }
}

fn brightness(snapshot: &DeviceSnapshot) -> Result<f64, String> {
    let control = snapshot
        .control(DeviceControl::Brightness)
        .ok_or("No observed brightness control.")?;
    if control.access != ControlAccess::Writable {
        return Err("The simulator did not expose writable UIKit brightness.".into());
    }
    match control.value {
        ControlValue::Level(value) if value.is_finite() => Ok(value),
        _ => Err("Brightness was not an observed level.".into()),
    }
}

async fn roundtrip(platform: Rc<dyn PlatformPlugin>) -> Value {
    let mut checks = Vec::<Value>::new();
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|time| time.as_nanos())
        .unwrap_or_default();
    let name = format!("FerroUISimulatorCheck{unique}");
    let mut created_id = None::<String>;
    let mut original_brightness = None;
    let result = async {
        let initial = snapshot(&platform).await?;
        if initial.permission(DevicePermission::Contacts) != PermissionState::Granted {
            return Err("The isolated simulator must grant Contacts before launch.".into());
        }
        if initial.role(DeviceRole::Dialer) != HostRole::Unsupported || initial.role(DeviceRole::Sms) != HostRole::Unsupported
            || initial.supports(DeviceOperation::CallControl) || initial.supports(DeviceOperation::SendMessages) {
            return Err("The unprovisioned simulator incorrectly advertised carrier roles or in-call control.".into());
        }
        if initial.control(DeviceControl::Flashlight).is_some_and(|state| state.access != ControlAccess::Unavailable) {
            return Err("The simulator incorrectly advertised a hardware torch.".into());
        }
        checks.push(json!({"name":"observed_simulator_capabilities","passed":true,"snapshot":initial}));
        let original = brightness(&initial)?;
        original_brightness = Some(original);
        let changed = if (original - 0.43).abs() < 0.1 { 0.67 } else { 0.43 };
        request(&platform, DeviceRequest::SetControl { control: DeviceControl::Brightness, value: ControlValue::Level(changed) }).await?;
        let observed = brightness(&snapshot(&platform).await?)?;
        if (observed - changed).abs() > 0.02 { return Err(format!("UIKit brightness readback was {observed}, expected {changed}.")); }
        checks.push(json!({"name":"brightness_write","passed":true,"before":original,"requested":changed,"observed":observed}));

        let contact = DeviceContact { id: String::new(), name: name.clone(), phones: vec!["+1 202 555 0147".into()], emails: vec!["ios-smoke@example.invalid".into()], favorite: false };
        request(&platform, DeviceRequest::SaveContact { contact }).await?;
        let mut found = contacts(&platform, &name).await?;
        if found.len() != 1 || found[0].name != name { return Err("The created native contact was not found uniquely.".into()); }
        let mut contact = found.remove(0);
        created_id = Some(contact.id.clone());
        checks.push(json!({"name":"contact_create_and_query","passed":true,"provider_id":contact.id}));
        contact.phones = vec!["+1 202 555 0193".into()];
        contact.emails = vec!["ios-smoke-updated@example.invalid".into()];
        request(&platform, DeviceRequest::SaveContact { contact: contact.clone() }).await?;
        let found = contacts(&platform, &name).await?;
        if found.len() != 1 || found[0].id != contact.id || found[0].name != name
            || found[0].phones != contact.phones || found[0].emails != contact.emails {
            return Err("The native contact update changed identity/name or did not preserve its requested values.".into());
        }
        checks.push(json!({"name":"contact_update_and_query","passed":true,"provider_id":contact.id}));
        request(&platform, DeviceRequest::DeleteContact { id: contact.id.clone() }).await?;
        if contacts(&platform, &name).await?.iter().any(|value| value.id == contact.id) {
            return Err("The deleted native contact remained in the contact store.".into());
        }
        created_id = None;
        checks.push(json!({"name":"contact_delete_and_query","passed":true}));
        Ok::<(), String>(())
    }.await;

    // Restore every changed setting even when a preceding assertion fails.
    let mut cleanup = Vec::<String>::new();
    if let Some(id) = created_id {
        if let Err(error) = request(&platform, DeviceRequest::DeleteContact { id }).await {
            cleanup.push(error);
        }
    }
    if let Some(original) = original_brightness {
        let restored = async {
            request(
                &platform,
                DeviceRequest::SetControl {
                    control: DeviceControl::Brightness,
                    value: ControlValue::Level(original),
                },
            )
            .await?;
            let observed = brightness(&snapshot(&platform).await?)?;
            if (observed - original).abs() > 0.02 {
                return Err("UIKit brightness did not restore its original level.".into());
            }
            Ok::<f64, String>(observed)
        }
        .await;
        match restored {
            Ok(observed) => {
                checks.push(json!({"name":"brightness_restore","passed":true,"observed":observed}))
            }
            Err(error) => cleanup.push(error),
        }
    }
    json!({"schema":1,"platform":"ios","simulator_only":true,
        "status":if result.is_ok() && cleanup.is_empty() {"passed"} else {"failed"},
        "checks":checks,"error":result.err(),"cleanup_errors":cleanup,
        "sent_calls":0,"sent_messages":0})
}

pub fn install(host: &Rc<NativeHost>) {
    if std::env::var("PHONE_IOS_DEVICE_TEST").as_deref() != Ok("1")
        || !cfg!(any(target_abi = "sim", target_arch = "x86_64"))
    {
        return;
    }
    report(&json!({"schema":1,"platform":"ios","status":"waiting_for_mount"}));
    let weak = Rc::downgrade(host);
    let attempts = std::cell::Cell::new(0);
    DispatcherTimer::run(move || {
        attempts.set(attempts.get() + 1);
        let Some(host) = weak.upgrade() else { return false };
        if !host.view().is_attached_to_visual_tree() {
            if attempts.get() >= 40 { report(&json!({"schema":1,"platform":"ios","status":"failed","error":"The native host did not mount for device roundtrips."})); return false; }
            return true;
        }
        let platform = host.runtime().platform();
        report(&json!({"schema":1,"platform":"ios","status":"running"}));
        drop(Dispatcher::current_dispatcher().spawn_local(async move {
            let value = roundtrip(platform).await;
            report(&value);
        }));
        false
    }, Duration::from_millis(500), DispatcherPriority::BACKGROUND).detach();
}
