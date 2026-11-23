//! Theme, sounds and readable display settings.
use super::update;
use ferroui_base::layout::{HorizontalAlignment as H, Orientation};
use ferroui_view::prelude::*;
use phone_controls::{flat, toggle};
use phone_domain::ACCENTS;
use phone_sdk::{
    common::{muted, text},
    session::Session,
};
use phone_state::PhoneViewModel;
use std::rc::Rc;

pub(super) fn theme(vm: Rc<PhoneViewModel>) -> Element {
    let settings = vm.data().settings.clone();
    let dark = vm.clone();
    let light = vm.clone();
    let more = vm.clone();
    let none = vm.clone();
    let live = vm.clone();
    let mut colors = grid()
        .column_definitions("*,*,*,*,*")
        .row_definitions("48,48,48,48")
        .row_spacing(8.0)
        .column_spacing(8.0);
    for (index, (name, color)) in ACCENTS.into_iter().enumerate() {
        let model = vm.clone();
        let selected = settings.accent.eq_ignore_ascii_case(color);
        colors=colors.child(rsx!{Button{Grid.row:(index/5)as i32,Grid.column:(index%5)as i32,class:"phone-flat",
            background:color,content:if selected{"✓"}else{""},foreground:"White",font_size:26.0,
            horizontal_alignment:H::Stretch,padding:0.0,automation_properties_name:format!("accent color {name}"),
            on_click:move |_|update(&model,|value|value.accent=color.into())
        }});
    }
    let photos = vm.data().photos.clone();
    let background = vm.clone();
    let wallpaper_picker = super::photo_picker::view(
        photos,
        280.0,
        Rc::new(move |source| update(&background, |settings| settings.wallpaper = source)),
    );
    rsx! {StackPanel{spacing:16.0,
        {muted("Background",18.0)}
        StackPanel{orientation:Orientation::Horizontal,spacing:18.0,
            {flat(if settings.dark{"● dark"}else{"○ dark"},move ||update(&dark,|value|value.dark=true))}
            {flat(if settings.dark{"○ light"}else{"● light"},move ||update(&light,|value|value.dark=false))}
        }
        {muted(&format!("Accent color · {}",settings.accent_name()),18.0)}
        {colors}
        {toggle("Show more Tiles",settings.more_tiles,move |enabled|update(&more,|value|value.more_tiles=enabled))}
        {toggle("Live Tiles",settings.live_tiles,move |enabled|update(&live,|value|value.live_tiles=enabled))}
        {muted("Start background",18.0)}
        {flat("none",move ||update(&none,|value|value.wallpaper.clear()))}
        {wallpaper_picker}
    }}
}

pub(super) fn sounds(vm: Rc<PhoneViewModel>, _ui: Rc<Session>) -> Element {
    if vm
        .platform_state()
        .supports(phone_sdk::host::platform::HostFeature::HardwareControls)
    {
        use phone_sdk::host::platform::DeviceControl;
        let mut body = stack_panel().spacing(22.0);
        for control in [
            DeviceControl::RingerMode,
            DeviceControl::RingerVolume,
            DeviceControl::MediaVolume,
            DeviceControl::AlarmVolume,
            DeviceControl::AudioOutput,
        ] {
            if vm.device_state().snapshot.control(control).is_some() {
                body = body
                    .child(text(control.label(), 24.0))
                    .child(phone_sdk::device::control(vm.clone(), control));
            }
        }
        let feedback = vm.clone();
        let vibration = vm.data().settings.vibration;
        return body
            .child(toggle("App touch feedback", vibration, move |value| {
                update(&feedback, |s| s.vibration = value)
            }))
            .into();
    }
    let settings = vm.data().settings.clone();
    let sound = vm.clone();
    let vibration = vm.clone();
    let ringer = vm.clone();
    let media = vm.clone();
    let mute_media = vm.clone();
    let panel = vm.clone();

    let notifications = vm.clone();
    rsx! {StackPanel{spacing:14.0,
        {toggle("Ringer",settings.sounds,move |enabled|update(&sound,|value|{value.sounds=enabled;if enabled&&value.ringer_volume==0{value.ringer_volume=1;}}))}
        {toggle("Vibrate",settings.vibration,move |enabled|update(&vibration,|value|value.vibration=enabled))}
        {text(&format!("Ringer + notifications   {}/10",settings.ringer_volume),23.0)}
        Slider{class:"phone-slider",minimum:0.0,maximum:10.0,value:settings.ringer_volume as f64,small_change:1.0,large_change:1.0,
            automation_properties_name:"Ringer volume",on_value_changed:move |event|{
                let current=ringer.data().settings.ringer_volume;let next=event.new_value().round().clamp(0.0,10.0)as u8;
                if next!=current{update(&ringer,|value|{value.ringer_volume=next;value.sounds=next>0;});}
            }}
        {text(&format!("Media + apps   {}/30",settings.volume),23.0)}
        Slider{class:"phone-slider",minimum:0.0,maximum:30.0,value:settings.volume as f64,small_change:1.0,large_change:5.0,
            automation_properties_name:"Media volume",on_value_changed:move |event|{
                let current=media.data().settings.volume;let next=event.new_value().round().clamp(0.0,30.0)as u8;
                if next!=current{update(&media,|value|{value.volume=next;value.media_muted=false;});}
            }}
        {toggle("Mute media",settings.media_muted,move |enabled|update(&mute_media,|value|value.media_muted=enabled))}
        {flat("show volume controls",move ||phone_sdk::show_volume(&panel))}
        {flat("manage app notifications",move ||notifications.navigate(phone_domain::Route::Setting("notifications+actions".into())))}
        {muted("These controls change sound and feedback inside this app.",15.0)}
    }}
}

pub(super) fn display(vm: Rc<PhoneViewModel>, accessibility: bool) -> Element {
    let settings = vm.data().settings.clone();
    let model = vm.clone();
    if accessibility {
        let contrast = vm.clone();
        let motion = vm.clone();
        rsx! {StackPanel{spacing:20.0,
            {text("Text size",25.0)}
            Slider{class:"phone-slider",minimum:0.8,maximum:1.6,value:settings.text_scale,small_change:0.1,automation_properties_name:"Text size",
                on_value_changed:move |event|{let next=event.new_value();let current=model.data().settings.text_scale;if (current-next).abs()>0.01{update(&model,|value|value.text_scale=next);}}}
            TextBlock{text:"Make your phone easier to read",font_size:24.0*settings.text_scale,text_wrapping:ferroui_base::media::TextWrapping::Wrap}
            {toggle("High contrast",settings.high_contrast,move |enabled|update(&contrast,|value|value.high_contrast=enabled))}
            {muted("Makes text and controls easier to see and removes decorative backgrounds.",16.0)}
            {toggle("Reduce motion",settings.reduced_motion,move |enabled|update(&motion,|value|value.reduced_motion=enabled))}
            {muted("Uses still content instead of tile motion and page transitions.",16.0)}
            {muted("Use your browser or operating system's accessibility tools for screen magnification and screen reading.",15.0)}
        }}
    } else {
        rsx! {StackPanel{spacing:20.0,
            {text(&format!("Level   {}%",(settings.brightness*100.0).round()),25.0)}
            Slider{class:"phone-slider",minimum:0.1,maximum:1.0,value:settings.brightness,small_change:0.1,automation_properties_name:"Brightness",
                on_value_changed:move |event|{let next=event.new_value();let current=model.data().settings.brightness;if (current-next).abs()>0.01{update(&model,|value|value.brightness=next);}}}
            {muted("Adjusts the appearance of this app.",15.0)}
        }}
    }
}
