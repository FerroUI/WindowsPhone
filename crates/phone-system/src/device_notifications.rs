use crate::session::Session;
use ferroui_base::{layout::HorizontalAlignment as H, media::TextWrapping};
use ferroui_view::prelude::*;
use phone_controls::{icon, Icon};
use phone_sdk::host::platform::*;
use phone_state::PhoneViewModel;
use std::{collections::BTreeMap, rc::Rc};
pub fn rows(vm: Rc<PhoneViewModel>, ui: Rc<Session>, visible: bool) -> Vec<Element> {
    if !visible || !vm.data().settings.notifications {
        return Vec::new();
    }
    let mut groups = BTreeMap::<String, Vec<DeviceNotification>>::new();
    for value in vm.device_state().notifications {
        groups
            .entry(value.package_id.clone())
            .or_default()
            .push(value);
    }
    let mut rows = Vec::new();
    for (_, items) in groups {
        let title = items[0].application.clone();
        let keys = items
            .iter()
            .filter(|v| v.clearable)
            .map(|v| v.key.clone())
            .collect::<Vec<_>>();
        let dismiss = vm.clone();
        let mut content = stack_panel().spacing(12.0);
        for item in items {
            let open = vm.clone();
            let close = ui.clone();
            let key = item.key.clone();
            let dismiss_one = vm.clone();
            let dismiss_key = item.key.clone();
            let mut actions = stack_panel().spacing(5.0);
            for action in item.actions {
                let model = vm.clone();
                let session = ui.clone();
                let key = item.key.clone();
                let id = action.id;
                let reply = action.allows_reply;
                actions=actions.child(rsx!{Button{class:"phone-flat",horizontal_alignment:H::Left,automation_properties_name:format!("{}: {}",item.application,action.title),
                    is_enabled:!ui.locked.get()&&vm.route()!=phone_domain::Route::Lock,
                    on_click:move |_|{
                        if reply{let model=model.clone();let key=key.clone();let id=id.clone();let error=session.form_error;session.form_with_action("reply",vec![("message",String::new())],"send reply",Rc::new(move|values|{
                            let value=values.first().cloned().unwrap_or_default();if value.trim().is_empty(){error.set("Type a reply.".into());return;}
                            model.request_device(DeviceRequest::NotificationAction{key:key.clone(),action:Some(id.clone()),reply:Some(value)});
                        }));}else{model.request_device(DeviceRequest::NotificationAction{key:key.clone(),action:Some(id.clone()),reply:None});}
                    },TextBlock{text:action.title,font_size:17.0,foreground:"#1BA1E2"}}});
            }
            content=content.child(rsx!{StackPanel{spacing:5.0,
                if item.clearable{Button{class:"phone-flat",horizontal_alignment:H::Right,automation_properties_name:format!("Dismiss device notification {}",item.title),on_click:move|_|dismiss_one.request_device(DeviceRequest::DismissNotification{key:dismiss_key.clone()}),{icon(Icon::Close,14.0,"#A6A6A6")}}}
                Button{class:"phone-flat",horizontal_content_alignment:H::Stretch,automation_properties_name:format!("Device notification: {}",item.title),
                    is_enabled:!ui.locked.get()&&vm.route()!=phone_domain::Route::Lock,
                    on_click:move |_|{close.notification_open.set(false);open.request_device(DeviceRequest::NotificationAction{key:key.clone(),action:None,reply:None});},
                    StackPanel{spacing:4.0,
                        TextBlock{text:item.title,font_size:22.0,foreground:"White",text_wrapping:TextWrapping::Wrap}
                        TextBlock{text:item.body,font_size:17.0,foreground:"#C4C4C4",text_wrapping:TextWrapping::Wrap}
                        TextBlock{text:phone_domain::time_text(item.timestamp.div_euclid(1000).saturating_add(i64::from(vm.utc_offset())),vm.data().settings.time_24h),font_size:12.0,foreground:"#A6A6A6"}
                    }
                }{actions}
            }});
        }
        rows.push(rsx!{Grid{column_definitions:"*,32",row_definitions:"Auto,Auto",row_spacing:9.0,
            TextBlock{text:title.to_lowercase(),font_size:19.0,foreground:"White"}
            if !keys.is_empty(){Button{Grid.column:1,class:"phone-flat",automation_properties_name:format!("Dismiss device {title} notifications"),
                on_click:move |_|dismiss.request_device(DeviceRequest::DismissNotifications{keys:keys.clone()}),{icon(Icon::Close,14.0,"#A6A6A6")}}}
            Border{Grid.row:1,Grid.column_span:2,{content}}
        }});
    }
    rows
}
pub fn clear(vm: &PhoneViewModel) {
    let keys = vm
        .device_state()
        .notifications
        .into_iter()
        .filter(|v| v.clearable)
        .map(|v| v.key)
        .collect::<Vec<_>>();
    if !keys.is_empty() {
        vm.request_device(DeviceRequest::DismissNotifications { keys });
    }
    vm.clear_notifications();
}
