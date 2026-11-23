//! Lock, clock, radio profiles, device identity and backup settings.
use super::update;
use crate::{
    common::{muted, text},
    session::Session,
};
use ferroui_view::prelude::*;
use phone_controls::{flat, toggle};
use phone_domain::Settings;
use phone_state::{HostRequest, PhoneViewModel};
use std::rc::Rc;

pub(super) fn date_time(vm: Rc<PhoneViewModel>) -> Element {
    let settings = vm.data().settings.clone();
    let model = vm.clone();
    let (year, month, day) = phone_domain::date_parts(vm.local_now());
    let offset = vm.utc_offset();
    let zone = format!(
        "UTC{}{:02}:{:02}",
        if offset < 0 { "−" } else { "+" },
        offset.unsigned_abs() / 3600,
        offset.unsigned_abs() % 3600 / 60
    );
    let region = vm.clone();
    rsx! {StackPanel{spacing:18.0,
        {text(&format!("{} {}",day,phone_domain::month_name(month)),32.0)}
        {muted(&year.to_string(),22.0)}
        {text(&phone_domain::time_text(vm.local_now(),settings.time_24h),46.0)}
        {toggle("24-hour clock",settings.time_24h,move |enabled|update(&model,|value|value.time_24h=enabled))}
        {muted("Time zone",18.0)}{text(&zone,25.0)}
        {muted("Short date",18.0)}{text(&settings.date_order.format(year,month,day),25.0)}
        {flat("regional format",move ||region.navigate(phone_domain::Route::Setting("region".into())))}
        {muted("Date and time are supplied by your device.",16.0)}
    }}
}

pub(super) fn backup(vm: Rc<PhoneViewModel>) -> Element {
    let data = vm.data();
    let detail = format!(
        "{} contacts\n{} notes\n{} appointments\n{} photos",
        data.contacts.len(),
        data.notes.len(),
        data.events.len(),
        data.photos.len()
    );
    drop(data);
    let export = vm.clone();
    let import = vm.clone();
    rsx! {StackPanel{spacing:20.0,
        {text("apps+settings",30.0)}
        {muted(&detail,18.0)}
        {flat("save a backup",move ||export.request(HostRequest::ExportBackup))}
        {flat("restore from a backup",move ||import.request(HostRequest::ImportBackup))}
        {muted("A backup includes this app's settings, tile layout and local data. Keep the backup file somewhere safe. Restoring replaces the current local data.",16.0)}
    }}
}

pub(super) fn about(vm: Rc<PhoneViewModel>, ui: Rc<Session>) -> Element {
    let name = vm.data().settings.phone_name.clone();
    let model = vm.clone();
    let rename = ui.clone();
    let link = vm.clone();
    rsx! {StackPanel{spacing:16.0,
        {muted("Name",17.0)}{text(&name,28.0)}
        {flat("edit name",move ||{
            let current=model.data().settings.phone_name.clone();let model=model.clone();let error=rename.form_error;
            rename.form("phone name",vec![("name",current)],Rc::new(move |values|{
                let name=values.first().cloned().unwrap_or_default().trim().to_owned();
                if name.is_empty(){error.set("Enter a phone name.".into());return;}
                update(&model,|value|value.phone_name=name);
            }));
        })}
        {muted("Software",17.0)}{text("Windows Phone 8.1 experience",25.0)}
        {muted("An independent recreation built with FerroUI and Rust. This is an app, with local data and host integrations; it does not run the Windows Phone operating system.",16.0)}
        {flat("project + source",move ||link.request(HostRequest::OpenUrl{url:"https://github.com/FerroUI/WindowsPhone".into()}))}
    }}
}

pub(super) fn radio(vm: Rc<PhoneViewModel>, name: &str) -> Element {
    let settings = vm.data().settings.clone();
    let (label, value, field): (&str, bool, fn(&mut Settings, bool)) = match name {
        "Wi-Fi" => ("Wi-Fi networking", settings.wi_fi, |s, v| s.wi_fi = v),
        "Bluetooth" => ("Bluetooth", settings.bluetooth, |s, v| s.bluetooth = v),
        "flight mode" => ("Status", settings.airplane, |s, v| s.airplane = v),
        "screen rotation" => ("Rotation lock", settings.rotation_lock, |s, v| {
            s.rotation_lock = v
        }),
        "battery saver" => ("Battery Saver", settings.battery_saver, |s, v| {
            s.battery_saver = v
        }),
        _ => ("Location", settings.location, |s, v| s.location = v),
    };
    let model = vm.clone();
    rsx! {StackPanel{spacing:18.0,
        {toggle(label,value,move |enabled|update(&model,|settings|field(settings,enabled)))}
        {muted(if name=="battery saver"{"Reduces live tile animation in this app."}else{"Changes this app's phone profile. To manage your physical device, use its system settings."},16.0)}
    }}
}
