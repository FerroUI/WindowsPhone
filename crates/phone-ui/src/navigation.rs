use crate::{common, session::Session};
use ferroui_base::input::HoldingState;
use ferroui_base::layout::{HorizontalAlignment as H, Orientation, VerticalAlignment as V};
use ferroui_base::media::FontWeight;
use ferroui_controls::primitives::ScrollBarVisibility;
use ferroui_view::prelude::*;
use phone_controls::{Icon, icon};
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
            if let Some(level)=level {Border{height:6.0,width:13.0*f64::from(level)/100.0,background:if vm.data().settings.battery_saver{"#F0A30A"}else{color},horizontal_alignment:H::Left,margin:(1.0,0.0)}}
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
    let go_home = ui.clone();
    rsx! {Grid{background:"Black",column_definitions:"*,*,*",height:54.0,
        Button{name:"HardwareBack",class:"phone-flat",automation_properties_name:"Back",horizontal_alignment:H::Stretch,horizontal_content_alignment:H::Center,
            on_click:move |_|crate::back(&back),
            on_holding:move |e|if e.holding_state()==HoldingState::Started{hold.switcher_open.set(true);e.set_handled(true);},
            on_context_requested:move |e|{context.switcher_open.set(true);e.set_handled(true);},
            {icon(Icon::Back,23.0,"White")}},
        Button{name:"HardwareStart",Grid.column:1,class:"phone-flat",automation_properties_name:"Start",horizontal_alignment:H::Stretch,horizontal_content_alignment:H::Center,
            on_click:move |_|{go_home.clear_overlays();go_home.locked.set(false);home.home();},{icon(Icon::Windows,25.0,"White")}},
        Button{name:"HardwareSearch",Grid.column:2,class:"phone-flat",automation_properties_name:"Search",horizontal_alignment:H::Stretch,horizontal_content_alignment:H::Center,
            on_click:move |_|search.navigate(Route::Search),{icon(Icon::Search,22.0,"White")}}
    }}
}
pub fn lock(vm: Rc<PhoneViewModel>, ui: Rc<Session>) -> Element {
    crate::lockscreen::view(vm, ui)
}
pub fn switcher(vm: Rc<PhoneViewModel>, ui: Rc<Session>, overlay: bool) -> Element {
    let (screen_width, screen_height, _) = crate::shell::phone_size(&vm);
    // Overlays cover the status/app rows; routed pages cover only the app row.
    // The hardware navigation row remains reachable in either presentation.
    let available_height = screen_height - if overlay { 54.0 } else { 80.0 };
    let compact = available_height < 430.0;
    let header_height = if compact { 58.0 } else { 84.0 };
    let card_width = (screen_width - 48.0).min(270.0).max(120.0);
    let max_image_height = (available_height - header_height - 54.0 - 44.0 - 10.0 - 20.0).max(1.0);
    let mut cards = stack_panel()
        .orientation(Orientation::Horizontal)
        .spacing(22.0)
        .margin((24.0, 10.0))
        .vertical_alignment(V::Top);
    for app in vm.recent_apps() {
        let open_vm = vm.clone();
        let open_ui = ui.clone();
        let close_vm = vm.clone();
        let close_ui = ui.clone();
        let color = if app.color().is_empty() {
            common::accent(&vm)
        } else {
            app.color().into()
        };
        let preview = ui.previews.get(app);
        let ratio = preview
            .as_ref()
            .map(|preview| preview.logical_size.height / preview.logical_size.width)
            .unwrap_or(screen_height / screen_width);
        let image_height = (card_width * ratio).min(max_image_height);
        let content = match preview {
            Some(preview) => rsx! {Image{
                name:format!("AppPreview-{}",app.icon()),source:Some(preview.image),
                stretch:ferroui_base::media::Stretch::Uniform,is_hit_test_visible:false
            }},
            None => rsx! {Border{background:color.as_str(),
                StackPanel{vertical_alignment:V::Center,spacing:16.0,
                    {icon(common::app_icon(app),(image_height*0.24).clamp(24.0,78.0),"White")},
                    if image_height > 180.0 {TextBlock{text:app.label().to_lowercase(),font_size:28.0,foreground:"White",margin:(16.0,0.0),text_wrapping:ferroui_base::media::TextWrapping::Wrap}}
                }
            }},
        };
        cards=cards.child(rsx!{StackPanel{width:card_width,spacing:10.0,
            Grid{height:44.0,column_definitions:"*,40",clip_to_bounds:true,
                TextBlock{text:app.label(),font_size:20.0,foreground:"White",vertical_alignment:V::Center},
                Button{Grid.column:1,class:"phone-flat",automation_properties_name:format!("Close {}",app.label()),on_click:move |_|{
                    if close_vm.route().app()==Some(app){close_ui.selected.set(None);close_ui.pivot.set(0);}
                    close_ui.previews.remove(app);
                    close_ui.app_selections.borrow_mut().remove(app.label());
                    close_ui.app_pivots.borrow_mut().remove(app.label());
                    close_vm.close_app(app);
                },{icon(Icon::Close,18.0,"White")}}
            },
            Button{class:"phone-flat",height:image_height,min_height:0.0,padding:0.0,
                horizontal_content_alignment:H::Stretch,vertical_content_alignment:V::Stretch,
                automation_properties_name:format!("Switch to {}",app.label()),on_click:move |_|open_ui.open_app(&open_vm,app),
                Border{background:"Black",border_brush:"#777777",border_thickness:1.0,clip_to_bounds:true,{content}}
            }
        }});
    }
    let done = ui.clone();
    let done_vm = vm.clone();
    let rows = format!("{header_height},*,54");
    rsx! {Grid{name:"AppSwitcher",background:"#222222",row_definitions:rows.as_str(),clip_to_bounds:true,
        TextBlock{text:"recent apps",font_size:if compact{26.0}else{35.0},font_weight:FontWeight::Light,foreground:"White",margin:(22.0,8.0),vertical_alignment:V::Center},
        ScrollViewer{Grid.row:1,horizontal_scroll_bar_visibility:ScrollBarVisibility::Hidden,vertical_scroll_bar_visibility:ScrollBarVisibility::Disabled,{cards}},
        Button{Grid.row:2,class:"phone-flat",horizontal_content_alignment:H::Center,on_click:move |_|{done.switcher_open.set(false);if done_vm.route()==Route::Switcher{done_vm.back();}},TextBlock{text:"back",font_size:18.0,foreground:"White"}}
    }}
}
