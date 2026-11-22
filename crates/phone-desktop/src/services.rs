use ferroui_base::input::platform::ClipboardExtensions;
use ferroui_base::{Ref, threading::Dispatcher};
use ferroui_controls::Window;
use phone_state::{HostRequest, PhoneViewModel};
use std::{process::Command, rc::Rc};

fn escape(text: &str) -> String {
    text.bytes()
        .map(|b| {
            if b.is_ascii_alphanumeric() || b"-._~".contains(&b) {
                (b as char).to_string()
            } else {
                format!("%{b:02X}")
            }
        })
        .collect()
}

fn open(url: &str) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    let mut command = Command::new("explorer.exe");
    #[cfg(target_os = "macos")]
    let mut command = Command::new("open");
    #[cfg(all(not(target_os = "windows"), not(target_os = "macos")))]
    let mut command = Command::new("xdg-open");
    command
        .arg(url)
        .spawn()
        .map(|_| ())
        .map_err(|e| format!("No application could open this request: {e}"))
}

pub fn handle(
    vm: &Rc<PhoneViewModel>,
    window: &Ref<Window>,
    request: HostRequest,
) -> Result<(), String> {
    match request {
        request @ (HostRequest::ImportDocuments { .. } | HostRequest::ExportDocument { .. }) => {
            crate::documents::start(vm.clone(), window.storage_provider(), request);
            Ok(())
        }
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
            if !url.starts_with("https://") && !url.starts_with("http://") { return Err("Enter an HTTP or HTTPS address.".into()); }
            open(&url)
        }
        HostRequest::Dial { number } => open(&format!("tel:{}", escape(&number))),
        HostRequest::Sms { number, body } => open(&format!("sms:{}?body={}", escape(&number), escape(&body))),
        HostRequest::Email { to, subject, body } => open(&format!("mailto:{}?subject={}&body={}", escape(&to), escape(&subject), escape(&body))),
        HostRequest::ExportBackup => {
            let path = crate::storage::data_path().with_file_name(format!("windows-phone-backup-{}.json", chrono::Local::now().format("%Y%m%d-%H%M%S")));
            crate::storage::save(&path, &vm.data())?;
            vm.set_status(format!("Backup saved to {}", path.display()));
            Ok(())
        }
        HostRequest::StopCamera | HostRequest::ClearBrowser | HostRequest::DeleteMedia { .. } => Ok(()),
        HostRequest::Notify { title, body } => { vm.set_status(format!("{title}: {body}")); Ok(()) }
        _ => Err("Open the web edition to use camera, media import, sharing, location and backup restore.".into()),
    }
}
