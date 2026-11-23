use ferroui_view::prelude::*;
use phone_controls::flat;
use phone_domain::{AppId, Route};
use phone_sdk::common::{muted, text};
use phone_state::{HostRequest, PhoneViewModel};
use std::rc::Rc;

pub fn storage(vm: Rc<PhoneViewModel>) -> Element {
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
        {phone_app_support::host::storage(vm.clone())}
    }}
}

pub fn size(bytes: usize) -> String {
    if bytes < 1024 {
        format!("{bytes} B")
    } else if bytes < 1024 * 1024 {
        format!("{:.1} KB", bytes as f64 / 1024.0)
    } else {
        format!("{:.2} MB", bytes as f64 / (1024.0 * 1024.0))
    }
}
