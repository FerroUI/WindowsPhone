use crate::{common::*, session::Session};
use ferroui_base::layout::{HorizontalAlignment as H, Orientation, VerticalAlignment as V};
use ferroui_view::prelude::*;
use phone_controls::{Icon, action, app_bar, command, input, pivot};
use phone_domain::{AppId, Route};
use phone_state::PhoneViewModel;
use std::rc::Rc;

pub fn view(vm: Rc<PhoneViewModel>, ui: Rc<Session>) -> Element {
    if let Some(app) = ui
        .selected
        .get()
        .and_then(|id| AppId::ALL.get(id.saturating_sub(1) as usize).copied())
    {
        return details(vm, ui, app);
    }
    let mut body = stack_panel();
    let mut tabs = stack_panel()
        .orientation(Orientation::Horizontal)
        .margin((20.0, 0.0));
    for (i, label) in ["featured", "apps"].iter().enumerate() {
        let state = ui.clone();
        tabs = tabs.child(pivot(label, ui.pivot.get() == i, move || {
            state.pivot.set(i)
        }));
    }
    body = body.child(tabs);
    if ui.pivot.get() == 0 && ui.query.get().is_empty() {
        let games = vm.clone();
        body=body.child(rsx!{Button{class:"phone-flat",margin:(20.0,12.0),min_height:160.0,background:"#108A15",padding:20.0,
            on_click:move |_|games.navigate(Route::App(AppId::Games)),
            StackPanel{spacing:16.0,
                TextBlock{text:"a little playtime",font_size:32.0,foreground:"White",font_weight:ferroui_base::media::FontWeight::Light}
                TextBlock{text:"tic tac toe • built into your phone",font_size:17.0,foreground:"White"}
                Border{horizontal_alignment:H::Right,{phone_controls::icon(Icon::Games,44.0,"White")}}
            }
        }});
    }
    body = body.child(
        input("Search apps")
            .name("StoreSearch")
            .text(ui.query.two_way())
            .margin((20.0, 12.0)),
    );
    let query = ui.query.get().to_lowercase();
    for (index, app) in AppId::ALL
        .into_iter()
        .enumerate()
        .filter(|(_, app)| app.label().to_lowercase().contains(&query))
    {
        let state = ui.clone();
        let installed = vm.data().installed.contains(&app);
        let color = accent(&vm);
        body=body.child(rsx!{Button{class:"phone-row",padding:(20.0,11.0),min_height:85.0,automation_properties_name:app.label(),
            on_click:move |_|state.selected.set(Some(index as u64+1)),
            Grid{column_definitions:"64,*",column_spacing:16.0,
                {app_square(app,64.0,&color)}
                StackPanel{Grid.column:1,spacing:5.0,vertical_alignment:V::Center,
                    {text(app.label(),23.0)}
                    {muted(if installed{"installed"}else{"free • included app"},15.0)}
                }
            }
        }});
    }
    page("store", scroll(body.into()), Element::Empty)
}

fn details(vm: Rc<PhoneViewModel>, ui: Rc<Session>, app: AppId) -> Element {
    let installed = vm.data().installed.contains(&app);
    let model = vm.clone();
    let primary = command(if installed { "open" } else { "install" }, move || {
        if installed {
            model.navigate(Route::App(app));
        } else {
            model.install(app);
        }
    });
    let mut body=stack_panel().margin(20.0).spacing(24.0)
        .child(rsx!{Grid{column_definitions:"84,*",column_spacing:18.0,
            {app_square(app,84.0,&accent(&vm))}
            StackPanel{Grid.column:1,spacing:7.0,{text(app.label(),31.0)}{muted("included with this phone",16.0)}}
        }})
        .child(primary).child(text("overview",38.0)).child(phone_controls::paragraph(description(app)))
        .child(muted("This collection contains the apps built into this recreation. The retired Windows Phone Store is not connected.",16.0));
    if installed {
        let uninstall = vm.clone();
        let sheet = ui.clone();
        body = body.child(command("uninstall", move || {
            let model = uninstall.clone();
            let state = sheet.clone();
            sheet.confirm(
                &format!("Uninstall {}?", app.label()),
                Rc::new(move || {
                    model.uninstall(app);
                    state.dialog.set(None);
                }),
            );
        }));
    }
    let pin = vm.clone();
    page(
        "store",
        scroll(body.into()),
        app_bar(
            vec![action(Icon::Pin, "pin to start", move || pin.pin(app))],
            false,
            vec![],
        ),
    )
}

fn description(app: AppId) -> &'static str {
    match app {
        AppId::Phone => {
            "Keep your call history, find a contact, and dial with your device's phone app."
        }
        AppId::Messaging => {
            "Follow your conversations, write messages, and hand them to your device's SMS composer."
        }
        AppId::People => {
            "Keep names, numbers and email addresses together. Add people and mark your favorites."
        }
        AppId::Outlook => {
            "Read your mail collection and compose messages in your connected email app."
        }
        AppId::Calendar => {
            "See your month at a glance and keep appointments with dates, times and locations."
        }
        AppId::Photos => {
            "Your Camera Roll, albums and favorite pictures. Import, view, zoom, share and delete photos."
        }
        AppId::Camera => {
            "Use your device's camera to capture a moment and add it to your Camera Roll."
        }
        AppId::InternetExplorer => {
            "Open the web from an address bar at the bottom of your screen. Save and manage favorites."
        }
        AppId::Music => "Add songs from your device and enjoy your own music collection.",
        AppId::Video => "Import and play your video files.",
        AppId::Games => "Play tic tac toe against your phone and keep a score for this session.",
        AppId::Store => "Find, install and pin the built-in app collection.",
        AppId::Settings => {
            "Make your phone yours. Pick an accent, change the background, and arrange the Start screen."
        }
        AppId::Alarms => "Create alarms, choose the days they repeat, and turn each one on or off.",
        AppId::Calculator => {
            "A responsive everyday calculator with keyboard support, memory and scientific functions."
        }
        AppId::OneNote => "Capture an idea, write a list, and come back to your notes later.",
        AppId::Maps => "Search OpenStreetMap and explore places around your current location.",
        AppId::Cortana => {
            "Find things on your phone, open apps, call a contact, or continue a search on the web."
        }
        AppId::Weather => "Look up current weather conditions for your location.",
        AppId::Files => {
            "Browse your photos, music and documents, or create and restore a phone backup."
        }
        AppId::Office => {
            "Find your recent notes and open your online documents in OneDrive or Microsoft 365."
        }
        AppId::BatterySaver => "Reduce unnecessary background activity in this app.",
        AppId::DataSense => "See how much data your phone collection uses and save a backup.",
    }
}
