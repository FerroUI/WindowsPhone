use crate::{common, session::Session};
use ferroui_base::layout::{HorizontalAlignment as H, VerticalAlignment as V};
use ferroui_base::media::{FontWeight, TextWrapping};
use ferroui_view::prelude::*;
use phone_controls::{icon, Icon};
use phone_domain::{AppId, Route};
use phone_state::PhoneViewModel;
use std::rc::Rc;

pub fn view(vm: Rc<PhoneViewModel>, ui: Rc<Session>) -> Element {
    let data = vm.data();
    let settings = data.settings.clone();
    let notifications = data.notifications.clone();
    drop(data);
    let mut quick = grid()
        .column_definitions("*,*,*,*")
        .column_spacing(3.0)
        .margin((12.0, 18.0, 12.0, 14.0));
    for (index, (kind, key, label, enabled)) in [
        (Icon::Wifi, "wi_fi", "Wi-Fi", settings.wi_fi),
        (
            Icon::Bluetooth,
            "bluetooth",
            "Bluetooth",
            settings.bluetooth,
        ),
        (Icon::Airplane, "airplane", "flight mode", settings.airplane),
        (
            Icon::Rotation,
            "rotation_lock",
            "rotation lock",
            settings.rotation_lock,
        ),
    ]
    .into_iter()
    .enumerate()
    {
        let toggle = vm.clone();
        quick=quick.child(rsx!{Button{Grid.column:index as i32,class:"phone-flat",height:96.0,horizontal_content_alignment:H::Stretch,automation_properties_name:format!("Toggle {label}"),on_click:move |_|toggle.toggle_setting(key),
            StackPanel{spacing:9.0,
                Border{height:58.0,background:if enabled{settings.accent.as_str()}else{"#383838"},{icon(kind,26.0,"White")}},
                TextBlock{text:label,font_size:12.0,foreground:"White",horizontal_alignment:H::Center}
            }
        }});
    }
    let mut list = stack_panel().spacing(12.0).margin((18.0, 14.0));
    let mut sorted = notifications;
    sorted.sort_by_key(|n| std::cmp::Reverse(n.timestamp));
    for item in sorted {
        let open = vm.clone();
        let open_ui = ui.clone();
        let dismiss = vm.clone();
        let route = item.route.clone();
        let id = item.id;
        list=list.child(rsx!{Grid{name:format!("Notification-{id}"),column_definitions:"*,32",row_definitions:"Auto,Auto",row_spacing:9.0,
            TextBlock{text:item.app.label().to_lowercase(),font_size:19.0,foreground:"White",font_weight:FontWeight::SemiBold},
            Button{Grid.column:1,class:"phone-flat",automation_properties_name:format!("Dismiss notification {}",item.title),on_click:move |_|dismiss.dismiss_notification(id),{icon(Icon::Close,14.0,"#A0A0A0")}},
            Button{Grid.row:1,Grid.column_span:2,class:"phone-flat",horizontal_content_alignment:H::Stretch,on_click:move |_|{open_ui.notification_open.set(false);open.dismiss_notification(id);open.navigate(route.clone());},
                StackPanel{spacing:4.0,
                    TextBlock{text:item.title,font_size:22.0,foreground:"White"},
                    TextBlock{text:item.body,font_size:17.0,foreground:"#C4C4C4",text_wrapping:TextWrapping::Wrap},
                    TextBlock{text:chrono::DateTime::from_timestamp(item.timestamp,0).unwrap_or_default().format("%H:%M").to_string(),font_size:12.0,foreground:"#909090",margin:(0.0,4.0,0.0,10.0)}
                }
            }
        }});
    }
    if vm.data().notifications.is_empty() {
        list=list.child(rsx!{TextBlock{text:"no new notifications",font_size:24.0,foreground:"#A0A0A0",margin:(0.0,24.0)}});
    }
    let clear = vm.clone();
    let all = vm.clone();
    let all_ui = ui.clone();
    let close = ui.clone();
    let close_vm = vm.clone();
    rsx! {Grid{name:"ActionCenter",row_definitions:"26,Auto,Auto,*,48",background:"#1F1F1F",
        TextBlock{text:format!("{}       100%",common::clock(&vm)),font_size:13.0,foreground:"White",horizontal_alignment:H::Right,margin:(0.0,5.0,16.0,0.0)},
        Border{Grid.row:1,{quick}},
        Grid{Grid.row:2,column_definitions:"*,*",margin:(18.0,0.0,18.0,14.0),
            Button{class:"phone-flat",automation_properties_name:"Clear all notifications",on_click:move |_|clear.clear_notifications(),TextBlock{text:"clear all",font_size:17.0,foreground:"White"}},
            Button{Grid.column:1,class:"phone-flat",horizontal_alignment:H::Right,automation_properties_name:"All settings",on_click:move |_|{all_ui.notification_open.set(false);all.navigate(Route::App(AppId::Settings));},TextBlock{text:"all settings",font_size:17.0,foreground:"White"}}
        },
        Border{Grid.row:3,{common::scroll(list.into())}},
        Button{Grid.row:4,class:"phone-flat",horizontal_content_alignment:H::Center,vertical_content_alignment:V::Center,automation_properties_name:"Close action center",on_click:move |_|{close.notification_open.set(false);if close_vm.route()==Route::Notifications{close_vm.back();}},
            Border{width:80.0,height:4.0,background:"#888888"}}
    }}
}
