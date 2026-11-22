use crate::{common, images, session::Session};
use ferroui_base::input::HoldingState;
use ferroui_base::layout::{HorizontalAlignment as H, Orientation, VerticalAlignment as V};
use ferroui_base::media::FontWeight;
use ferroui_controls::primitives::ScrollBarVisibility;
use ferroui_view::prelude::*;
use phone_controls::{icon, Icon};
use phone_domain::Route;
use phone_state::PhoneViewModel;
use std::rc::Rc;

pub fn status(vm: Rc<PhoneViewModel>, ui: Rc<Session>) -> Element {
    let data = vm.data();
    let wifi = data.settings.wi_fi;
    let flight = data.settings.airplane;
    drop(data);
    let color = common::fg(&vm);
    rsx! {Button{name:"StatusBar",class:"phone-flat",height:26.0,horizontal_alignment:H::Stretch,horizontal_content_alignment:H::Stretch,
        automation_properties_name:"Open action center",on_click:move |_|ui.notification_open.set(true),
        Grid{column_definitions:"Auto,Auto,Auto,*,Auto",column_spacing:7.0,margin:(15.0,3.0),
            {icon(if flight{Icon::Airplane}else{Icon::Signal},14.0,&color)}
            Border{Grid.column:1,if wifi{{icon(Icon::Wifi,14.0,&color)}}}
            Border{Grid.column:2,{icon(Icon::Battery,17.0,&color)}}
            TextBlock{Grid.column:4,text:common::clock(&vm),font_size:13.0,font_weight:FontWeight::SemiBold,vertical_alignment:V::Center}
        }
    }}
}
pub fn bar(vm: Rc<PhoneViewModel>, ui: Rc<Session>) -> Element {
    let back = vm.clone();
    let home = vm.clone();
    let search = vm.clone();
    let hold = ui.clone();
    let context = ui.clone();
    let go_home = ui.clone();
    rsx! {Grid{background:"Black",column_definitions:"*,*,*",height:54.0,
        Button{name:"HardwareBack",class:"phone-flat",automation_properties_name:"Back",horizontal_alignment:H::Stretch,horizontal_content_alignment:H::Center,
            on_click:move |_|crate::back(&back),
            on_holding:move |e|if e.holding_state()==HoldingState::Started{hold.switcher_open.set(true);e.set_handled(true);},
            on_context_requested:move |e|{context.switcher_open.set(true);e.set_handled(true);},
            {icon(Icon::Back,23.0,"White")}},
        Button{name:"HardwareStart",Grid.column:1,class:"phone-flat",automation_properties_name:"Start",horizontal_alignment:H::Stretch,horizontal_content_alignment:H::Center,
            on_click:move |_|{while go_home.dismiss(){}go_home.locked.set(false);home.home();},{icon(Icon::Windows,25.0,"White")}},
        Button{name:"HardwareSearch",Grid.column:2,class:"phone-flat",automation_properties_name:"Search",horizontal_alignment:H::Stretch,horizontal_content_alignment:H::Center,
            on_click:move |_|search.navigate(Route::Search),{icon(Icon::Search,22.0,"White")}}
    }}
}
pub fn lock(vm: Rc<PhoneViewModel>, ui: Rc<Session>) -> Element {
    let data = vm.data();
    let wallpaper = if data.settings.wallpaper.is_empty() {
        "coast".into()
    } else {
        data.settings.wallpaper.clone()
    };
    let messages = data.unread_messages();
    let mail = data.unread_mail();
    drop(data);
    let date = chrono::DateTime::from_timestamp(vm.now(), 0).unwrap_or_default();
    let unlock = ui.clone();
    let unlock_vm = vm.clone();
    rsx! {Grid{name:"LockScreen",background:"Black",row_definitions:"*,Auto,Auto",clip_to_bounds:true,
        Border{Grid.row_span:3,{images::photo(&wallpaper)}},
        Border{Grid.row_span:3,background:"#33000000"},
        StackPanel{Grid.row:1,margin:(22.0,0.0,20.0,24.0),spacing:0.0,
            TextBlock{text:common::clock(&vm),font_size:86.0,font_weight:FontWeight::Light,foreground:"White"},
            TextBlock{text:date.format("%A").to_string(),font_size:29.0,font_weight:FontWeight::Light,foreground:"White"},
            TextBlock{text:date.format("%e %B").to_string().trim().to_owned(),font_size:29.0,font_weight:FontWeight::Light,foreground:"White"},
            StackPanel{orientation:Orientation::Horizontal,spacing:16.0,margin:(0.0,20.0,0.0,0.0),
                {icon(Icon::Message,20.0,"White")} TextBlock{text:messages.to_string(),foreground:"White",font_size:20.0},
                {icon(Icon::Mail,20.0,"White")} TextBlock{text:mail.to_string(),foreground:"White",font_size:20.0}
            }
        },
        Button{Grid.row:2,class:"phone-flat",automation_properties_name:"Unlock",horizontal_alignment:H::Stretch,horizontal_content_alignment:H::Center,height:58.0,
            on_click:move |_|{unlock.locked.set(false);if unlock_vm.route()==Route::Lock{unlock_vm.back();}},
            TextBlock{text:"swipe up to unlock",font_size:14.0,foreground:"White"}}
    }}
}
pub fn switcher(vm: Rc<PhoneViewModel>, ui: Rc<Session>) -> Element {
    let recent = vm.recent_apps();
    let mut cards = stack_panel()
        .orientation(Orientation::Horizontal)
        .spacing(22.0)
        .margin((28.0, 30.0));
    for app in recent {
        let open_vm = vm.clone();
        let open_ui = ui.clone();
        let close_vm = vm.clone();
        let color = if app.color().is_empty() {
            common::accent(&vm)
        } else {
            app.color().into()
        };
        cards=cards.child(rsx!{StackPanel{width:270.0,spacing:16.0,
            Grid{column_definitions:"*,40",TextBlock{text:app.label(),font_size:20.0,foreground:"White"},
                Button{Grid.column:1,class:"phone-flat",automation_properties_name:format!("Close {}",app.label()),on_click:move |_|close_vm.close_app(app),{icon(Icon::Close,18.0,"White")}}
            },
            Button{class:"phone-flat",height:400.0,horizontal_content_alignment:H::Stretch,automation_properties_name:format!("Switch to {}",app.label()),on_click:move |_|open_ui.open_app(&open_vm,app),
                Border{background:color.as_str(),border_brush:"#777777",border_thickness:1.0,padding:22.0,
                    StackPanel{vertical_alignment:V::Center,spacing:26.0,{icon(common::app_icon(app),78.0,"White")},TextBlock{text:app.label().to_lowercase(),font_size:36.0,foreground:"White",text_wrapping:ferroui_base::media::TextWrapping::Wrap}}
                }
            }
        }});
    }
    let done = ui.clone();
    let done_vm = vm.clone();
    rsx! {Grid{name:"AppSwitcher",background:"#222222",row_definitions:"Auto,*,54",
        TextBlock{text:"recent apps",font_size:35.0,foreground:"White",margin:(22.0,20.0)},
        ScrollViewer{Grid.row:1,horizontal_scroll_bar_visibility:ScrollBarVisibility::Hidden,vertical_scroll_bar_visibility:ScrollBarVisibility::Disabled,{cards}},
        Button{Grid.row:2,class:"phone-flat",horizontal_content_alignment:H::Center,on_click:move |_|{done.switcher_open.set(false);if done_vm.route()==Route::Switcher{done_vm.back();}},TextBlock{text:"back",font_size:18.0,foreground:"White"}}
    }}
}
