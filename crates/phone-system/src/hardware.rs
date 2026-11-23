//! Windows Phone volume flyout. Native streams use observed device controls;
//! embedded/browser hosts retain the app's own audio preferences.
use crate::session::Session;
use ferroui_base::layout::{HorizontalAlignment as H, Orientation, VerticalAlignment as V};
use ferroui_base::media::{FontWeight, TextWrapping};
use ferroui_view::prelude::*;
use phone_controls::{icon, Icon};
use phone_sdk::host::platform::{
    ControlAccess, ControlValue, DeviceControl, DeviceRequest, HostFeature,
};
use phone_state::{HostRequest, PhoneViewModel};
use std::{cell::Cell, rc::Rc};

pub struct HardwareState {
    pub open: Signal<bool>,
    pub expanded: Signal<bool>,
    pub media: Signal<bool>,
    deadline: Cell<i64>,
    held: Cell<bool>,
    ringer_before_mute: Cell<f64>,
    media_before_mute: Cell<f64>,
}

impl Default for HardwareState {
    fn default() -> Self {
        Self::new()
    }
}

impl HardwareState {
    pub fn new() -> Self {
        Self {
            open: signal(false),
            expanded: signal(false),
            media: signal(false),
            deadline: Cell::new(0),
            held: Cell::new(false),
            ringer_before_mute: Cell::new(0.5),
            media_before_mute: Cell::new(0.5),
        }
    }
    pub fn show(&self, now: i64, media: bool) {
        if !self.open.get_untracked() {
            self.expanded.set(false);
        }
        self.media.set(media);
        self.open.set(true);
        self.touch(now);
    }
    pub fn touch(&self, now: i64) {
        self.deadline.set(now.saturating_add(6));
    }
    pub fn begin_interaction(&self, now: i64) {
        self.held.set(true);
        self.touch(now);
    }
    pub fn end_interaction(&self, now: i64) {
        if self.held.replace(false) {
            self.touch(now);
        }
    }
    pub fn close(&self) {
        self.open.set(false);
        self.expanded.set(false);
        self.held.set(false);
    }
    pub fn is_open(&self) -> bool {
        self.open.get()
    }
    pub fn tick(&self, now: i64) {
        if self.open.get_untracked() && !self.held.get() && now >= self.deadline.get() {
            self.close();
        }
    }
}

/// Hardware keys adjust the active channel and expose the same native flyout.
pub fn adjust(vm: &PhoneViewModel, state: &HardwareState, delta: i8, media: bool) {
    if vm.platform_state().supports(HostFeature::HardwareControls) {
        let control = if media {
            DeviceControl::MediaVolume
        } else {
            DeviceControl::RingerVolume
        };
        if let Some(observed) = vm.device_state().snapshot.control(control) {
            match (&observed.access, &observed.value) {
                (ControlAccess::Writable, ControlValue::Level(value)) => {
                    vm.request_device(DeviceRequest::SetControl {
                        control,
                        value: ControlValue::Level(
                            (value + f64::from(delta) * 0.05).clamp(0.0, 1.0),
                        ),
                    })
                }
                (ControlAccess::Panel | ControlAccess::Permission, _) => {
                    vm.request_device(DeviceRequest::OpenControl { control })
                }
                _ => {
                    vm.set_status("This device controls its volume through system settings.".into())
                }
            }
        }
        state.show(vm.now(), media);
        return;
    }
    let mut settings = vm.data().settings.clone();
    if media {
        settings.volume = (i16::from(settings.volume) + i16::from(delta)).clamp(0, 30) as u8;
        settings.media_muted = false;
    } else {
        let level = if settings.sounds {
            settings.ringer_volume
        } else {
            0
        };
        settings.ringer_volume = (i16::from(level) + i16::from(delta)).clamp(0, 10) as u8;
        settings.sounds = settings.ringer_volume != 0;
    }
    vm.set_settings(settings);
    state.show(vm.now(), media);
}
pub fn mute(vm: &PhoneViewModel, state: &HardwareState, media: bool) {
    if vm.platform_state().supports(HostFeature::HardwareControls) {
        let control = if media {
            DeviceControl::MediaVolume
        } else {
            DeviceControl::RingerVolume
        };
        let observed = vm.device_state().snapshot.control(control).cloned();
        if let Some(observed) = observed {
            match (observed.access, observed.value) {
                (ControlAccess::Writable, ControlValue::Level(level)) => {
                    let saved = if media {
                        &state.media_before_mute
                    } else {
                        &state.ringer_before_mute
                    };
                    let value = if level > 0.001 {
                        saved.set(level);
                        0.0
                    } else {
                        saved.get()
                    };
                    vm.request_device(DeviceRequest::SetControl {
                        control,
                        value: ControlValue::Level(value),
                    });
                }
                (ControlAccess::Panel | ControlAccess::Permission, _) => {
                    vm.request_device(DeviceRequest::OpenControl { control })
                }
                _ => vm.set_status("This device controls mute through system settings.".into()),
            }
        }
    } else {
        let mut settings = vm.data().settings.clone();
        if media {
            settings.media_muted = !settings.media_muted;
        } else {
            settings.sounds = !settings.sounds;
        }
        vm.set_settings(settings);
    }
    state.show(vm.now(), media);
}

pub fn view(vm: Rc<PhoneViewModel>, ui: Rc<Session>) -> Element {
    let settings = vm.data().settings.clone();
    let native = vm.platform_state().supports(HostFeature::HardwareControls);
    let mode = vm
        .device_state()
        .snapshot
        .control(DeviceControl::RingerMode)
        .map(|v| phone_sdk::device::value_label(&v.value))
        .unwrap_or_else(|| "sound mode".into());
    let expanded = ui.state::<crate::hardware::HardwareState>().expanded.get();
    let media = ui.state::<crate::hardware::HardwareState>().media.get();
    let toggle = ui.clone();
    let toggle_vm = vm.clone();
    let vibration = ui.clone();
    let vibration_vm = vm.clone();
    let open_settings = ui.clone();
    let settings_vm = vm.clone();
    let close = ui.clone();
    let mut channels = stack_panel().spacing(12.0);
    channels = channels.child(channel(
        vm.clone(),
        ui.clone(),
        if expanded { false } else { media },
    ));
    if expanded {
        channels = channels.child(channel(vm.clone(), ui.clone(), true));
    }
    let playing = ui.media_playing.get();
    let track = ui.media_title.get();
    let playback = ui.clone();
    let playback_vm = vm.clone();
    rsx! { Border {
        name:"PhoneVolumePanel",background:if settings.high_contrast{"#000000"}else{"#1F1F1F"},
        border_brush:"#555555",border_thickness:(0.0,0.0,0.0,1.0),padding:(18.0,14.0,18.0,12.0),
        vertical_alignment:V::Top,horizontal_alignment:H::Stretch,
        automation_properties_name:"Volume controls",
        StackPanel {spacing:10.0,
            Grid {column_definitions:"*,44",column_spacing:12.0,
                {channels}
                Button {Grid.column:1,class:"phone-flat",padding:0.0,width:44.0,height:44.0,vertical_alignment:V::Top,
                    automation_properties_name:if expanded{"Collapse volume controls"}else{"Expand volume controls"},
                    on_click:move |_|{toggle.state::<crate::hardware::HardwareState>().expanded.set(!expanded);toggle.state::<crate::hardware::HardwareState>().touch(toggle_vm.now());},
                    {icon(if expanded{Icon::ChevronUp}else{Icon::ChevronDown},24.0,"White")}
                }
            }
            if !track.is_empty() {
                Grid {column_definitions:"*,48",margin:(0.0,4.0),
                    TextBlock{text:track,font_size:19.0,foreground:"White",text_wrapping:TextWrapping::Wrap,vertical_alignment:V::Center}
                    Button {Grid.column:1,class:"phone-flat",height:44.0,padding:0.0,
                        automation_properties_name:if playing{"Pause playback"}else{"Resume playback"},
                        on_click:move |_|{
                            let playing=playback.media_playing.get_untracked();
                            playback_vm.request(HostRequest::MediaControl{action:if playing{"pause"}else{"play"}.into()});
                            playback.media_playing.set(!playing);playback.state::<crate::hardware::HardwareState>().touch(playback_vm.now());
                        },
                        {icon(if playing{Icon::Pause}else{Icon::Play},26.0,"White")}
                    }
                }
            }
            if expanded {
                StackPanel {orientation:Orientation::Horizontal,spacing:18.0,
                    Button {class:"phone-flat",content:if native{mode}else{if settings.vibration{"vibrate on"}else{"vibrate off"}.into()},foreground:"White",font_size:18.0,
                        automation_properties_name:if native{"Device sound mode"}else{"Toggle vibration"},
                        on_click:move |_|{if native{vibration_vm.activate_device_control(DeviceControl::RingerMode);}else{let mut s=vibration_vm.data().settings.clone();s.vibration=!s.vibration;vibration_vm.set_settings(s);}vibration.state::<crate::hardware::HardwareState>().touch(vibration_vm.now());}}
                    Button {class:"phone-flat",content:"settings",foreground:"White",font_size:18.0,
                        automation_properties_name:"Open sound settings",on_click:move |_|{open_settings.state::<crate::hardware::HardwareState>().close();settings_vm.navigate(phone_domain::Route::Setting("ringtones+sounds".into()));}}
                }
            }
            Button {class:"phone-flat",content:"close",foreground:"#FFFFFF",font_size:14.0,horizontal_alignment:H::Right,
                automation_properties_name:"Close volume controls",on_click:move |_|close.state::<crate::hardware::HardwareState>().close()}
        }
    }}
}

fn channel(vm: Rc<PhoneViewModel>, ui: Rc<Session>, media: bool) -> Element {
    if vm.platform_state().supports(HostFeature::HardwareControls) {
        return native_channel(vm, ui, media);
    }
    let settings = vm.data().settings.clone();
    let muted = if media {
        settings.media_muted
    } else {
        !settings.sounds
    };
    let maximum = if media { 30.0 } else { 10.0 };
    let level = if media {
        settings.volume
    } else {
        settings.ringer_volume
    };
    let displayed = if muted { 0 } else { level };
    let label = if media {
        "media + apps"
    } else {
        "ringer + notifications"
    };
    let mute_vm = vm.clone();
    let mute_ui = ui.clone();
    let slider_vm = vm.clone();
    let slider_ui = ui.clone();
    rsx! {StackPanel{spacing:1.0,
        Grid {column_definitions:"44,*,Auto",column_spacing:8.0,
            Button {class:"phone-flat",width:44.0,height:44.0,padding:0.0,
                automation_properties_name:format!("{} {label}",if muted{"Unmute"}else{"Mute"}),
                on_click:move |_|{
                    let mut s=mute_vm.data().settings.clone();
                    if media{s.media_muted=!s.media_muted;}else{s.sounds=!s.sounds;if s.sounds && s.ringer_volume==0{s.ringer_volume=1;}}
                    mute_vm.set_settings(s);mute_ui.state::<crate::hardware::HardwareState>().touch(mute_vm.now());
                },
                {icon(if muted{Icon::Mute}else{Icon::Volume},29.0,"White")}
            }
            TextBlock {Grid.column:1,text:label,font_size:18.0,foreground:"White",vertical_alignment:V::Center,text_wrapping:TextWrapping::Wrap}
            TextBlock {Grid.column:2,text:if muted{"00".into()}else{format!("{level:02}")},font_size:36.0,font_weight:FontWeight::Light,foreground:"White",vertical_alignment:V::Center}
        }
        Slider {class:"phone-slider",minimum:0.0,maximum:maximum,small_change:1.0,large_change:1.0,
            value:if muted{0.0}else{f64::from(level)},margin:(48.0,0.0,0.0,0.0),height:38.0,
            automation_properties_name:format!("{label} volume"),on_value_changed:move |event|{
                let next=event.new_value().round().clamp(0.0,maximum) as u8;
                if next == displayed { return; }
                let mut s=slider_vm.data().settings.clone();
                let old=if media{s.volume}else{s.ringer_volume};
                let currently_muted=if media{s.media_muted}else{!s.sounds};
                if (if currently_muted{0}else{old})==next{return;}
                if media{s.volume=next;s.media_muted=false;}else{s.ringer_volume=next;s.sounds=next>0;}
                slider_vm.set_settings(s);slider_ui.state::<crate::hardware::HardwareState>().touch(slider_vm.now());
            }}
    }}
}

fn native_channel(vm: Rc<PhoneViewModel>, ui: Rc<Session>, media: bool) -> Element {
    let control = if media {
        DeviceControl::MediaVolume
    } else {
        DeviceControl::RingerVolume
    };
    let observed = vm.device_state().snapshot.control(control).cloned();
    let level = observed.as_ref().and_then(|v| match v.value {
        ControlValue::Level(value) => Some(value),
        _ => None,
    });
    let writable = observed
        .as_ref()
        .is_some_and(|v| v.access == ControlAccess::Writable)
        && level.is_some();
    let panel = observed
        .as_ref()
        .is_some_and(|v| matches!(v.access, ControlAccess::Panel | ControlAccess::Permission));
    let label = if media {
        "media + apps"
    } else {
        "ringer + notifications"
    };
    let muted = level.is_some_and(|v| v <= 0.001);
    if let Some(level) = level.filter(|value| *value > 0.001) {
        let state = ui.state::<HardwareState>();
        if media {
            state.media_before_mute.set(level);
        } else {
            state.ringer_before_mute.set(level);
        }
    }
    let mute_vm = vm.clone();
    let mute_ui = ui.clone();
    let slider_vm = vm.clone();
    let slider_ui = ui.clone();
    let open_vm = vm.clone();
    let open_ui = ui.clone();
    rsx! {StackPanel{spacing:1.0,
        Grid{column_definitions:"44,*,Auto",column_spacing:8.0,
            Button{class:"phone-flat",width:44.0,height:44.0,padding:0.0,is_enabled:writable,
                automation_properties_name:format!("{} device {label}",if muted{"Unmute"}else{"Mute"}),
                on_click:move |_|{let state=mute_ui.state::<HardwareState>();let restore=if media{state.media_before_mute.get()}else{state.ringer_before_mute.get()};mute_vm.request_device(DeviceRequest::SetControl{control,value:ControlValue::Level(if muted{restore}else{0.0})});state.touch(mute_vm.now());},
                {icon(if muted{Icon::Mute}else{Icon::Volume},29.0,"White")}}
            TextBlock{Grid.column:1,text:label,font_size:18.0,foreground:"White",vertical_alignment:V::Center,text_wrapping:TextWrapping::Wrap}
            TextBlock{Grid.column:2,text:level.map(|v|format!("{}%",(v*100.0).round()as u8)).unwrap_or_else(||"—".into()),font_size:30.0,font_weight:FontWeight::Light,foreground:"White",vertical_alignment:V::Center}
        }
        if writable{Slider{class:"phone-slider",minimum:0.0,maximum:1.0,small_change:0.05,large_change:0.1,value:level.unwrap_or(0.0),margin:(48.0,0.0,0.0,0.0),height:38.0,
            automation_properties_name:format!("Device {label} volume"),on_value_changed:move |event|{let value=event.new_value().clamp(0.0,1.0);if (value-level.unwrap_or(0.0)).abs()<0.01{return;}slider_vm.request_device(DeviceRequest::SetControl{control,value:ControlValue::Level(value)});slider_ui.state::<HardwareState>().touch(slider_vm.now());}}}
        if panel{Button{class:"phone-flat",content:"open system volume",foreground:"White",font_size:18.0,
            on_click:move |_|{open_vm.request_device(DeviceRequest::OpenControl{control});open_ui.state::<HardwareState>().touch(open_vm.now());}}}
    }}
}
