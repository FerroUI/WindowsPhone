use ferroui_base::layout::Orientation;
use ferroui_view::prelude::*;
use phone_controls::{action, app_bar, flat, pivot, Icon};
use phone_domain::Route;
use phone_sdk::{
    common::{muted, page, scroll, text},
    session::Session,
};
use phone_state::{HostRequest, PhoneViewModel};
use std::rc::Rc;

use phone_storage_support::size;
pub fn view(vm: Rc<PhoneViewModel>, ui: Rc<Session>) -> Element {
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
        rsx! {Border{margin:20.0,{phone_storage_support::storage(vm.clone())}}}
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

mod registration;
pub use registration::registration;
