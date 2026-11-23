use ferroui_base::input::platform::ClipboardExtensions;
use ferroui_base::{threading::Dispatcher, Ref};
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
        HostRequest::ChooseFile {
            request_id,
            extensions,
            max_bytes,
        } => {
            crate::file_picker::start(
                vm,
                window.storage_provider(),
                request_id,
                extensions,
                max_bytes,
            );
            Ok(())
        }
        request @ (HostRequest::ImportContacts
        | HostRequest::ImportCalendar
        | HostRequest::ExportContacts { .. }
        | HostRequest::ShareContacts { .. }
        | HostRequest::ExportCalendar { .. }
        | HostRequest::ShareCalendar { .. }) => {
            crate::pim::start(vm.clone(), window.storage_provider(), request);
            Ok(())
        }
        request @ (HostRequest::ImportPhotos
        | HostRequest::ImportMusic
        | HostRequest::ImportVideo) => {
            crate::media_files::start(vm.clone(), window.storage_provider(), request);
            Ok(())
        }
        request @ (HostRequest::ExportBackup | HostRequest::ImportBackup) => {
            crate::backup::start(vm.clone(), window.storage_provider(), request);
            Ok(())
        }
        request @ (HostRequest::PlayMedia { .. }
        | HostRequest::MediaControl { .. }
        | HostRequest::StartCamera
        | HostRequest::StopCamera
        | HostRequest::CapturePhoto
        | HostRequest::AlarmAlert { .. }) => crate::native_media::handle(request),
        HostRequest::DeleteMedia { source } => {
            let model = vm.clone();
            drop(Dispatcher::current_dispatcher().spawn_local(async move {
                if let Err(error) =
                    crate::background::run(move || crate::media_files::remove(&source)).await
                {
                    model.set_status(error);
                }
            }));
            Ok(())
        }
        HostRequest::RequestNotificationPermission => {
            crate::notifications::refresh(vm, true);
            Ok(())
        }
        HostRequest::RequestPersistentStorage => {
            vm.set_status("Phone data is stored in your device's application data folder.".into());
            Ok(())
        }
        HostRequest::RefreshEnvironment => {
            crate::notifications::refresh(vm, false);
            vm.set_status("Device information refreshed. Power updates automatically.".into());
            Ok(())
        }
        request @ (HostRequest::ImportDocuments { .. }
        | HostRequest::ExportDocument { .. }
        | HostRequest::ShareDocuments { .. }) => {
            crate::documents::start(vm.clone(), window.storage_provider(), request);
            Ok(())
        }
        HostRequest::CopyText { text } => {
            let clipboard = window
                .clipboard()
                .ok_or("Clipboard access is unavailable.")?;
            let vm = vm.clone();
            drop(Dispatcher::current_dispatcher().spawn_local(async move {
                match clipboard.set_text_async(Some(&text)).await {
                    Ok(()) => vm.set_status("Copied.".into()),
                    Err(error) => vm.set_status(format!("Could not copy: {error}")),
                }
            }));
            Ok(())
        }
        HostRequest::OpenUrl { url } | HostRequest::Browse { url } => open(urls::web_url(&url)?),
        HostRequest::Dial { number } => open(urls::dial_url(&number)?),
        HostRequest::Sms { number, body } => open(urls::sms_url(&number, &body)?),
        HostRequest::Email { to, subject, body } => open(urls::mail_url(&to, &subject, &body)?),
        HostRequest::ClearBrowser => Ok(()),
        HostRequest::Notify { title, body } => {
            crate::notifications::show(vm, window, title, body);
            Ok(())
        }
        HostRequest::Share { url, .. } if url.starts_with("media:") => {
            crate::media_files::share_copy(vm.clone(), window.storage_provider(), url);
            Ok(())
        }
        HostRequest::Share { title, text, url } => {
            let text = [title, text, url]
                .into_iter()
                .filter(|value| !value.is_empty())
                .collect::<Vec<_>>()
                .join("\n");
            handle(vm, window, HostRequest::CopyText { text })
        }
        HostRequest::Locate => {
            crate::location::start(vm, false);
            Ok(())
        }
        HostRequest::FetchWeather => {
            crate::location::start(vm, true);
            Ok(())
        }
    }
}
