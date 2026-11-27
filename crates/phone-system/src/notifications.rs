use crate::{common, session::Session};
use ferroui_base::layout::{HorizontalAlignment as H, VerticalAlignment as V};
use ferroui_base::media::{FontWeight, TextWrapping};
use ferroui_dynamic_data::IObservableCache;
use ferroui_view::prelude::*;
use phone_controls::{Icon, icon};
use phone_domain::{AppId, Route};
use phone_state::PhoneViewModel;
use std::rc::Rc;

mod dismissal;
#[cfg(test)]
mod tests;
pub(crate) use dismissal::Dismissal;

/// Both stores use one chronology. Device timestamps are milliseconds; local
/// phone records use seconds and are converted only for this presentation.
pub(crate) struct Group {
    pub timestamp: i64,
    pub key: String,
    pub view: Element,
}

pub(crate) fn contents_visible(vm: &PhoneViewModel, ui: &Session) -> bool {
    let locked = ui.locked.get() || vm.route() == Route::Lock;
    !locked || vm.data().settings.notifications_on_lock
}

fn chronological(groups: &mut [Group]) {
    groups.sort_by(|left, right| {
        right
            .timestamp
            .cmp(&left.timestamp)
            .then_with(|| left.key.cmp(&right.key))
    });
}

pub fn view(vm: Rc<PhoneViewModel>, ui: Rc<Session>) -> Element {
    let settings = vm.data().settings.clone();
    let peek = ui.notification_peek.get();
    let visible = contents_visible(&vm, &ui);
    let notifications = vm
        .notifications
        .items()
        .into_iter()
        .filter(|item| visible && settings.notifications_allowed(item.app))
        .collect::<Vec<_>>();
    let quick = crate::quick_settings::view(vm.clone(), ui.clone(), peek);
    let mut list = stack_panel().spacing(20.0).margin((18.0, 14.0));
    let mut sorted = notifications;
    sorted.sort_by_key(|n| std::cmp::Reverse(n.timestamp));
    let mut groups = Vec::<(AppId, Vec<_>)>::new();
    for item in sorted {
        if let Some((_, items)) = groups.iter_mut().find(|(app, _)| *app == item.app) {
            items.push(item);
        } else {
            groups.push((item.app, vec![item]));
        }
    }
    let mut rendered = crate::device_notifications::rows(vm.clone(), ui.clone(), visible);
    if groups.is_empty() && rendered.is_empty() {
        list = list.child(rsx! {TextBlock{
            text:if visible{"no new notifications"}else{"unlock to see notifications"},
            font_size:24.0,foreground:"#A0A0A0",margin:(0.0,24.0),text_wrapping:TextWrapping::Wrap
        }});
    }
    for (app, items) in groups {
        let group_id = items[0].id;
        let timestamp = items[0].timestamp.saturating_mul(1000);
        let dismiss = vm.clone();
        let target = Dismissal::from_name(&vm, &format!("Notification-{group_id}"));
        let mut rows = stack_panel().spacing(12.0);
        for item in items {
            let open = vm.clone();
            let open_ui = ui.clone();
            let route = item.route.clone();
            let id = item.id;
            rows=rows.child(rsx! {Button{class:"phone-flat",horizontal_content_alignment:H::Stretch,
                automation_properties_name:format!("{}: {}",item.title,item.body),
                on_click:move |_|{open_ui.locked.set(false);open_ui.notification_open.set(false);open_ui.notification_peek.set(false);open.dismiss_notification(id);open.navigate(route.clone());},
                StackPanel{spacing:4.0,
                    TextBlock{text:item.title.clone(),font_size:22.0,foreground:"White",text_wrapping:TextWrapping::Wrap}
                    TextBlock{text:item.body.clone(),font_size:17.0,foreground:if settings.high_contrast{"White"}else{"#C4C4C4"},text_wrapping:TextWrapping::Wrap}
                    TextBlock{text:phone_domain::time_text(item.timestamp.saturating_add(i64::from(vm.utc_offset())),settings.time_24h),font_size:12.0,foreground:if settings.high_contrast{"White"}else{"#A6A6A6"},margin:(0.0,4.0,0.0,8.0)}
                }
            }});
        }
        rendered.push(Group {timestamp,key:format!("phone.{}",app.icon()),view:rsx! {Grid{name:format!("Notification-{group_id}"),column_definitions:"*,32",row_definitions:"Auto,Auto",row_spacing:9.0,
            TextBlock{name:format!("NotificationHeader-{}",app.icon()),text:app.label().to_lowercase(),font_size:19.0,foreground:"White",font_weight:FontWeight::SemiBold}
            Button{Grid.column:1,class:"phone-flat",automation_properties_name:format!("Dismiss {} notifications",app.label()),on_click:move |_|{if let Some(target)=&target{target.dismiss(&dismiss);}}, {icon(Icon::Close,14.0,"#A6A6A6")}}
            Border{Grid.row:1,Grid.column_span:2,{rows}}
        }}});
    }
    chronological(&mut rendered);
    for group in rendered {
        list = list.child(group.view);
    }
    let clear = vm.clone();
    let all = vm.clone();
    let all_ui = ui.clone();
    let close = ui.clone();
    let close_vm = vm.clone();
    let battery = vm
        .environment()
        .battery_percent
        .map(|level| format!("       {level}%"))
        .unwrap_or_default();
    rsx! {Grid{name:"ActionCenter",row_definitions:if peek{"26,*,0,0,36"}else{"26,Auto,Auto,*,48"},background:if settings.high_contrast{"Black"}else{"#1F1F1F"},clip_to_bounds:true,
        TextBlock{text:format!("{}{battery}",common::clock(&vm)),font_size:13.0,foreground:"White",horizontal_alignment:H::Right,margin:(0.0,5.0,16.0,0.0)},
        Border{Grid.row:1,{quick}},
        if !peek {Grid{Grid.row:2,column_definitions:"*,*",margin:(18.0,0.0,18.0,14.0),
            Button{class:"phone-flat",is_enabled:visible&&settings.notifications,automation_properties_name:"Clear all notifications",on_click:move |_|crate::device_notifications::clear(&clear),TextBlock{text:"clear all",font_size:17.0,foreground:"White"}},
            Button{Grid.column:1,class:"phone-flat",horizontal_alignment:H::Right,automation_properties_name:"All settings",on_click:move |_|{all_ui.locked.set(false);all_ui.notification_peek.set(false);all_ui.notification_open.set(false);all.navigate(Route::App(AppId::Settings));},TextBlock{text:"all settings",font_size:17.0,foreground:"White"}}
        }}
        if !peek {Border{Grid.row:3,{common::scroll(list.into())}}}
        Button{Grid.row:4,class:"phone-flat",horizontal_content_alignment:H::Center,vertical_content_alignment:V::Center,automation_properties_name:"Close action center",on_click:move |_|{close.notification_peek.set(false);close.notification_open.set(false);if close_vm.route()==Route::Notifications{close_vm.back();}},
            Border{width:80.0,height:4.0,background:"#888888"}}
    }}
}
