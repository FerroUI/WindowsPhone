use crate::session::SystemSessionExt;
use crate::{
    app_list, common, dialogs, gestures, navigation, notifications, session::Session, start,
};
use ferroui_base::layout::{HorizontalAlignment as H, VerticalAlignment as V};
use ferroui_controls::{Application, Control};
use ferroui_view::prelude::*;
use phone_domain::Route;
use phone_state::PhoneViewModel;
use std::rc::Rc;

pub fn phone_size(vm: &PhoneViewModel) -> (f64, f64, bool) {
    let (w, h) = vm.viewport();
    let host = vm.platform_state();
    let desktop = w > 600.0
        && host.mode == phone_host::platform::HostMode::App
        && !matches!(
            host.platform,
            phone_host::HostPlatform::Android | phone_host::HostPlatform::Ios
        );
    if desktop {
        (400.0, (h - 86.0).clamp(540.0, 820.0), true)
    } else {
        (w.max(280.0), h.max(280.0), false)
    }
}

pub fn view(vm: Rc<PhoneViewModel>, ui: Rc<Session>) -> Element {
    ui.reconcile_document(&vm);
    let route = vm.route();
    let locked = ui.locked.get() || route == Route::Lock;
    let privacy = ui.state::<PrivacyState>();
    if privacy.locked.replace(locked) != locked && locked {
        vm.request(phone_state::HostRequest::StopCamera);
    }
    let alert = ui
        .state::<phone_alarm_service::AlarmState>()
        .active
        .get()
        .is_some();
    let (width, height, desktop) = phone_size(&vm);
    let page_motion = ui.state::<crate::page_motion::NavigationMotion>();
    let picker_open = phone_controls::has_open_phone_picker();
    let obscured = locked || alert || ui.preview_obscured(&route) || !vm.status().is_empty();
    // An app-local editor is excluded from previews, but it may own this
    // picker. Ordinary redraws must not dismiss the editor's staged selection.
    // Only leaving its route or an actual shell/privacy takeover does that.
    if page_motion.route_departing(&route) || ui.shell_obscured(&route) || !vm.status().is_empty() {
        phone_controls::close_phone_pickers();
    }
    // Capture the old settled page before route cleanup changes any of its
    // native bindings. Covered pages never become a retiring image.
    let visit = page_motion.prepare(
        &route,
        width,
        height,
        vm.motion_enabled() && !obscured && !picker_open,
    );
    ui.state::<crate::previews::PreviewCache>().before_render(
        &route,
        ui.switcher_open.get_untracked() || route == Route::Switcher,
        &vm.recent_entries(),
    );
    ui.route_changed(&vm, &route);
    let registry = ui.registry();
    if let Some(app) = registry.for_route(&route) {
        if let Some(payload) = ui.take_activation(app.manifest.id) {
            if let Err(error) = registry.activate(&vm, &ui, app.manifest.id, payload) {
                vm.set_status(error);
            }
        }
    }
    phone_controls::set_phone_viewport_height(height);
    let data = vm.data();
    let accent = data.settings.accent.clone();
    let dark = data.settings.dark || data.settings.high_contrast;
    phone_controls::set_high_contrast(data.settings.high_contrast);
    phone_controls::set_motion_enabled(vm.motion_enabled());
    ui.live_clock_enabled.set(vm.live_tiles_enabled());
    let text_scale = data.settings.text_scale;
    drop(data);
    phone_controls::set_text_scale(text_scale);
    if let Some(app) = Application::current() {
        phone_controls::set_palette(&app, &accent, !dark);
    }
    let background = common::bg(&vm);
    let attach_vm = vm.clone();
    let attach_ui = ui.clone();
    let dismiss_status = vm.clone();
    let outgoing_motion = page_motion.clone();
    // Restoration resets navigation's loaded-page visit even when the route
    // stays on Start. Remount that document's page so Loaded registers its new
    // capture source; ordinary revisions keep their native controls and state.
    let page_key = format!("page-{}-{route:?}", vm.document_epoch());
    let screen = rsx! {Grid{name:"PhoneScreen",width:width,height:height,row_definitions:"26,*,54",background:background.as_str(),clip_to_bounds:true,
        with:move |root| {
            attach_ui.state::<crate::previews::PreviewCache>().attach(root);
            gestures::attach(root,attach_vm.clone(),attach_ui.clone());
        },
        Border{is_enabled:!alert,{navigation::status(vm.clone(),ui.clone())}}
        Border{Grid.row:1,name:"PhonePage",key:page_key,clip_to_bounds:true,
            is_visible:!ui.locked.get(),is_enabled:!ui.locked.get() && !alert,
            on_loaded:move |event|if let Some(control)=event.source().and_then(|source|source.cast::<Control>()){
                page_motion.loaded(&control,visit);
            },
            {match &route {
                Route::Start|Route::Folder(_)=>start::view(vm.clone(),ui.clone()),
                Route::Apps=>app_list::view(vm.clone(),ui.clone()),
                Route::Search=>ui.registry().view(vm.clone(),ui.clone()),
                Route::Lock=>navigation::lock(vm.clone(),ui.clone()),
                Route::Notifications=>notifications::view(vm.clone(),ui.clone()),
                Route::Switcher=>navigation::switcher(vm.clone(),ui.clone(),false),
                _=>ui.registry().view(vm.clone(),ui.clone()),
            }}
        }
        Border{Grid.row:1,name:"PhoneOutgoingPage",clip_to_bounds:true,
            opacity:0.0,is_hit_test_visible:false,is_enabled:false,
            with:move |root|outgoing_motion.attach_outgoing(root),
            Image{name:"PhoneOutgoingImage",stretch:ferroui_base::media::Stretch::Fill,
                is_hit_test_visible:false}
        }
        Border{Grid.row:2,{navigation::bar(vm.clone(),ui.clone())}}
        if !locked && ui.switcher_open.get(){Border{Grid.row_span:2,background:background.as_str(),{navigation::switcher(vm.clone(),ui.clone(),true)}}}
        if ui.locked.get(){Border{Grid.row_span:3,{navigation::lock(vm.clone(),ui.clone())}}}
        if ui.notification_open.get(){Border{Grid.row_span:2,vertical_alignment:if ui.notification_peek.get(){V::Top}else{V::Stretch},
            height:if ui.notification_peek.get(){190.0}else{f64::NAN},background:"#1F1F1F",
            on_loaded:move |event|if let Some(control)=event.source().and_then(|source|source.cast::<Control>()){phone_controls::reveal(&control);},
            {notifications::view(vm.clone(),ui.clone())}
        }}
        if !locked && !alert && ui.dialog.get().is_some(){Border{Grid.row_span:2,background:background.as_str(),{dialogs::view(vm.clone(),ui.clone())}}}
        if !locked && !vm.status().is_empty(){Border{Grid.row:1,vertical_alignment:V::Top,background:accent.as_str(),padding:(16.0,12.0),
            Grid{column_definitions:"*,36",column_spacing:8.0,
                TextBlock{text:vm.status(),foreground:"White",font_size:15.0,text_wrapping:ferroui_base::media::TextWrapping::Wrap},
                Border{Grid.column:1,{phone_controls::flat("×",move||dismiss_status.clear_status())}}
            }
        }}
        if alert {Border{Grid.row_span:2,{crate::alarm::view(vm.clone(),ui.clone())}}}
        if ui.state::<crate::hardware::HardwareState>().open.get(){Border{Grid.row_span:3,vertical_alignment:V::Top,
            on_loaded:move |event|if let Some(control)=event.source().and_then(|source|source.cast::<Control>()){phone_controls::reveal(&control);},
            {crate::hardware::view(vm.clone(),ui.clone())}
        }}
    }};
    ui.state::<crate::previews::PreviewCache>().presented(
        &route,
        ui.preview_obscured(&route) || !vm.status().is_empty(),
        ui.switcher_open.get_untracked() || route == Route::Switcher,
    );
    if desktop {
        let up = vm.clone();
        let down = vm.clone();
        let power = vm.clone();
        let camera = vm.clone();
        rsx! {Grid{name:"WindowsPhoneApp",background:"#151619",focusable:true,
            Border{width:width+24.0,height:height+58.0,background:"#080809",border_brush:"#393B3F",border_thickness:1.0,corner_radius:26.0,horizontal_alignment:H::Center,vertical_alignment:V::Center,
                Grid{row_definitions:"31,*,25",
                    Border{width:52.0,height:4.0,corner_radius:2.0,background:"#3A3A3C",horizontal_alignment:H::Center,vertical_alignment:V::Center},
                    Border{Grid.row:1,horizontal_alignment:H::Center,{screen}},
                    Border{Grid.row:2,width:24.0,height:3.0,background:"#202022",horizontal_alignment:H::Center,vertical_alignment:V::Center}
                }
            }
            Grid{width:width+42.0,height:height+58.0,horizontal_alignment:H::Center,vertical_alignment:V::Center,is_hit_test_visible:true,
                StackPanel{horizontal_alignment:H::Right,vertical_alignment:V::Top,margin:(0.0,115.0,0.0,0.0),spacing:3.0,
                    Button{class:"phone-flat",width:12.0,height:44.0,min_width:0.0,min_height:0.0,padding:0.0,background:"#3A3A3C",automation_properties_name:"Volume up",on_click:move |_|crate::hardware_key(&up,"volumeUp")},
                    Button{class:"phone-flat",width:12.0,height:44.0,min_width:0.0,min_height:0.0,padding:0.0,background:"#3A3A3C",automation_properties_name:"Volume down",on_click:move |_|crate::hardware_key(&down,"volumeDown")},
                    Button{class:"phone-flat",width:12.0,height:38.0,min_width:0.0,min_height:0.0,padding:0.0,margin:(0.0,40.0,0.0,0.0),background:"#3A3A3C",automation_properties_name:"Power and lock",on_click:move |_|crate::hardware_key(&power,"power")},
                    Button{class:"phone-flat",width:12.0,height:44.0,min_width:0.0,min_height:0.0,padding:0.0,margin:(0.0,85.0,0.0,0.0),background:"#3A3A3C",automation_properties_name:"Camera button",on_click:move |_|crate::hardware_key(&camera,"camera")}
                }
            }
        }}
    } else {
        rsx! {Grid{name:"WindowsPhoneApp",background:background.as_str(),{screen}}}
    }
}

#[cfg(test)]
mod tests;

#[derive(Default)]
struct PrivacyState {
    locked: std::cell::Cell<bool>,
}
