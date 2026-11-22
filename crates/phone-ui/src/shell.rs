use crate::{
    app_list, apps, common, dialogs, gestures, navigation, notifications, session::Session, start,
};
use ferroui_base::layout::{HorizontalAlignment as H, VerticalAlignment as V};
use ferroui_controls::Application;
use ferroui_view::prelude::*;
use phone_domain::Route;
use phone_state::PhoneViewModel;
use std::rc::Rc;

pub fn phone_size(vm: &PhoneViewModel) -> (f64, f64, bool) {
    let (w, h) = vm.viewport();
    let desktop = w > 600.0;
    if desktop {
        (400.0, (h - 86.0).clamp(540.0, 820.0), true)
    } else {
        (w.max(280.0), h.max(280.0), false)
    }
}

pub fn view(vm: Rc<PhoneViewModel>, ui: Rc<Session>) -> Element {
    let route = vm.route();
    ui.route_changed(&route);
    let (width, height, desktop) = phone_size(&vm);
    let data = vm.data();
    let accent = data.settings.accent.clone();
    let dark = data.settings.dark;
    drop(data);
    if let Some(app) = Application::current() {
        phone_controls::set_palette(&app, &accent, !dark);
    }
    let background = common::bg(&vm);
    let attach_vm = vm.clone();
    let attach_ui = ui.clone();
    let dismiss_status=vm.clone();
    let screen = rsx! {Grid{name:"PhoneScreen",width:width,height:height,row_definitions:"26,*,54",background:background.as_str(),clip_to_bounds:true,
        with:move |root| {
            gestures::attach(root,attach_vm.clone(),attach_ui.clone());
        },
        {navigation::status(vm.clone(),ui.clone())}
        Border{Grid.row:1,clip_to_bounds:true,
            {match &route {
                Route::Start|Route::Folder(_)=>start::view(vm.clone(),ui.clone()),
                Route::Apps=>app_list::view(vm.clone(),ui.clone()),
                Route::Search=>apps::services::view(vm.clone(),ui.clone(),phone_domain::AppId::Cortana),
                Route::Lock=>navigation::lock(vm.clone(),ui.clone()),
                Route::Notifications=>notifications::view(vm.clone(),ui.clone()),
                Route::Switcher=>navigation::switcher(vm.clone(),ui.clone()),
                _=>apps::view(vm.clone(),ui.clone()),
            }}
        }
        Border{Grid.row:2,{navigation::bar(vm.clone(),ui.clone())}}
        if ui.notification_open.get(){Border{Grid.row_span:3,background:background.as_str(),{notifications::view(vm.clone(),ui.clone())}}}
        if ui.switcher_open.get(){Border{Grid.row_span:3,background:background.as_str(),{navigation::switcher(vm.clone(),ui.clone())}}}
        if ui.locked.get(){Border{Grid.row_span:3,{navigation::lock(vm.clone(),ui.clone())}}}
        if ui.dialog.get().is_some(){Border{Grid.row_span:3,background:background.as_str(),{dialogs::view(vm.clone(),ui.clone())}}}
        if !vm.status().is_empty(){Border{Grid.row:1,vertical_alignment:V::Top,background:accent.as_str(),padding:(16.0,12.0),
            Grid{column_definitions:"*,36",column_spacing:8.0,
                TextBlock{text:vm.status(),foreground:"White",font_size:15.0,text_wrapping:ferroui_base::media::TextWrapping::Wrap},
                Border{Grid.column:1,{phone_controls::flat("×",move||dismiss_status.clear_status())}}
            }
        }}
    }};
    if desktop {
        rsx! {Grid{name:"WindowsPhoneApp",background:"#151619",focusable:true,
            Border{width:width+24.0,height:height+58.0,background:"#080809",border_brush:"#393B3F",border_thickness:1.0,corner_radius:26.0,horizontal_alignment:H::Center,vertical_alignment:V::Center,
                Grid{row_definitions:"31,*,25",
                    Border{width:52.0,height:4.0,corner_radius:2.0,background:"#3A3A3C",horizontal_alignment:H::Center,vertical_alignment:V::Center},
                    Border{Grid.row:1,horizontal_alignment:H::Center,{screen}},
                    Border{Grid.row:2,width:24.0,height:3.0,background:"#202022",horizontal_alignment:H::Center,vertical_alignment:V::Center}
                }
            }
        }}
    } else {
        rsx! {Grid{name:"WindowsPhoneApp",background:background.as_str(),{screen}}}
    }
}
