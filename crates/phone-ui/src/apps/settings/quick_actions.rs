//! Notification preferences and the four configurable Action Center slots.
use super::update;
use crate::{
    common::{muted, text},
    session::Session,
};
use ferroui_base::layout::HorizontalAlignment as H;
use ferroui_view::prelude::*;
use phone_controls::{Icon, flat, icon, toggle};
use phone_domain::{AppId, Route};
use phone_state::PhoneViewModel;
use std::rc::Rc;

pub(super) fn notifications(vm: Rc<PhoneViewModel>, ui: Rc<Session>) -> Element {
    let settings = vm.data().settings.clone();
    if let Some(slot) = ui.selected.get().filter(|slot| *slot < 4) {
        let mut choices = stack_panel().spacing(6.0);
        for key in [
            "wi_fi",
            "bluetooth",
            "airplane",
            "rotation_lock",
            "location",
            "brightness",
            "camera",
        ] {
            let (label, kind) = quick_action(key);
            let model = vm.clone();
            let close = ui.selected;
            let current = settings.quick_actions[slot as usize] == key;
            choices = choices.child(rsx! { Button {
                class:"phone-flat",horizontal_alignment:H::Stretch,horizontal_content_alignment:H::Stretch,
                min_height:64.0,automation_properties_name:format!("Use {label} for quick action {}",slot+1),
                on_click:move |_|{
                    update(&model,|settings|{
                        let index=slot as usize;
                        let previous=settings.quick_actions[index].clone();
                        if let Some(other)=settings.quick_actions.iter().position(|value|value==key) {
                            settings.quick_actions[other]=previous;
                        }
                        settings.quick_actions[index]=key.into();
                    });
                    close.set(None);
                },
                Grid{column_definitions:"44,*,30",column_spacing:12.0,
                    Border{height:40.0,{icon(kind,25.0,settings.foreground())}}
                    TextBlock{Grid.column:1,text:label,font_size:24.0,vertical_alignment:ferroui_base::layout::VerticalAlignment::Center}
                    TextBlock{Grid.column:2,text:if current{"✓"}else{""},font_size:24.0,foreground:settings.accent.as_str(),vertical_alignment:ferroui_base::layout::VerticalAlignment::Center}
                }
            }});
        }
        let cancel = ui.selected;
        return rsx! {StackPanel{spacing:16.0,
            {text("choose a quick action",27.0)}
            {muted(&format!("Position {}",slot+1),17.0)}
            {choices}
            {flat("cancel",move||cancel.set(None))}
        }};
    }
    let enabled = vm.clone();
    let clear = vm.clone();
    let locked = vm.clone();
    let mut slots = grid().column_definitions("*,*,*,*").column_spacing(7.0);
    for (index, key) in settings.quick_actions.iter().enumerate() {
        let (label, kind) = quick_action(key);
        let pick = ui.selected;
        slots=slots.child(rsx!{Button{Grid.column:index as i32,class:"phone-flat",padding:0.0,
            horizontal_alignment:H::Stretch,horizontal_content_alignment:H::Stretch,min_height:104.0,
            automation_properties_name:format!("Choose quick action {}: {label}",index+1),
            on_click:move |_|pick.set(Some(index as u64)),
            StackPanel{spacing:8.0,
                Border{height:57.0,background:settings.accent.as_str(),{icon(kind,25.0,"White")}}
                TextBlock{text:label,font_size:13.0,text_wrapping:ferroui_base::media::TextWrapping::Wrap,horizontal_alignment:H::Center}
            }
        }});
    }
    let mut apps = stack_panel().spacing(3.0);
    for app in [
        AppId::Phone,
        AppId::Messaging,
        AppId::Outlook,
        AppId::Calendar,
        AppId::Alarms,
        AppId::Store,
        AppId::OneNote,
    ] {
        let model = vm.clone();
        apps=apps.child(rsx!{Button{class:"phone-flat",horizontal_alignment:H::Stretch,horizontal_content_alignment:H::Stretch,padding:(0.0,10.0),
            automation_properties_name:format!("Notification settings for {}",app.label()),
            on_click:move |_|model.navigate(Route::Setting(format!("notification:{}",app.label()))),
            StackPanel{spacing:5.0,{text(app.label(),27.0)}{muted(&super::notification_apps::summary(settings.notification_preference(app)),15.0)}}
        }});
    }
    rsx! {StackPanel{spacing:16.0,
        {text("Choose your quick actions",23.0)}
        {slots}
        {muted("Tap a button to change it. Your four choices appear in Action Center.",17.0)}
        {toggle("Show notifications",settings.notifications,move |value|update(&enabled,|settings|settings.notifications=value))}
        {toggle("Show notifications when locked",settings.notifications_on_lock,move |value|update(&locked,|settings|settings.notifications_on_lock=value))}
        {flat("clear all notifications",move ||clear.dispatch(phone_domain::Action::ClearNotifications))}
        {text("Manage notifications for each app",24.0)}{apps}
    }}
}

fn quick_action(key: &str) -> (&'static str, Icon) {
    match key {
        "wi_fi" => ("Wi-Fi", Icon::Wifi),
        "bluetooth" => ("Bluetooth", Icon::Bluetooth),
        "airplane" => ("flight mode", Icon::Airplane),
        "rotation_lock" => ("rotation lock", Icon::Rotation),
        "location" => ("location", Icon::Location),
        "brightness" => ("brightness", Icon::Sun),
        _ => ("camera", Icon::Camera),
    }
}
