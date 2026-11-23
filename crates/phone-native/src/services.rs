use ferroui_base::input::platform::ClipboardExtensions;
use ferroui_base::{threading::Dispatcher, Ref};
use ferroui_controls::TopLevel;
use phone_host::urls;
use phone_state::{HostRequest, PhoneViewModel};
use std::rc::Rc;

fn open(vm: &Rc<PhoneViewModel>, top: &Ref<TopLevel>, url: String) -> Result<(), String> {
    let uri = url::Url::parse(&url).map_err(|_| "Invalid handoff URI.")?;
    let operation = top.launcher().launch_uri_async(&uri);
    let weak = Rc::downgrade(vm);
    let epoch = vm.document_epoch();
    drop(Dispatcher::current_dispatcher().spawn_local(async move {
        if !operation.await {
            if let Some(vm) = weak.upgrade().filter(|vm| vm.document_epoch() == epoch) {
                vm.set_status("No device application accepted this request.".into());
            }
        }
    }));
    Ok(())
}

pub fn handle(
    vm: &Rc<PhoneViewModel>,
    window: &Ref<TopLevel>,
    request: HostRequest,
) -> Result<(), String> {
    match request {
        HostRequest::Platform { .. } => {
            Err("Platform requests belong to the runtime broker.".into())
        }
        HostRequest::HttpGet {
            request_id,
            url,
            max_bytes,
        } => {
            crate::network::start(request_id, url, max_bytes);
            Ok(())
        }
        HostRequest::MapTile {
            request_id,
            zoom,
            x,
            y,
        } => {
            crate::network::map_tile(request_id, zoom, x, y);
            Ok(())
        }
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
            if !source.starts_with("media:") {
                // A streamed item owns a library reference, never the remote
                // server's bytes. Removal must not become an HTTP operation.
                phone_host::http::validate_url(&source)?;
                crate::native_media::forget_source(&source);
                return Ok(());
            }
            crate::native_media::forget_source(&source);
            let model = vm.clone();
            let store = crate::asset_store::AssetStore::default();
            drop(Dispatcher::current_dispatcher().spawn_local(async move {
                if let Err(error) =
                    crate::background::run(move || crate::media_files::remove(&store, &source))
                        .await
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
        HostRequest::CheckForUpdates => {
            vm.set_status("Install an update through the source that provided this app. Your phone data is kept separately.".into());
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
        HostRequest::OpenUrl { url } | HostRequest::Browse { url } => {
            open(vm, window, urls::web_url(&url)?)
        }
        HostRequest::Dial { number } => open(vm, window, urls::dial_url(&number)?),
        HostRequest::Sms { number, body } => open(vm, window, urls::sms_url(&number, &body)?),
        HostRequest::Email { to, subject, body } => {
            open(vm, window, urls::mail_url(&to, &subject, &body)?)
        }
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
