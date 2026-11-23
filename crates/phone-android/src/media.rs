use crate::media_wire;
use ferroui_android::{interop::java::*, FerroAndroidApplication};
use phone_host::native::{
    media_backend::MediaBackend,
    media_session::{Command, Snapshot, Status},
};

/// Own on the UI thread; Java callbacks retain generation values, never this object.
pub(crate) struct AndroidMediaSession {
    java: JavaObject,
}
pub(crate) fn prepare() {
    let _ = JavaClass::find("org/ferroui/phone/PhoneMediaSession");
}
impl AndroidMediaSession {
    pub(crate) fn new() -> Result<Self, String> {
        let context = FerroAndroidApplication::context();
        let class = JavaClass::find("org/ferroui/phone/PhoneMediaSession");
        let java = call_static_object(
            &class,
            "create",
            "(Landroid/content/Context;)Lorg/ferroui/phone/PhoneMediaSession;",
            &[JavaValue::Object(Some(&context))],
        )
        .ok_or_else(|| "Android could not register its media session.".to_string())?
        .to_global();
        Ok(Self { java })
    }
}
impl MediaBackend for AndroidMediaSession {
    fn status(&self) -> Status {
        match call_object(&self.java, "status", "()Ljava/lang/String;", &[])
            .map(|value| string_of(&value))
            .as_deref()
        {
            Some("ready") => Status::Ready,
            _ => Status::Unavailable("The Android media session is closed or unavailable.".into()),
        }
    }
    fn publish(&self, snapshot: &Snapshot) -> Result<(), String> {
        let json = media_wire::snapshot(snapshot);
        let result = call_object(
            &self.java,
            "publish",
            "(Ljava/lang/String;)Ljava/lang/String;",
            &[JavaValue::String(&json)],
        )
        .ok_or_else(|| "Android did not acknowledge its media state.".to_string())?;
        let error = string_of(&result);
        if error.is_empty() {
            Ok(())
        } else {
            Err(error)
        }
    }
    fn take_commands(&self) -> Vec<Command> {
        call_object(&self.java, "takeCommands", "()Ljava/lang/String;", &[])
            .and_then(|value| media_wire::commands(&string_of(&value)).ok())
            .unwrap_or_default()
    }
}
impl Drop for AndroidMediaSession {
    fn drop(&mut self) {
        call_void(&self.java, "close", "()V", &[]);
    }
}
