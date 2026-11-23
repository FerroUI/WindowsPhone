use crate::protocol::{self, Observation};
use ferroui_android::{interop::java::*, FerroAndroidApplication};
use ferroui_base::{
    reactive::Subscription,
    threading::{DispatcherPriority, DispatcherTimer},
};
use phone_host::platform::{DeviceRequest, DeviceResponse, PlatformResponse, PlatformTask};
use std::{
    future::Future,
    path::PathBuf,
    pin::Pin,
    task::{Context, Poll},
    time::{Duration, Instant},
};

const BRIDGE: &str = "org/ferroui/phone/PhoneBridge";

pub(crate) fn class() -> JavaClass {
    JavaClass::find(BRIDGE)
}

pub(crate) fn notification_policy(policy: &phone_host::platform::DeviceNotificationPolicy) {
    let Ok(json) = serde_json::to_string(policy) else {
        return;
    };
    let context = FerroAndroidApplication::context();
    call_static_void(
        &class(),
        "notificationPolicy",
        "(Landroid/content/Context;Ljava/lang/String;)V",
        &[JavaValue::Object(Some(&context)), JavaValue::String(&json)],
    );
}

pub(crate) fn close() {
    let context = FerroAndroidApplication::context();
    call_static_void(
        &class(),
        "close",
        "(Landroid/content/Context;)V",
        &[JavaValue::Object(Some(&context))],
    );
}

pub(crate) fn data_path() -> Result<PathBuf, String> {
    let context = FerroAndroidApplication::context();
    let directory = call_object(&context, "getFilesDir", "()Ljava/io/File;", &[])
        .and_then(|directory| {
            call_object(&directory, "getAbsolutePath", "()Ljava/lang/String;", &[])
        })
        .map(|path| PathBuf::from(string_of(&path)))
        .ok_or_else(|| "Android did not provide an application data directory.".to_string())?;
    Ok(directory.join("windows-phone").join("phone.json"))
}

pub(crate) fn observation() -> Observation {
    let context = FerroAndroidApplication::context();
    call_static_object(
        &class(),
        "state",
        "(Landroid/content/Context;)Ljava/lang/String;",
        &[JavaValue::Object(Some(&context))],
    )
    .and_then(|value| {
        let text = string_of(&value);
        (text.len() <= 16384)
            .then(|| serde_json::from_str(&text).ok())
            .flatten()
    })
    .unwrap_or_default()
}

pub(crate) fn command(action: &str, args: serde_json::Value) -> Result<PlatformResponse, String> {
    let context = FerroAndroidApplication::context();
    let text = args.to_string();
    let result = call_static_object(
        &class(),
        "command",
        "(Landroid/content/Context;Ljava/lang/String;Ljava/lang/String;)Ljava/lang/String;",
        &[
            JavaValue::Object(Some(&context)),
            JavaValue::String(action),
            JavaValue::String(&text),
        ],
    )
    .ok_or_else(|| "Android did not return a command result.".to_string())?;
    protocol::response(&string_of(&result))?
        .ok_or_else(|| "A synchronous Android command returned pending.".into())
}

pub(crate) fn applications() -> PlatformTask {
    let context = FerroAndroidApplication::context();
    task(call_static_long(
        &class(),
        "applications",
        "(Landroid/content/Context;)J",
        &[JavaValue::Object(Some(&context))],
    ))
}

pub(crate) fn location() -> PlatformTask {
    task(call_static_long(&class(), "locate", "()J", &[]))
}

pub(crate) fn device(request: DeviceRequest) -> PlatformTask {
    let context = FerroAndroidApplication::context();
    let timeout = if matches!(request, DeviceRequest::PickMessageAttachments { .. }) {
        Duration::from_secs(185)
    } else {
        Duration::from_secs(35)
    };
    let json = match serde_json::to_string(&request) {
        Ok(json) => json,
        Err(error) => return phone_host::platform::ready(Err(error.to_string())),
    };
    // Java starts UI/consent operations synchronously here. Only their bounded
    // completion crosses the future, which is cancelled if its owner disappears.
    task_with_timeout(
        call_static_long(
            &class(),
            "device",
            "(Landroid/content/Context;Ljava/lang/String;)J",
            &[JavaValue::Object(Some(&context)), JavaValue::String(&json)],
        ),
        timeout,
    )
}

pub(crate) fn events() -> Vec<PlatformResponse> {
    let context = FerroAndroidApplication::context();
    let Some(value) = call_static_object(
        &class(),
        "events",
        "(Landroid/content/Context;)Ljava/lang/String;",
        &[JavaValue::Object(Some(&context))],
    ) else {
        return Vec::new();
    };
    let json = string_of(&value);
    if json.len() > 1024 * 1024 {
        return Vec::new();
    }
    let Ok(events) = serde_json::from_str::<Vec<DeviceResponse>>(&json) else {
        return Vec::new();
    };
    if events.len() > 16 {
        return Vec::new();
    }
    let mut results: Vec<_> = events
        .into_iter()
        .filter_map(|event| {
            event
                .validate()
                .ok()
                .map(|_| PlatformResponse::Device(event))
        })
        .collect();
    if let Some(value) = call_static_object(&class(), "catalogEvents", "()Ljava/lang/String;", &[])
    {
        let encoded = string_of(&value);
        if !encoded.is_empty() {
            if let Ok(Some(response @ PlatformResponse::Applications(_))) =
                protocol::response(&encoded)
            {
                results.push(response);
            }
        }
    }
    results
}

fn task(id: i64) -> PlatformTask {
    task_with_timeout(id, Duration::from_secs(35))
}

fn task_with_timeout(id: i64, timeout: Duration) -> PlatformTask {
    Box::pin(AndroidTask {
        id,
        started: Instant::now(),
        timer: None,
        complete: false,
        timeout,
    })
}

struct AndroidTask {
    id: i64,
    started: Instant,
    timer: Option<Subscription>,
    complete: bool,
    timeout: Duration,
}

impl Future for AndroidTask {
    type Output = Result<PlatformResponse, String>;
    fn poll(mut self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<Self::Output> {
        self.timer.take();
        if self.id <= 0 || self.started.elapsed() > self.timeout {
            return Poll::Ready(Err(
                "Android could not start or finish the bounded request.".into(),
            ));
        }
        let reply = call_static_object(
            &class(),
            "poll",
            "(J)Ljava/lang/String;",
            &[JavaValue::Long(self.id)],
        )
        .ok_or_else(|| "The Android request did not return a status.".to_string())
        .and_then(|value| protocol::response(&string_of(&value)));
        match reply {
            Ok(None) => {
                let waker = context.waker().clone();
                self.timer = Some(DispatcherTimer::run_once(
                    move || waker.wake_by_ref(),
                    Duration::from_millis(100),
                    DispatcherPriority::BACKGROUND,
                ));
                Poll::Pending
            }
            result => {
                if result.is_err() {
                    call_static_void(&class(), "cancel", "(J)V", &[JavaValue::Long(self.id)]);
                }
                self.complete = true;
                Poll::Ready(result.and_then(|value| {
                    value.ok_or_else(|| "Android request is still pending.".into())
                }))
            }
        }
    }
}

impl Drop for AndroidTask {
    fn drop(&mut self) {
        self.timer.take();
        if !self.complete && self.id > 0 {
            call_static_void(&class(), "cancel", "(J)V", &[JavaValue::Long(self.id)]);
        }
    }
}
