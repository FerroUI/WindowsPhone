use crate::{common, session::Session};
use ferroui_base::input::HoldingState;
use ferroui_base::layout::{HorizontalAlignment as H, VerticalAlignment as V};
use ferroui_base::media::FontWeight;
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
        automation_properties_name:"Open action center",on_click:move |_|{ui.notification_peek.set(false);ui.notification_open.set(true);},
        Grid{column_definitions:"Auto,Auto,Auto,*,Auto",column_spacing:7.0,margin:(15.0,3.0),
            {icon(if flight{Icon::Airplane}else{Icon::Signal},14.0,&color)}
            Border{Grid.column:1,if wifi{{icon(Icon::Wifi,14.0,&color)}}}
            Border{Grid.column:2,{battery(&vm,&color)}}
            TextBlock{Grid.column:4,text:common::clock(&vm),font_size:13.0,font_weight:FontWeight::SemiBold,vertical_alignment:V::Center}
        }
    }}
}
pub fn battery(vm: &PhoneViewModel, color: &str) -> Element {
    let environment = vm.environment();
    let level = environment.battery_percent;
    let label = level
        .map(|level| {
            format!(
                "Battery {level}%{}",
                if environment.charging == Some(true) {
                    ", charging"
                } else {
                    ""
                }
            )
        })
        .unwrap_or_else(|| "Battery status unavailable".into());
    rsx! {Grid{width:19.0,height:12.0,column_definitions:"17,2",automation_properties_name:label,
        Border{height:10.0,border_brush:color,border_thickness:1.0,vertical_alignment:V::Center,
            if let Some(level)=level {Border{height:6.0,width:13.0*f64::from(level)/100.0,background:if vm.data().settings.battery_saver || environment.power_saver==Some(true){"#F0A30A"}else{color},horizontal_alignment:H::Left,margin:(1.0,0.0)}}
        }
        Border{Grid.column:1,height:4.0,background:color,vertical_alignment:V::Center}
    }}
}
pub fn bar(vm: Rc<PhoneViewModel>, ui: Rc<Session>) -> Element {
    let back = vm.clone();
    let home = vm.clone();
    let search = vm.clone();
    let hold = ui.clone();
    let context = ui.clone();
    let alarm = ui
        .state::<phone_alarm_service::AlarmState>()
        .active
        .get()
        .is_some();
    rsx! {Grid{background:"Black",column_definitions:"*,*,*",height:54.0,
        Button{name:"HardwareBack",class:"phone-flat",automation_properties_name:"Back",horizontal_alignment:H::Stretch,horizontal_content_alignment:H::Center,
            on_click:move |_|crate::back(&back),
            on_holding:move |e|if e.holding_state()==HoldingState::Started && hold.state::<phone_alarm_service::AlarmState>().active.get_untracked().is_none(){hold.switcher_open.set(true);e.set_handled(true);},
            on_context_requested:move |e|{if context.state::<phone_alarm_service::AlarmState>().active.get_untracked().is_none(){context.switcher_open.set(true);}e.set_handled(true);},
            {icon(Icon::Back,23.0,"White")}},
        Button{name:"HardwareStart",Grid.column:1,class:"phone-flat",automation_properties_name:"Start",horizontal_alignment:H::Stretch,horizontal_content_alignment:H::Center,
            is_enabled:!alarm,on_click:move |_|crate::home(&home),{icon(Icon::Windows,25.0,"White")}},
        Button{name:"HardwareSearch",Grid.column:2,class:"phone-flat",automation_properties_name:"Search",horizontal_alignment:H::Stretch,horizontal_content_alignment:H::Center,
            is_enabled:!alarm,on_click:move |_|search.navigate(Route::Search),{icon(Icon::Search,22.0,"White")}}
    }}
}
pub fn lock(vm: Rc<PhoneViewModel>, ui: Rc<Session>) -> Element {
    crate::lockscreen::view(vm, ui)
}
mod switcher;
pub use switcher::view as switcher;
