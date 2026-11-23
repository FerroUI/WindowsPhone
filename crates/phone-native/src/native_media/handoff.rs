//! Use the platform launcher and temporary mobile grants, never a shell command.
use super::*;
use phone_host::platform::PlatformResponse;
impl NativeMedia {
    pub(super) fn handoff(self: &Rc<Self>, source: String, generation: u64) -> Result<(), String> {
        let root = self
            .window
            .upgrade()
            .ok_or("The media surface is closed.")?;
        let top = ferroui_controls::TopLevel::get_top_level(Some(&root))
            .ok_or("The media surface is not attached.")?;
        let launcher = top.launcher();
        let weak = Rc::downgrade(self);
        let store = crate::asset_store::AssetStore::default();
        drop(Dispatcher::current_dispatcher().spawn_local(async move {
            let result: Result<String, String> = async {
                let path = if source.starts_with("media:") {
                    {
                        let owned_source = source.clone();
                        Some(crate::background::run(move || store.playable(&owned_source)).await?)
                    }
                } else {
                    None
                };
                let Some(state) = weak
                    .upgrade()
                    .filter(|state| state.media_generation.get() == generation)
                else {
                    return Ok(String::new());
                };
                if let Some((path, metadata)) = path {
                    if let Some(task) = state.platform.open_file(&path, &metadata.mime) {
                        return match task.await? {
                            PlatformResponse::Accepted { message } => Ok(message),
                            _ => Err("The file provider returned an unexpected response.".into()),
                        };
                    }
                    if !launcher.launch_file_info_async(&path).await {
                        return Err("No device application accepted this media file.".into());
                    }
                } else {
                    let uri = url::Url::parse(&phone_host::http::validate_url(&source)?)
                        .map_err(|_| "Invalid media address.")?;
                    if !launcher.launch_uri_async(&uri).await {
                        return Err("No device application accepted this media address.".into());
                    }
                }
                Ok(
                    "Requested playback in your device's media app. Use its playback controls."
                        .into(),
                )
            }
            .await;
            if let Some(state) = weak
                .upgrade()
                .filter(|state| state.media_generation.get() == generation)
            {
                match result {
                    Ok(message) if !message.is_empty() => state.status(message),
                    Err(error) => state.status(error),
                    _ => {}
                }
            }
        }));
        Ok(())
    }
}
