use ferroui_base::input::platform::ClipboardExtensions;
use ferroui_base::{Ref, threading::Dispatcher};
use ferroui_controls::Window;
use phone_host::{native::open_external, urls};
use phone_state::{HostRequest, PhoneViewModel};
use std::rc::Rc;

fn open(url: String) -> Result<(), String> {
    open_external(std::ffi::OsStr::new(&url))
}

pub fn handle(
    vm: &Rc<PhoneViewModel>,
    window: &Ref<Window>,
    request: HostRequest,
) -> Result<(), String> {
    match request {
        request @ (HostRequest::ImportContacts | HostRequest::ImportCalendar
            | HostRequest::ExportContacts { .. } | HostRequest::ShareContacts { .. }
            | HostRequest::ExportCalendar { .. } | HostRequest::ShareCalendar { .. }) => {
            crate::pim::start(vm.clone(), window.storage_provider(), request);
            Ok(())
        }
        request @ (HostRequest::ImportPhotos | HostRequest::ImportMusic | HostRequest::ImportVideo) => {
            crate::media_files::start(vm.clone(), window.storage_provider(), request);
            Ok(())
        }
        request @ (HostRequest::ExportBackup | HostRequest::ImportBackup) => {
            crate::backup::start(vm.clone(), window.storage_provider(), request);
            Ok(())
        }
        HostRequest::PlayMedia { source } => {
            phone_ui::show_media(String::new(), source.clone(), false);
            let model = vm.clone();
            drop(Dispatcher::current_dispatcher().spawn_local(async move {
                let result = crate::background::run(move || {
                    let (path, name) = crate::media_files::open_media(&source)?;
                    open_external(path.as_os_str())?;
                    Ok((name, source))
                }).await;
                match result {
                    Ok((name, source)) => {
                        phone_ui::show_media(name, source, false);
                        model.set_status("Requested playback in your device's media app. Use that app's playback and volume controls.".into());
                    }
                    Err(error) => model.set_status(error),
                }
            }));
            Ok(())
        }
        HostRequest::MediaControl { .. } => Err("Playback is managed by your device's media app. Use its controls.".into()),
        HostRequest::DeleteMedia { source } => {
            let model = vm.clone();
            drop(Dispatcher::current_dispatcher().spawn_local(async move {
                if let Err(error) = crate::background::run(move || crate::media_files::remove(&source)).await { model.set_status(error); }
            }));
            Ok(())
        }
        HostRequest::RequestNotificationPermission => Err("System notification delivery is not connected on this desktop host. Phone alerts still appear inside the app.".into()),
        HostRequest::RequestPersistentStorage => { vm.set_status("Phone data is stored in your device's application data folder.".into()); Ok(()) }
        HostRequest::RefreshEnvironment => { vm.set_status("Device power information updates automatically.".into()); Ok(()) }
        request @ (HostRequest::ImportDocuments { .. } | HostRequest::ExportDocument { .. }) => {
            crate::documents::start(vm.clone(), window.storage_provider(), request);
            Ok(())
        }
        HostRequest::ShareDocuments { .. } => Err("Sharing document files is available in supported web browsers. Use export to save a desktop copy.".into()),
        HostRequest::AlarmAlert { .. } => Ok(()),
        HostRequest::CopyText { text } => {
            let clipboard = window.clipboard().ok_or("Clipboard access is unavailable.")?;
            let vm = vm.clone();
            drop(Dispatcher::current_dispatcher().spawn_local(async move {
                match clipboard.set_text_async(Some(&text)).await {
                    Ok(()) => vm.set_status("Copied.".into()),
                    Err(error) => vm.set_status(format!("Could not copy: {error}")),
                }
            }));
            Ok(())
        }
        HostRequest::OpenUrl { url } | HostRequest::Browse { url } => {
            open(urls::web_url(&url)?)
        }
        HostRequest::Dial { number } => open(urls::dial_url(&number)?),
        HostRequest::Sms { number, body } => open(urls::sms_url(&number, &body)?),
        HostRequest::Email { to, subject, body } => open(urls::mail_url(&to, &subject, &body)?),
        HostRequest::StopCamera | HostRequest::ClearBrowser => Ok(()),
        HostRequest::Notify { title, body } => { vm.set_status(format!("{title}: {body}")); Ok(()) }
        HostRequest::Share { url, .. } if url.starts_with("media:") => {
            crate::media_files::share_copy(vm.clone(), window.storage_provider(), url);
            Ok(())
        }
        HostRequest::Share { title, text, url } => {
            let text = [title, text, url].into_iter().filter(|value| !value.is_empty()).collect::<Vec<_>>().join("\n");
            handle(vm, window, HostRequest::CopyText { text })
        }
        HostRequest::StartCamera | HostRequest::CapturePhoto => Err("Live camera capture is available in the web edition. Choose photos to import images on this desktop.".into()),
        HostRequest::Locate | HostRequest::FetchWeather => Err("Location access is available in the web edition with your permission.".into()),
    }
}
