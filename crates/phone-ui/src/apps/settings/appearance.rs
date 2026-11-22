//! Theme, sounds and readable display settings.
use super::update;
use crate::common::{muted, text};
use ferroui_base::layout::{HorizontalAlignment as H, Orientation};
use ferroui_view::prelude::*;
use phone_controls::{flat, toggle};
use phone_domain::ACCENTS;
use phone_state::PhoneViewModel;
use std::rc::Rc;

pub(super) fn theme(vm: Rc<PhoneViewModel>) -> Element {
    let settings = vm.data().settings.clone();
    let dark = vm.clone();
    let light = vm.clone();
    let more = vm.clone();
    let none = vm.clone();
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
    let wallpaper_options = photos
        .into_iter()
        .map(|photo| {
            let model = vm.clone();
            let source = photo.source.clone();
            flat(&photo.name, move || {
                update(&model, |value| value.wallpaper = source.clone())
            })
        })
        .collect::<Vec<_>>();
    rsx! {StackPanel{spacing:16.0,
        {muted("Background",18.0)}
        StackPanel{orientation:Orientation::Horizontal,spacing:18.0,
            {flat(if settings.dark{"● dark"}else{"○ dark"},move ||update(&dark,|value|value.dark=true))}
            {flat(if settings.dark{"○ light"}else{"● light"},move ||update(&light,|value|value.dark=false))}
        }
        {muted(&format!("Accent color · {}",settings.accent_name()),18.0)}
        {colors}
        {toggle("Show more Tiles",settings.more_tiles,move |enabled|update(&more,|value|value.more_tiles=enabled))}
        {muted("Start background",18.0)}
        {flat("none",move ||update(&none,|value|value.wallpaper.clear()))}
        for option in wallpaper_options {{option}}
    }}
}

pub(super) fn sounds(vm: Rc<PhoneViewModel>) -> Element {
    let settings = vm.data().settings.clone();
    let sound = vm.clone();
    let vibration = vm.clone();
    let volume = vm.clone();
    rsx! {StackPanel{spacing:14.0,
        {toggle("Sounds",settings.sounds,move |enabled|update(&sound,|value|value.sounds=enabled))}
        {toggle("Vibrate",settings.vibration,move |enabled|update(&vibration,|value|value.vibration=enabled))}
        {text(&format!("Volume   {}/30",settings.volume),23.0)}
        Slider{minimum:0.0,maximum:30.0,value:settings.volume as f64,small_change:1.0,large_change:5.0,
            automation_properties_name:"Volume",on_value_changed:move |event|{
                let current=volume.data().settings.volume;let next=event.new_value().round().clamp(0.0,30.0)as u8;
                if next!=current{update(&volume,|value|value.volume=next);}
            }}
        {muted("These controls change sound and feedback inside this app.",15.0)}
    }}
}

pub(super) fn display(vm: Rc<PhoneViewModel>, accessibility: bool) -> Element {
    let settings = vm.data().settings.clone();
    let model = vm.clone();
    if accessibility {
        rsx! {StackPanel{spacing:20.0,
            {text("Text size",25.0)}
            Slider{minimum:0.8,maximum:1.6,value:settings.text_scale,small_change:0.1,automation_properties_name:"Text size",
                on_value_changed:move |event|{let next=event.new_value();let current=model.data().settings.text_scale;if (current-next).abs()>0.01{update(&model,|value|value.text_scale=next);}}}
            TextBlock{text:"Make your phone easier to read",font_size:24.0*settings.text_scale,text_wrapping:ferroui_base::media::TextWrapping::Wrap}
            {muted("Use your browser or operating system's accessibility tools for screen magnification and screen reading.",15.0)}
        }}
    } else {
        rsx! {StackPanel{spacing:20.0,
            {text(&format!("Level   {}%",(settings.brightness*100.0).round()),25.0)}
            Slider{minimum:0.1,maximum:1.0,value:settings.brightness,small_change:0.1,automation_properties_name:"Brightness",
                on_value_changed:move |event|{let next=event.new_value();let current=model.data().settings.brightness;if (current-next).abs()>0.01{update(&model,|value|value.brightness=next);}}}
            {muted("Adjusts the appearance of this app.",15.0)}
        }}
    }
}
