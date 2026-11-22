use crate::{
    common::{muted, page, row, scroll, text, title},
    session::Session,
};
use ferroui_base::layout::{HorizontalAlignment as H, Orientation};
use ferroui_view::prelude::*;
use phone_controls::{action, flat, pivot, toggle, Icon};
use phone_domain::{AppId, Route, Settings, ACCENTS};
use phone_state::{HostRequest, PhoneViewModel};
use std::rc::Rc;

pub fn view(vm: Rc<PhoneViewModel>, ui: Rc<Session>) -> Element {
    if let Route::Setting(name) = vm.route() {
        let content = match name.as_str() {
            "start+theme" => theme(vm.clone()),
            "ringtones+sounds" | "music+videos" => sounds(vm.clone()),
            "notifications+actions" => notifications(vm.clone()),
            "lock screen" => lock(vm.clone(), ui.clone()),
            "brightness" => display(vm.clone(), false),
            "ease of access" => display(vm.clone(), true),
            "date+time" => date_time(vm.clone()),
            "backup" => backup(vm.clone()),
            "about" => about(vm.clone(), ui.clone()),
            "photos+camera" => photos(vm.clone()),
            "Internet Explorer" => browser(vm.clone()),
            "messaging" => messaging(vm.clone()),
            "maps" => location(vm.clone()),
            _ => radio(vm.clone(), &name),
        };
        return page(
            "settings",
            scroll(
                rsx! {StackPanel{spacing:12.0,{title(&name)}Border{margin:(20.0,0.0,20.0,24.0),{content}}}},
            ),
            Element::Empty,
        );
    }
    let p = ui.pivot.get().min(1);
    let system = ui.pivot;
    let applications = ui.pivot;
    let settings = vm.data().settings.clone();
    let items = if p == 0 {
        vec![
            (
                "start+theme",
                format!(
                    "{}, {}",
                    if settings.dark { "dark" } else { "light" },
                    settings.accent_name()
                ),
            ),
            (
                "ringtones+sounds",
                if settings.sounds {
                    "sounds on".into()
                } else {
                    "silent".into()
                },
            ),
            (
                "notifications+actions",
                if settings.notifications {
                    "on".into()
                } else {
                    "off".into()
                },
            ),
            ("lock screen", format!("{} seconds", settings.lock_timeout)),
            ("Wi-Fi", on_off(settings.wi_fi).into()),
            ("Bluetooth", on_off(settings.bluetooth).into()),
            ("flight mode", on_off(settings.airplane).into()),
            (
                "screen rotation",
                if settings.rotation_lock {
                    "locked".into()
                } else {
                    "unlocked".into()
                },
            ),
            (
                "brightness",
                format!("{}%", (settings.brightness * 100.0).round()),
            ),
            ("battery saver", on_off(settings.battery_saver).into()),
            (
                "date+time",
                if settings.time_24h {
                    "24-hour clock".into()
                } else {
                    "12-hour clock".into()
                },
            ),
            (
                "ease of access",
                format!("text size {}%", (settings.text_scale * 100.0).round()),
            ),
            ("backup", "apps, settings and local content".into()),
            ("about", settings.phone_name.clone()),
        ]
    } else {
        vec![
            ("Internet Explorer", "favorites and browsing".into()),
            ("maps", on_off(settings.location).into()),
            ("messaging", "notifications and messages".into()),
            ("music+videos", "volume and playback".into()),
            ("photos+camera", "photos and camera".into()),
        ]
    };
    let rows = items
        .into_iter()
        .map(|(name, description)| {
            let model = vm.clone();
            row(name, &description, move || {
                model.navigate(Route::Setting(name.into()))
            })
        })
        .collect::<Vec<_>>();
    let body = rsx! {StackPanel{
        StackPanel{orientation:Orientation::Horizontal,margin:(20.0,0.0,0.0,10.0),
            {pivot("system",p==0,move ||system.set(0))}
            {pivot("applications",p==1,move ||applications.set(1))}
        }
        for item in rows {{item}}
    }};
    page("settings", scroll(body), Element::Empty)
}

fn theme(vm: Rc<PhoneViewModel>) -> Element {
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

fn sounds(vm: Rc<PhoneViewModel>) -> Element {
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

fn notifications(vm: Rc<PhoneViewModel>) -> Element {
    let settings = vm.data().settings.clone();
    let enabled = vm.clone();
    let clear = vm.clone();
    rsx! {StackPanel{spacing:16.0,
        {toggle("Show notifications",settings.notifications,move |value|update(&enabled,|settings|settings.notifications=value))}
        {text("Choose your quick actions",23.0)}
        {muted("Wi-Fi · Bluetooth · flight mode · rotation lock",17.0)}
        {flat("clear all notifications",move ||clear.dispatch(phone_domain::Action::ClearNotifications))}
        {muted("Notification settings apply to this phone experience.",15.0)}
    }}
}

fn lock(vm: Rc<PhoneViewModel>, ui: Rc<Session>) -> Element {
    let timeout = vm.data().settings.lock_timeout;
    let options = [
        ("30 seconds", 30),
        ("1 minute", 60),
        ("3 minutes", 180),
        ("5 minutes", 300),
        ("never", 0),
    ];
    let choices = options
        .into_iter()
        .map(|(label, seconds)| {
            let model = vm.clone();
            flat(
                &format!("{} {label}", if timeout == seconds { "●" } else { "○" }),
                move || update(&model, |value| value.lock_timeout = seconds),
            )
        })
        .collect::<Vec<_>>();
    rsx! {StackPanel{spacing:12.0,
        {muted("Screen times out after",18.0)}
        for choice in choices {{choice}}
        {flat("lock now",move ||ui.locked.set(true))}
        {muted("This locks the app view. Your device's own security settings remain separate.",15.0)}
    }}
}

fn display(vm: Rc<PhoneViewModel>, accessibility: bool) -> Element {
    let settings = vm.data().settings.clone();
    let model = vm.clone();
    if accessibility {
        rsx! {StackPanel{spacing:20.0,
            {text("Text size",25.0)}
            Slider{minimum:0.8,maximum:1.6,value:settings.text_scale,small_change:0.1,automation_properties_name:"Text size",
                on_value_changed:move |event|{let next=event.new_value();if (model.data().settings.text_scale-next).abs()>0.01{update(&model,|value|value.text_scale=next);}}}
            TextBlock{text:"Make your phone easier to read",font_size:24.0*settings.text_scale,text_wrapping:ferroui_base::media::TextWrapping::Wrap}
            {muted("Use your browser or operating system's accessibility tools for screen magnification and screen reading.",15.0)}
        }}
    } else {
        rsx! {StackPanel{spacing:20.0,
            {text(&format!("Level   {}%",(settings.brightness*100.0).round()),25.0)}
            Slider{minimum:0.1,maximum:1.0,value:settings.brightness,small_change:0.1,automation_properties_name:"Brightness",
                on_value_changed:move |event|{let next=event.new_value();if (model.data().settings.brightness-next).abs()>0.01{update(&model,|value|value.brightness=next);}}}
            {muted("Adjusts the appearance of this app.",15.0)}
        }}
    }
}

fn date_time(vm: Rc<PhoneViewModel>) -> Element {
    let settings = vm.data().settings.clone();
    let model = vm.clone();
    let (year, month, day) = phone_domain::date_parts(vm.now());
    rsx! {StackPanel{spacing:18.0,
        {text(&format!("{} {}",day,phone_domain::month_name(month)),32.0)}
        {muted(&year.to_string(),22.0)}
        {text(&phone_domain::time_text(vm.now(),settings.time_24h),46.0)}
        {toggle("24-hour clock",settings.time_24h,move |enabled|update(&model,|value|value.time_24h=enabled))}
        {muted("Date and time are supplied by your device.",16.0)}
    }}
}

fn backup(vm: Rc<PhoneViewModel>) -> Element {
    let data = vm.data();
    let detail = format!(
        "{} contacts\n{} notes\n{} appointments\n{} photos",
        data.contacts.len(),
        data.notes.len(),
        data.events.len(),
        data.photos.len()
    );
    drop(data);
    let export = vm.clone();
    let import = vm.clone();
    rsx! {StackPanel{spacing:20.0,
        {text("apps+settings",30.0)}
        {muted(&detail,18.0)}
        {flat("download a backup",move ||export.request(HostRequest::ExportBackup))}
        {flat("restore from a backup",move ||import.request(HostRequest::ImportBackup))}
        {muted("A backup includes this app's settings, tile layout and local data. Keep the downloaded file somewhere safe. Restoring replaces the current local data.",16.0)}
    }}
}

fn about(vm: Rc<PhoneViewModel>, ui: Rc<Session>) -> Element {
    let name = vm.data().settings.phone_name.clone();
    let model = vm.clone();
    let rename = ui.clone();
    let link = vm.clone();
    rsx! {StackPanel{spacing:16.0,
        {muted("Name",17.0)}{text(&name,28.0)}
        {flat("edit name",move ||{
            let current=model.data().settings.phone_name.clone();let model=model.clone();let error=rename.form_error;
            rename.form("phone name",vec![("name",current)],Rc::new(move |values|{
                let name=values.first().cloned().unwrap_or_default().trim().to_owned();
                if name.is_empty(){error.set("Enter a phone name.".into());return;}
                update(&model,|value|value.phone_name=name);
            }));
        })}
        {muted("Software",17.0)}{text("Windows Phone 8.1 experience",25.0)}
        {muted("An independent recreation built with FerroUI and Rust. This is an app, with local data and browser integrations; it does not run the Windows Phone operating system.",16.0)}
        {flat("project + source",move ||link.request(HostRequest::OpenUrl{url:"https://github.com/FerroUI/WindowsPhone".into()}))}
    }}
}

fn radio(vm: Rc<PhoneViewModel>, name: &str) -> Element {
    let settings = vm.data().settings.clone();
    let (label, value, field): (&str, bool, fn(&mut Settings, bool)) = match name {
        "Wi-Fi" => ("Wi-Fi networking", settings.wi_fi, |s, v| s.wi_fi = v),
        "Bluetooth" => ("Bluetooth", settings.bluetooth, |s, v| s.bluetooth = v),
        "flight mode" => ("Status", settings.airplane, |s, v| s.airplane = v),
        "screen rotation" => ("Rotation lock", settings.rotation_lock, |s, v| {
            s.rotation_lock = v
        }),
        "battery saver" => ("Battery Saver", settings.battery_saver, |s, v| {
            s.battery_saver = v
        }),
        _ => ("Location", settings.location, |s, v| s.location = v),
    };
    let model = vm.clone();
    rsx! {StackPanel{spacing:18.0,
        {toggle(label,value,move |enabled|update(&model,|settings|field(settings,enabled)))}
        {muted(if name=="battery saver"{"Reduces live tile animation in this app."}else{"Changes this app's phone profile. To manage your physical device, use its system settings."},16.0)}
    }}
}

fn photos(vm: Rc<PhoneViewModel>) -> Element {
    let import = vm.clone();
    let capture = vm.clone();
    let open = vm.clone();
    rsx! {StackPanel{spacing:14.0,
        {flat("choose photos",move ||import.request(HostRequest::ImportPhotos))}
        {flat("open camera",move ||capture.navigate(Route::App(AppId::Camera)))}
        {flat("open Photos",move ||open.navigate(Route::App(AppId::Photos)))}
        {muted("Pictures stay in this browser unless you share or export them.",16.0)}
    }}
}

fn browser(vm: Rc<PhoneViewModel>) -> Element {
    let count = vm.data().bookmarks.len();
    let open = vm.clone();
    rsx! {StackPanel{spacing:18.0,
        {text(&format!("{count} favorites"),27.0)}
        {flat("open Internet Explorer",move ||open.navigate(Route::App(AppId::InternetExplorer)))}
        {muted("Web addresses open with your browser. Website security settings determine whether a page can be embedded.",16.0)}
    }}
}

fn messaging(vm: Rc<PhoneViewModel>) -> Element {
    let enabled = vm.data().settings.notifications;
    let model = vm.clone();
    let open = vm.clone();
    rsx! {StackPanel{spacing:18.0,
        {toggle("Notifications",enabled,move |value|update(&model,|settings|settings.notifications=value))}
        {flat("open messaging",move ||open.navigate(Route::App(AppId::Messaging)))}
        {muted("Sending a text opens your device's messaging app. Local drafts do not indicate delivery.",16.0)}
    }}
}

fn location(vm: Rc<PhoneViewModel>) -> Element {
    let value = vm.data().settings.location;
    let model = vm.clone();
    let locate = vm.clone();
    rsx! {StackPanel{spacing:18.0,
        {toggle("Location services",value,move |enabled|update(&model,|settings|settings.location=enabled))}
        {action(Icon::Location,"find my location",move ||locate.request(HostRequest::Locate))}
        {muted("Your browser asks for permission before sharing your location.",16.0)}
    }}
}

fn on_off(value: bool) -> &'static str {
    if value {
        "on"
    } else {
        "off"
    }
}
fn update(vm: &PhoneViewModel, change: impl FnOnce(&mut Settings)) {
    let mut settings = vm.data().settings.clone();
    change(&mut settings);
    vm.set_settings(settings);
}
