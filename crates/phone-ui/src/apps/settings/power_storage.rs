//! Real host battery observations and measured application-document storage.
use super::update;
use crate::{
    common::{muted, page, scroll, text},
    session::Session,
};
use ferroui_base::layout::Orientation;
use ferroui_view::prelude::*;
use phone_controls::{Icon, action, app_bar, flat, pivot, toggle};
use phone_domain::{AppId, Route};
use phone_state::{HostRequest, PhoneViewModel};
use std::rc::Rc;

pub(super) fn battery_settings(vm: Rc<PhoneViewModel>) -> Element {
    let settings = vm.data().settings.clone();
    let environment = vm.environment();
    let percent = environment
        .battery_percent
        .map(|level| format!("{level}%"))
        .unwrap_or_else(|| "not available".into());
    let charging = environment
        .charging
        .map(|value| {
            if value {
                "charging"
            } else if environment.external_power == Some(true) {
                "connected to power"
            } else {
                "not charging"
            }
        })
        .unwrap_or("battery information unavailable");
    let save = vm.clone();
    let motion = vm.clone();
    rsx! {StackPanel{spacing:20.0,
        {muted("Remaining battery life",18.0)}{text(&percent,55.0)}{muted(charging,19.0)}
        {toggle("Battery Saver",settings.battery_saver,move |enabled|update(&save,|s|s.battery_saver=enabled))}
        if environment.power_saver==Some(true) {{muted("Your device is also saving power. Decorative motion and live tiles pause until that policy changes.",16.0)}}
        {muted("Pauses live-tile animation and decorative motion inside this app. Media playback and saved changes remain available.",17.0)}
        {flat("ease of access",move ||motion.navigate(Route::Setting("ease of access".into())))}
        {muted("Battery readings are shown only when your host supplies them. Your device controls charging and system power policies.",15.0)}
    }}
}

pub fn battery_page(vm: Rc<PhoneViewModel>, ui: Rc<Session>) -> Element {
    let selected = ui.pivot.get().min(1);
    let usage = ui.pivot;
    let settings = ui.pivot;
    let environment = vm.environment();
    let level = environment
        .battery_percent
        .map(|value| format!("{value}%"))
        .unwrap_or_else(|| "–".into());
    let body = if selected == 1 {
        rsx! {Border{margin:20.0,{battery_settings(vm.clone())}}}
    } else {
        rsx! {StackPanel{spacing:20.0,margin:20.0,
            {text(&level,72.0)}{text("remaining battery",24.0)}
            {muted(if environment.charging==Some(true){"Connected to power"}else if environment.battery_percent.is_some(){"Using your host's current battery reading"}else{"This host does not expose its battery level."},18.0)}
            {muted("Per-app battery usage is not available from this host. This page does not estimate another app's power consumption.",17.0)}
        }}
    };
    page(
        "battery saver",
        scroll(rsx! {StackPanel{
            StackPanel{orientation:Orientation::Horizontal,margin:(20.0,0.0),
                {pivot("usage",selected==0,move ||usage.set(0))}{pivot("settings",selected==1,move ||settings.set(1))}}
            {body}
        }}),
        Element::Empty,
    )
}

pub(super) fn storage(vm: Rc<PhoneViewModel>) -> Element {
    let data = vm.data();
    let total = serde_json::to_vec(&*data)
        .map(|bytes| bytes.len())
        .unwrap_or(0);
    let categories = [
        (
            "documents",
            serde_json::to_vec(&data.documents)
                .map(|v| v.len())
                .unwrap_or(0),
            AppId::Office,
            "#D24726",
        ),
        (
            "photos",
            serde_json::to_vec(&data.photos)
                .map(|v| v.len())
                .unwrap_or(0),
            AppId::Photos,
            "#00ABA9",
        ),
        (
            "music+videos",
            serde_json::to_vec(&data.tracks)
                .map(|v| v.len())
                .unwrap_or(0),
            AppId::Music,
            "#F15A24",
        ),
        (
            "contacts",
            serde_json::to_vec(&data.contacts)
                .map(|v| v.len())
                .unwrap_or(0),
            AppId::People,
            "#A4C400",
        ),
        (
            "messages",
            serde_json::to_vec(&data.conversations)
                .map(|v| v.len())
                .unwrap_or(0),
            AppId::Messaging,
            "#D80073",
        ),
        (
            "mail",
            serde_json::to_vec(&data.mail).map(|v| v.len()).unwrap_or(0),
            AppId::Outlook,
            "#1BA1E2",
        ),
        (
            "notes",
            serde_json::to_vec(&data.notes)
                .map(|v| v.len())
                .unwrap_or(0),
            AppId::OneNote,
            "#80397B",
        ),
    ];
    drop(data);
    let mut list = stack_panel().spacing(10.0);
    for (name, bytes, app, color) in categories {
        let open = vm.clone();
        let columns = format!("{}*,{}*", bytes.max(1), total.saturating_sub(bytes).max(1));
        list=list.child(rsx!{Button{class:"phone-flat",horizontal_alignment:ferroui_base::layout::HorizontalAlignment::Stretch,
            horizontal_content_alignment:ferroui_base::layout::HorizontalAlignment::Stretch,padding:0.0,min_height:70.0,
            automation_properties_name:format!("Storage for {name}: {}",size(bytes)),on_click:move |_|open.navigate(Route::App(app)),
            StackPanel{spacing:6.0,
                Grid{column_definitions:"*,Auto",TextBlock{text:name,font_size:25.0}TextBlock{Grid.column:1,text:size(bytes),font_size:18.0}}
                Grid{height:7.0,background:"#444444",column_definitions:columns.as_str(),Border{background:color}}
            }
        }});
    }
    let backup = vm.clone();
    let files = vm.clone();
    rsx! {StackPanel{spacing:18.0,
        {text("phone",36.0)}{text(&size(total),48.0)}{muted("saved phone document",18.0)}
        {list}
        {muted("These are measured JSON document sizes. Imported photo, audio and video bytes are stored separately; your host's free disk space is not reported here.",16.0)}
        {flat("manage files",move ||files.navigate(Route::App(AppId::Files)))}
        {flat("back up now",move ||backup.request(HostRequest::ExportBackup))}
        {super::host::storage(vm.clone())}
    }}
}

pub fn data_page(vm: Rc<PhoneViewModel>, ui: Rc<Session>) -> Element {
    let selected = ui.pivot.get().min(1);
    let overview = ui.pivot;
    let usage = ui.pivot;
    let total = vm.export_json().map(|value| value.len()).unwrap_or(0);
    let connection = match vm.environment().online {
        Some(true) => "online",
        Some(false) => "offline",
        None => "not reported",
    };
    let settings = vm.clone();
    let backup = vm.clone();
    let content = if selected == 1 {
        rsx! {Border{margin:20.0,{storage(vm.clone())}}}
    } else {
        rsx! {StackPanel{margin:20.0,spacing:20.0,
            {text(&size(total),68.0)}{text("saved phone data",24.0)}
            {muted(&format!("Connection: {connection}"),20.0)}
            {muted("Your host does not provide cellular data usage or your carrier allowance. This view measures local saved content.",18.0)}
            {flat("storage sense",move ||settings.navigate(Route::Setting("storage sense".into())))}
        }}
    };
    page(
        "data sense",
        scroll(rsx! {StackPanel{
            StackPanel{orientation:Orientation::Horizontal,margin:(20.0,0.0),
                {pivot("overview",selected==0,move ||overview.set(0))}{pivot("usage",selected==1,move ||usage.set(1))}}
            {content}
        }}),
        app_bar(
            vec![action(Icon::Save, "back up", move || {
                backup.request(HostRequest::ExportBackup)
            })],
            false,
            vec![],
        ),
    )
}

fn size(bytes: usize) -> String {
    if bytes < 1024 {
        format!("{bytes} B")
    } else if bytes < 1024 * 1024 {
        format!("{:.1} KB", bytes as f64 / 1024.0)
    } else {
        format!("{:.2} MB", bytes as f64 / (1024.0 * 1024.0))
    }
}
