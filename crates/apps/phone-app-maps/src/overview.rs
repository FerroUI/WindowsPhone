use crate::{
    commands,
    model::{Place, Saved},
    state::MapSession,
    viewport,
};
use ferroui_base::layout::{HorizontalAlignment as H, VerticalAlignment as V};
use ferroui_view::prelude::*;
use phone_controls::{flat, input};
use phone_sdk::{common::*, AppContext};

pub fn view(cx: AppContext, saved: &Saved, tabs: Element) -> Element {
    let state = cx.session.state::<MapSession>();
    let keyboard = cx.clone();
    let query = cx.session.query.get();
    let results = state.results.get();
    let busy = state.busy.get();
    let content = if !query.trim().is_empty() && (busy || !results.is_empty()) {
        let close = state.clone();
        let ui = cx.session.clone();
        let mut results_view = stack_panel()
            .spacing(8.0)
            .child(flat("back to map", move || {
                commands::cancel_search(&ui);
                close.results.set(Vec::new());
                ui.query.set(String::new());
            }));
        if busy {
            results_view = results_view.child(muted("Searching cities…", 20.0));
        }
        for place in results {
            let cx = cx.clone();
            let title = place.name.clone();
            let detail = place.coordinates.label();
            results_view = results_view.child(row(&title, &detail, move || {
                commands::select(&cx, commands::from_match(place.clone()))
            }));
        }
        results_view = results_view.child(muted("City search: Open-Meteo", 13.0));
        scroll(results_view.into())
    } else {
        viewport::view(cx.clone(), saved.clone(), None, None)
    };
    let mut shortcuts = uniform_grid().columns(2).rows(1).margin((12.0, 3.0));
    for (home, label, id) in [(true, "home", saved.home), (false, "work", saved.work)] {
        let context = cx.clone();
        let place = id
            .and_then(|id| saved.places.iter().find(|place| place.id == id))
            .cloned();
        let caption = place
            .as_ref()
            .map(|place| place.name.clone())
            .unwrap_or_else(|| "tap to set".into());
        shortcuts=shortcuts.child(rsx!{Button{class:"phone-flat",height:60.0,horizontal_content_alignment:H::Left,padding:(8.0,3.0),automation_properties_name:label,
            on_click:move|_|{if let Some(place)=place.clone(){commands::select(&context,place)}else{commands::edit(context.clone(),Place{id:0,name:if home{"Home"}else{"Work"}.into(),address:String::new(),coordinates:None},Some(home));}},
            StackPanel{spacing:1.0,vertical_alignment:V::Center,TextBlock{text:label,font_size:21.0},TextBlock{text:caption,font_size:14.0,foreground:phone_controls::secondary_color()}}
        }});
    }
    let external = cx.clone();
    rsx! {Grid{row_definitions:"Auto,Auto,*,Auto,Auto",row_spacing:5.0,
        {tabs}
        Border{Grid.row:1,margin:(20.0,3.0),{input("Search for a city or coordinates").name("MapSearch").text(cx.session.query.two_way()).on_key_down(move|event|if event.key==ferroui_base::input::Key::Return{event.set_handled(true);commands::search(keyboard.clone());})}}
        Border{Grid.row:2,margin:(12.0,0.0),clip_to_bounds:true,{content}}
        if !query.trim().is_empty(){Border{Grid.row:3,margin:(20.0,0.0),{flat("search on map",move||commands::external_search(&external))}}}
        Border{Grid.row:4,{shortcuts}}
    }}
}
