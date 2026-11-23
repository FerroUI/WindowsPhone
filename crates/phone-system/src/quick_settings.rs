use crate::session::Session;
use ferroui_base::layout::HorizontalAlignment as H;
use ferroui_view::prelude::*;
use phone_controls::{icon, Icon};
use phone_domain::{AppId, Route};
use phone_sdk::{device, host::platform::*};
use phone_state::PhoneViewModel;
use std::rc::Rc;
struct State {
    expanded: Signal<bool>,
}
impl Default for State {
    fn default() -> Self {
        Self {
            expanded: signal(false),
        }
    }
}
pub fn view(vm: Rc<PhoneViewModel>, ui: Rc<Session>, peek: bool) -> Element {
    let settings = vm.data().settings.clone();
    let native = vm.platform_state().supports(HostFeature::HardwareControls);
    let state = ui.state::<State>();
    let expanded = state.expanded.get() && !peek;
    let snapshot = vm.device_state().snapshot;
    let mut keys = settings.quick_actions.to_vec();
    if expanded {
        for control in DeviceControl::ALL {
            if snapshot
                .control(control)
                .is_some_and(|v| v.access != ControlAccess::Unavailable)
                && !keys.iter().any(|key| key == control.key())
            {
                keys.push(control.key().into());
            }
        }
        if !keys.iter().any(|k| k == "camera") {
            keys.push("camera".into());
        }
    }
    let mut grid = grid()
        .column_definitions("*,*,*,*")
        .row_definitions(&vec!["96"; keys.len().div_ceil(4)].join(","))
        .column_spacing(3.0)
        .row_spacing(6.0);
    for (index, key) in keys.into_iter().enumerate() {
        let control = DeviceControl::from_key(&key);
        let observed = control.and_then(|v| snapshot.control(v));
        let (kind, label) =
            control.map_or((Icon::Camera, "camera"), |v| (device::icon(v), v.label()));
        let enabled = if native {
            matches!(observed.map(|v| &v.value), Some(ControlValue::Toggle(true)))
        } else {
            match key.as_str() {
                "wi_fi" => settings.wi_fi,
                "bluetooth" => settings.bluetooth,
                "airplane" => settings.airplane,
                "rotation_lock" => settings.rotation_lock,
                "location" => settings.location,
                _ => false,
            }
        };
        let available = key == "camera"
            || !native
            || observed.is_some_and(|v| {
                matches!(
                    v.access,
                    ControlAccess::Writable | ControlAccess::Panel | ControlAccess::Permission
                )
            });
        let action_name = if native {
            format!("Open device {label}")
        } else if matches!(key.as_str(), "brightness" | "camera") {
            format!("Open {label}")
        } else {
            format!("Toggle {label}")
        };
        let model = vm.clone();
        let session = ui.clone();
        let page_vm = vm.clone();
        let page_ui = ui.clone();
        let info = observed
            .map(|v| device::value_label(&v.value))
            .unwrap_or_default();
        grid=grid.child(rsx!{Button{Grid.column:(index%4)as i32,Grid.row:(index/4)as i32,class:"phone-flat",height:96.0,padding:0.0,
            is_enabled:available,horizontal_content_alignment:H::Stretch,automation_properties_name:action_name,
            on_click:move |_|{
                if key=="camera"{session.locked.set(false);session.notification_open.set(false);model.navigate(Route::App(AppId::Camera));}
                else if let Some(control)=control.filter(|_|native){
                    if matches!(control,DeviceControl::Brightness|DeviceControl::RingerVolume|DeviceControl::MediaVolume|DeviceControl::AlarmVolume|DeviceControl::ScreenTimeout){session.notification_open.set(false);model.navigate(Route::Setting(format!("device/{}",control.key())));}
                    else {model.activate_device_control(control);}
                }else if key=="brightness"{session.locked.set(false);session.notification_open.set(false);model.navigate(Route::Setting("brightness".into()));}
                else{model.toggle_setting(&key);}
            },
            on_context_requested:move|event|{if let Some(control)=control.filter(|_|native){page_ui.notification_open.set(false);page_vm.navigate(Route::Setting(format!("device/{}",control.key())));}event.set_handled(true);},
            StackPanel{spacing:5.0,
                Border{height:50.0,background:if enabled{settings.accent.as_str()}else{"#383838"},{icon(kind,25.0,"White")}}
                TextBlock{text:label,font_size:12.0,foreground:"White",horizontal_alignment:H::Center,text_wrapping:ferroui_base::media::TextWrapping::Wrap}
                if native&&!info.is_empty(){TextBlock{text:info,font_size:10.0,foreground:"#BBBBBB",horizontal_alignment:H::Center}}
            }
        }});
    }
    let toggle = state.expanded;
    let refresh = vm.clone();
    rsx! {StackPanel{margin:(12.0,14.0,12.0,8.0),spacing:4.0,
        ScrollViewer{max_height:if expanded{(vm.viewport().1*0.43).min(340.0)}else{102.0},horizontal_scroll_bar_visibility:ferroui_controls::primitives::ScrollBarVisibility::Disabled,{grid}}
        if native&&!peek{Button{class:"phone-flat",height:32.0,horizontal_alignment:H::Right,automation_properties_name:if expanded{"Collapse quick actions"}else{"Expand quick actions"},
            on_click:move |_|{toggle.set(!toggle.get_untracked());refresh.refresh_device();},
            TextBlock{text:if expanded{"collapse"}else{"expand"},font_size:15.0,foreground:"White"}}}
    }}
}
