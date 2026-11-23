use crate::bridge;
use phone_host::{platform::*, HostEnvironment};
use serde_json::json;

#[derive(Default)]
pub struct AndroidPlatformPlugin {
    embedded: bool,
}

impl AndroidPlatformPlugin {
    pub fn data_path() -> Result<std::path::PathBuf, String> {
        bridge::data_path()
    }

    /// An owner mounts NativeHost into its existing FerroUI Android surface.
    pub fn embedded() -> Self {
        Self { embedded: true }
    }
}

impl PlatformPlugin for AndroidPlatformPlugin {
    fn id(&self) -> &'static str {
        "org.ferroui.platform.android"
    }

    fn state(&self) -> PlatformState {
        bridge::observation().platform_state(self.embedded)
    }

    fn environment(&self) -> HostEnvironment {
        bridge::observation().environment()
    }

    fn lifecycle(&self, event: HostLifecycle) {
        if event == HostLifecycle::Closing {
            bridge::close();
        }
    }

    fn media_backend(
        &self,
    ) -> Result<Option<Box<dyn phone_host::native::media_backend::MediaBackend>>, String> {
        Ok(Some(Box::new(crate::media::AndroidMediaSession::new()?)))
    }

    fn open_file(&self, path: &std::path::Path, mime: &str) -> Option<PlatformTask> {
        Some(ready(match path.to_str() {
            Some(path) if path.len() <= 4096 && mime.len() <= 128 => {
                bridge::command("open_file", json!({"path":path,"mime":mime}))
            }
            _ => Err("This Android file identity or content type is invalid.".into()),
        }))
    }

    fn dispatch(&self, request: PlatformRequest) -> PlatformTask {
        if let Err(error) = request.validate() {
            return ready(Err(error));
        }
        let result = match request {
            PlatformRequest::Refresh => return ready(Ok(PlatformResponse::State(self.state()))),
            PlatformRequest::ListApplications => return bridge::applications(),
            PlatformRequest::Locate => return bridge::location(),
            PlatformRequest::LaunchApplication { id } => {
                bridge::command("launch", json!({"id":id}))
            }
            PlatformRequest::RequestLauncher => bridge::command("request_home", json!({})),
            PlatformRequest::OpenSettings { page } => bridge::command(
                "settings",
                json!({"page": match page {
                    SystemSettingsPage::Application=>"application", SystemSettingsPage::Network=>"network",
                    SystemSettingsPage::Notifications=>"notifications", SystemSettingsPage::DefaultApps=>"default_apps",
                }}),
            ),
            PlatformRequest::Haptic { kind } => bridge::command(
                "haptic",
                json!({"kind":match kind {
                    HapticKind::Selection=>"selection", HapticKind::Light=>"light", HapticKind::Medium=>"medium", HapticKind::Heavy=>"heavy",
                }}),
            ),
            PlatformRequest::ShareText { title, text, url } => {
                bridge::command("share", json!({"title":title,"text":text,"url":url}))
            }
            PlatformRequest::RequestNotifications => {
                bridge::command("request_notifications", json!({}))
            }
            PlatformRequest::Notify { title, body } => {
                bridge::command("notify", json!({"title":title,"body":body}))
            }
        };
        ready(result)
    }
}
