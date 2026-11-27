use crate::{
    notifications::{Dismissal, Group},
    session::Session,
};
use ferroui_base::{
    layout::HorizontalAlignment as H,
    media::{FontWeight, TextWrapping},
};
use ferroui_view::prelude::*;
use phone_controls::{Icon, icon};
use phone_sdk::host::platform::*;
use phone_state::PhoneViewModel;
use std::{collections::BTreeMap, rc::Rc};
pub(crate) fn rows(vm: Rc<PhoneViewModel>, ui: Rc<Session>, visible: bool) -> Vec<Group> {
    if !visible || !vm.data().settings.notifications {
        return Vec::new();
    }
    let device = vm.device_state();
    let actions_available = device
        .snapshot
        .supports(DeviceOperation::NotificationActions);
    let mut groups = BTreeMap::<String, Vec<DeviceNotification>>::new();
    for value in device.notifications {
        groups
            .entry(value.package_id.clone())
            .or_default()
            .push(value);
    }
    let mut rows = Vec::new();
    let high_contrast = vm.data().settings.high_contrast;
    for (package, items) in groups {
        let title = items[0].application.clone();
        let timestamp = items.iter().map(|item| item.timestamp).max().unwrap_or(0);
        let name = format!("DeviceNotifications-{package}");
        let keys = if actions_available {
            items
                .iter()
                .filter(|item| item.clearable)
                .map(|item| item.key.clone())
                .collect()
        } else {
            Vec::new()
        };
        let target = Dismissal::device(&vm, &package, keys);
        let dismissible = target.is_some();
        let dismiss = vm.clone();
        let mut content = stack_panel().spacing(12.0);
        for item in items {
            let open = vm.clone();
            let close = ui.clone();
            let key = item.key.clone();
            let mut actions = stack_panel().spacing(5.0);
            for action in item.actions {
                let model = vm.clone();
                let session = ui.clone();
                let key = item.key.clone();
                let id = action.id;
                let reply = action.allows_reply;
                actions=actions.child(rsx!{Button{class:"phone-flat",horizontal_alignment:H::Left,automation_properties_name:format!("{}: {}",item.application,action.title),
                    is_enabled:actions_available&&!ui.locked.get()&&vm.route()!=phone_domain::Route::Lock,
                    on_click:move |_|{
                        if reply{let model=model.clone();let key=key.clone();let id=id.clone();let error=session.form_error;session.form_with_action("reply",vec![("message",String::new())],"send reply",Rc::new(move|values|{
                            let value=values.first().cloned().unwrap_or_default();if value.trim().is_empty(){error.set("Type a reply.".into());return;}
                            model.request_device(DeviceRequest::NotificationAction{key:key.clone(),action:Some(id.clone()),reply:Some(value)});
                        }));}else{model.request_device(DeviceRequest::NotificationAction{key:key.clone(),action:Some(id.clone()),reply:None});}
                    },TextBlock{text:action.title,font_size:17.0,foreground:"#1BA1E2"}}});
            }
            content=content.child(rsx!{StackPanel{spacing:5.0,
                Button{class:"phone-flat",horizontal_content_alignment:H::Stretch,automation_properties_name:format!("Device notification: {}",item.title),
                    is_enabled:actions_available&&!ui.locked.get()&&vm.route()!=phone_domain::Route::Lock,
                    on_click:move |_|{close.notification_open.set(false);open.request_device(DeviceRequest::NotificationAction{key:key.clone(),action:None,reply:None});},
                    StackPanel{spacing:4.0,
                        TextBlock{text:item.title,font_size:22.0,foreground:"White",text_wrapping:TextWrapping::Wrap}
                        TextBlock{text:item.body,font_size:17.0,foreground:if high_contrast{"White"}else{"#C4C4C4"},text_wrapping:TextWrapping::Wrap}
                        TextBlock{text:phone_domain::time_text(item.timestamp.div_euclid(1000).saturating_add(i64::from(vm.utc_offset())),vm.data().settings.time_24h),font_size:12.0,foreground:if high_contrast{"White"}else{"#A6A6A6"},margin:(0.0,4.0,0.0,8.0)}
                    }
                }{actions}
            }});
        }
        rows.push(Group {timestamp,key:format!("device.{package}"),view:rsx!{Grid{name:name,column_definitions:"*,32",row_definitions:"Auto,Auto",row_spacing:9.0,
            TextBlock{name:format!("DeviceNotificationHeader-{package}"),text:title.to_lowercase(),font_size:19.0,font_weight:FontWeight::SemiBold,foreground:"White"}
            if dismissible{Button{Grid.column:1,class:"phone-flat",automation_properties_name:format!("Dismiss device {title} notifications"),
                on_click:move |_|{if let Some(target)=&target{target.dismiss(&dismiss);}}, {icon(Icon::Close,14.0,if high_contrast{"White"}else{"#A6A6A6"})}}}
            Border{Grid.row:1,Grid.column_span:2,{content}}
        }}});
    }
    rows
}
pub fn clear(vm: &PhoneViewModel) {
    if !vm.data().settings.notifications {
        return;
    }
    let epoch = vm.document_epoch();
    let captured = clearable_keys(vm, None);
    // The provider may synchronously restore a document. Finish the local
    // mutation first and never clear a document installed by its callback.
    vm.clear_notifications();
    if vm.document_epoch() == epoch && vm.data().settings.notifications {
        let keys = clearable_keys(vm, None)
            .into_iter()
            .filter(|key| captured.contains(key))
            .collect::<Vec<_>>();
        if !keys.is_empty() {
            vm.request_device(DeviceRequest::DismissNotifications { keys });
        }
    }
}

pub(crate) fn clearable_keys(vm: &PhoneViewModel, package: Option<&str>) -> Vec<String> {
    let device = vm.device_state();
    if !device.snapshot.supports(DeviceOperation::ReadNotifications)
        || !device
            .snapshot
            .supports(DeviceOperation::NotificationActions)
    {
        return Vec::new();
    }
    device
        .notifications
        .into_iter()
        .filter(|item| item.clearable && package.is_none_or(|package| item.package_id == package))
        .map(|item| item.key)
        .collect()
}
