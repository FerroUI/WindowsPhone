use crate::{common::*, session::Session};
use ferroui_base::layout::{HorizontalAlignment as H, Orientation};
use ferroui_view::prelude::*;
use phone_controls::{Icon, action, app_bar, icon, input, pivot, toggle};
use phone_domain::{AppId, Route};
use phone_state::{HostRequest, PhoneViewModel};
use std::rc::Rc;

pub fn view(vm: Rc<PhoneViewModel>, ui: Rc<Session>, app: AppId) -> Element {
    match app {
        AppId::Maps => maps(vm, ui),
        AppId::Weather => weather(vm, ui),
        AppId::Files => files(vm),
        AppId::Office => office(vm, ui),
        AppId::BatterySaver => battery(vm),
        AppId::DataSense => data_sense(vm),
        _ => unreachable!(),
    }
}

fn maps(vm: Rc<PhoneViewModel>, ui: Rc<Session>) -> Element {
    let search = vm.clone();
    let state = ui.clone();
    let locate = vm.clone();
    let key_vm = vm.clone();
    let key_ui = ui.clone();
    let field = input("Search for a place")
        .text(ui.query.two_way())
        .on_key_down(move |event| {
            if event.key == ferroui_base::input::Key::Return {
                event.set_handled(true);
                find_place(&key_vm, &key_ui);
            }
        });
    let body = rsx! {StackPanel{spacing:24.0,margin:20.0,
        {text("maps",48.0)}{field}
        Border{height:180.0,{icon(Icon::Maps,100.0,&accent(&vm))}}
        {text("Where do you want to go?",28.0)}
        {muted("Find a place, explore its map, or use your current location.",20.0)}
    }};
    page(
        "maps",
        scroll(body),
        app_bar(
            vec![
                action(Icon::Search, "search", move || {
                    find_place(&search, &state);
                }),
                action(Icon::Location, "my location", move || {
                    locate.request(HostRequest::Locate)
                }),
            ],
            false,
            vec![],
        ),
    )
}

fn find_place(vm: &PhoneViewModel, ui: &Session) {
    let query = ui.query.get_untracked();
    let query = query.trim();
    if query.is_empty() {
        vm.set_status("Enter a place to search for.".into());
        return;
    }
    vm.request(HostRequest::OpenUrl {
        url: format!(
            "https://www.openstreetmap.org/search?query={}",
            super::encode(query)
        ),
    });
}

fn weather(vm: Rc<PhoneViewModel>, ui: Rc<Session>) -> Element {
    let current = ui.weather.get();
    let update = vm.clone();
    let (place, temperature, condition) = current
        .map(|(place, temp, condition)| (place, format!("{temp:.0}°"), condition))
        .unwrap_or((
            "your weather".into(),
            "–°".into(),
            "Find out what's happening outside.".into(),
        ));
    let body = rsx! {StackPanel{spacing:18.0,margin:24.0,
        {text(&place,34.0)}
        TextBlock{text:temperature,font_size:100.0,font_weight:ferroui_base::media::FontWeight::Light}
        Border{width:90.0,height:90.0,horizontal_alignment:H::Left,{icon(Icon::Weather,78.0,&accent(&vm))}}
        {text(&condition,24.0)}
        {muted("Current conditions for your location",16.0)}
        {muted("Weather data: Open-Meteo",12.0)}
    }};
    page(
        "weather",
        scroll(body),
        app_bar(
            vec![action(Icon::Location, "use my location", move || {
                update.request(HostRequest::FetchWeather)
            })],
            false,
            vec![],
        ),
    )
}

fn files(vm: Rc<PhoneViewModel>) -> Element {
    let photos = vm.data().photos.len();
    let music = vm
        .data()
        .tracks
        .iter()
        .filter(|track| track.media_kind != "video")
        .count();
    let videos = vm
        .data()
        .tracks
        .iter()
        .filter(|track| track.media_kind == "video")
        .count();
    let notes = vm.data().notes.len();
    let mut body = stack_panel().child(title("phone"));
    for (label, count, app) in [
        ("Pictures", photos, AppId::Photos),
        ("Music", music, AppId::Music),
        ("Videos", videos, AppId::Video),
        ("Documents", notes, AppId::OneNote),
    ] {
        let model = vm.clone();
        body = body.child(row(label, &format!("{count} items"), move || {
            model.navigate(Route::App(app))
        }));
    }
    let import = vm.clone();
    let export = vm.clone();
    page(
        "files",
        scroll(body.into()),
        app_bar(
            vec![
                action(Icon::Download, "export backup", move || {
                    export.request(HostRequest::ExportBackup)
                }),
                action(Icon::Upload, "restore backup", move || {
                    import.request(HostRequest::ImportBackup)
                }),
            ],
            false,
            vec![],
        ),
    )
}

fn office(vm: Rc<PhoneViewModel>, ui: Rc<Session>) -> Element {
    let mut body = stack_panel();
    let mut tabs = stack_panel()
        .orientation(Orientation::Horizontal)
        .margin((20.0, 0.0));
    for (i, label) in ["recent", "places"].iter().enumerate() {
        let state = ui.clone();
        tabs = tabs.child(pivot(label, ui.pivot.get() == i, move || {
            state.pivot.set(i)
        }));
    }
    body = body.child(tabs);
    if ui.pivot.get() == 1 {
        let notes = vm.clone();
        let drive = vm.clone();
        body = body
            .child(row("phone", "your notes and documents", move || {
                notes.navigate(Route::App(AppId::OneNote))
            }))
            .child(row("OneDrive", "open your files online", move || {
                drive.request(HostRequest::OpenUrl {
                    url: "https://onedrive.live.com/".into(),
                })
            }));
    } else {
        for note in vm.data().notes.clone() {
            let model = vm.clone();
            let id = note.id;
            body = body.child(row(&note.title, "OneNote", move || {
                model.navigate(Route::Note(id))
            }));
        }
        let online = vm.clone();
        body = body.child(row(
            "Microsoft 365",
            "open Word, Excel, and PowerPoint online",
            move || {
                online.request(HostRequest::OpenUrl {
                    url: "https://www.microsoft365.com/".into(),
                })
            },
        ));
    }
    let new = vm.clone();
    page(
        "office",
        scroll(body.into()),
        app_bar(
            vec![action(Icon::Plus, "new note", move || {
                new.navigate(Route::App(AppId::OneNote))
            })],
            false,
            vec![],
        ),
    )
}

fn battery(vm: Rc<PhoneViewModel>) -> Element {
    let enabled = vm.data().settings.battery_saver;
    let model = vm.clone();
    let body = rsx! {StackPanel{spacing:22.0,margin:20.0,
        {text("settings",46.0)}
        {text("Conserve battery",28.0)}
        {toggle("Battery Saver",enabled,move|_|model.toggle_setting("battery_saver"))}
        {muted("When Battery Saver is on, nonessential live tile updates pause while this app is in the background.",20.0)}
        {muted("Your device controls its system battery and power settings.",16.0)}
    }};
    page("battery saver", scroll(body), Element::Empty)
}

fn data_sense(vm: Rc<PhoneViewModel>) -> Element {
    let bytes = vm.export_json().map(|json| json.len()).unwrap_or(0);
    let size = if bytes < 1024 {
        format!("{bytes} B")
    } else {
        format!("{:.1} KB", bytes as f64 / 1024.0)
    };
    let count = vm.data().photos.len() + vm.data().tracks.len();
    let backup = vm.clone();
    let body = rsx! {StackPanel{spacing:22.0,margin:20.0,
        {text("overview",46.0)}{text(&size,72.0)}
        {text("saved phone data",24.0)}
        {muted(&format!("{count} photos and media entries; media files are stored separately."),18.0)}
        {muted("Your browser cannot read your cellular allowance or other apps' data use.",18.0)}
    }};
    page(
        "data sense",
        scroll(body),
        app_bar(
            vec![action(Icon::Save, "back up", move || {
                backup.request(HostRequest::ExportBackup)
            })],
            false,
            vec![],
        ),
    )
}
