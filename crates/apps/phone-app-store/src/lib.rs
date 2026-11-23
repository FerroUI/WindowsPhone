mod description;
mod details;
mod registration;
mod state;

pub use registration::registration;

use ferroui_base::layout::{HorizontalAlignment as H, Orientation, VerticalAlignment as V};
use ferroui_view::prelude::*;
use phone_controls::{input, pivot, Icon};
use phone_domain::{AppId, Route};
use phone_sdk::{common::*, session::Session, AppCategory, AppManifest};
use phone_state::PhoneViewModel;
use state::StoreSession;
use std::rc::Rc;

pub fn view(vm: Rc<PhoneViewModel>, ui: Rc<Session>) -> Element {
    let selected = ui.state::<StoreSession>().selected.get();
    if let Some(app) = selected
        .as_deref()
        .and_then(|id| ui.registry().by_id(id).map(|app| app.manifest))
    {
        return details::view(vm, ui, app);
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
    let manifests = ui.app_manifests();
    let games = manifests
        .iter()
        .filter(|app| app.category == AppCategory::Game && app.listed)
        .map(|app| app.title)
        .collect::<Vec<_>>();
    if ui.pivot.get() == 0 && ui.query.get().is_empty() && !games.is_empty() {
        let model = vm.clone();
        let subtitle = games.join(" • ");
        body=body.child(rsx!{Button{class:"phone-flat",margin:(20.0,12.0),min_height:160.0,background:"#108A15",padding:20.0,
            on_click:move |_|model.navigate(Route::App(AppId::Games)),
            StackPanel{spacing:16.0,
                TextBlock{text:"a little playtime",font_size:32.0,foreground:"White",font_weight:ferroui_base::media::FontWeight::Light}
                TextBlock{text:subtitle,font_size:17.0,foreground:"White",text_wrapping:ferroui_base::media::TextWrapping::Wrap}
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
    let query = ui.query.get().trim().to_lowercase();
    let mut apps = manifests
        .into_iter()
        .filter(|app| app.listed && app.title.to_lowercase().contains(&query))
        .collect::<Vec<_>>();
    apps.sort_by_key(|app| app.title.to_lowercase());
    if apps.is_empty() {
        body = body.child(empty("no results", "Try a different app name."));
    }
    for app in apps {
        let state = ui.state::<StoreSession>();
        let enabled = enabled(&vm, &app);
        let status = if enabled {
            "installed"
        } else if app.builtin.is_some() {
            "free • included app"
        } else {
            "disabled • included app"
        };
        body=body.child(rsx!{Button{class:"phone-row",padding:(20.0,11.0),min_height:85.0,automation_properties_name:app.title,
            on_click:move |_|state.selected.set(Some(app.id.into())),
            Grid{column_definitions:"64,*",column_spacing:16.0,
                {square(&vm,&app,64.0)}
                StackPanel{Grid.column:1,spacing:5.0,vertical_alignment:V::Center,
                    {text(app.title,23.0)}{muted(status,15.0)}
                }
            }
        }});
    }
    page("store", scroll(body.into()), Element::Empty)
}

pub(crate) fn enabled(vm: &PhoneViewModel, app: &AppManifest) -> bool {
    app.builtin.map_or_else(
        || vm.extension_enabled(app.id),
        |id| vm.data().installed.contains(&id),
    )
}

pub(crate) fn square(vm: &PhoneViewModel, app: &AppManifest, size: f64) -> Element {
    if let Some(id) = app.builtin {
        return app_square(id, size, &accent(vm));
    }
    let color = if app.accent.is_empty() {
        accent(vm)
    } else {
        app.accent.into()
    };
    rsx! {Border{width:size,height:size,background:color.as_str(),{phone_controls::icon(app.icon,size*0.56,"White")}}}
}
