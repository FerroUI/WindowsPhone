//! Contact/calendar interchange through each native platform storage provider.
use crate::operations::Lease;
use ferroui_base::{
    platform::storage::{
        FilePickerFileType, FilePickerOpenOptions, FilePickerSaveOptions, IStorageProvider,
    },
    threading::Dispatcher,
};
use phone_state::{HostRequest, PhoneViewModel};
use std::rc::Rc;

pub fn start(vm: Rc<PhoneViewModel>, provider: Rc<dyn IStorageProvider>, request: HostRequest) {
    let Ok(lease) = Lease::capture(&vm) else {
        return;
    };
    drop(Dispatcher::current_dispatcher().spawn_local(async move {
        if let Err(error) = handle(&lease, &vm, provider, request).await {
            lease.report(error);
        }
    }));
}

async fn handle(
    lease: &Lease,
    vm: &PhoneViewModel,
    provider: Rc<dyn IStorageProvider>,
    request: HostRequest,
) -> Result<(), String> {
    lease.check()?;
    match request {
        HostRequest::ImportContacts | HostRequest::ImportCalendar => {
            let contacts = matches!(request, HostRequest::ImportContacts);
            let mut options = FilePickerOpenOptions::new();
            options.set_title(Some(
                if contacts {
                    "Import contacts"
                } else {
                    "Import local-time appointments"
                }
                .into(),
            ));
            options.set_allow_multiple(false);
            options.set_file_type_filter(Some(vec![FilePickerFileType::new(Some(if contacts {
                "vCard contacts"
            } else {
                "iCalendar appointments"
            }))
            .with_patterns(&[if contacts { "*.vcf" } else { "*.ics" }])]));
            let files = provider
                .open_file_picker_async(options)
                .await
                .map_err(|e| e.to_string())?;
            lease.check()?;
            if let Some(file) = files.first() {
                let bytes = lease
                    .read(
                        file.as_ref(),
                        phone_domain::interchange::MAX_INTERCHANGE_BYTES,
                    )
                    .await?;
                lease.check()?;
                let ids = if contacts {
                    vm.import_vcards(&bytes)?
                } else {
                    vm.import_icalendar(&bytes)?
                };
                vm.set_status(format!(
                    "Imported {} {}.",
                    ids.len(),
                    if contacts { "contacts" } else { "appointments" }
                ));
            }
        }
        request => {
            let (file, sharing) = match request {
                HostRequest::ExportContacts { ids } => (vm.export_vcards(&ids)?, false),
                HostRequest::ShareContacts { ids } => (vm.export_vcards(&ids)?, true),
                HostRequest::ExportCalendar { ids } => (vm.export_icalendar(&ids)?, false),
                HostRequest::ShareCalendar { ids } => (vm.export_icalendar(&ids)?, true),
                _ => return Err("Unsupported contact or calendar file operation.".into()),
            };
            let mut options = FilePickerSaveOptions::new();
            options.set_title(Some(
                if sharing {
                    "Save a copy to share"
                } else {
                    "Save a copy"
                }
                .into(),
            ));
            options.set_suggested_file_name(Some(file.name));
            options.set_show_overwrite_prompt(Some(true));
            if let Some(destination) = provider
                .save_file_picker_async(options)
                .await
                .map_err(|e| e.to_string())?
            {
                lease.write(destination.as_ref(), &file.bytes).await?;
                lease.check()?;
                vm.set_status(if sharing {
                    format!(
                        "Saved {}. Attach this file in your preferred sharing app.",
                        destination.name()
                    )
                } else {
                    format!("Saved {}.", destination.name())
                });
            }
        }
    }
    Ok(())
}
