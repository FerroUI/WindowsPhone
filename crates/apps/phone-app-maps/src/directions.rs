use crate::{
    commands,
    model::{Saved, TravelMode},
    state::MapSession,
};
use ferroui_base::layout::{HorizontalAlignment as H, Orientation};
use ferroui_view::prelude::*;
use phone_controls::{action, app_bar, flat, input, Icon};
use phone_sdk::{common::*, AppContext};
pub fn view(cx: AppContext, saved: &Saved) -> Element {
    let state = cx.session.state::<MapSession>();
    let mut modes = grid()
        .column_definitions("*,*")
        .row_definitions("Auto,Auto")
        .column_spacing(12.0)
        .row_spacing(8.0);
    for (index, mode) in TravelMode::ALL.into_iter().enumerate() {
        let context = cx.clone();
        let selected = mode == saved.travel_mode;
        let color = if selected {
            accent(&cx.vm)
        } else {
            "Transparent".into()
        };
        modes=modes.child(rsx!{Button{Grid.column:(index%2)as i32,Grid.row:(index/2)as i32,class:"phone-flat",min_height:44.0,padding:8.0,
            border_thickness:(0.0,0.0,0.0,2.0),border_brush:color.as_str(),
            automation_properties_name:mode.label(),
            on_click:move |_|{if let Err(error)=commands::change(&context,|saved|{saved.travel_mode=mode;Ok(())}){context.vm.set_status(error)}},
            {text(mode.label(),20.0)}}});
    }
    let mut shortcuts = stack_panel()
        .orientation(Orientation::Horizontal)
        .spacing(12.0);
    for (label, id) in [("from home", saved.home), ("from work", saved.work)] {
        if let Some(place) = id.and_then(|id| saved.places.iter().find(|place| place.id == id)) {
            let state = state.clone();
            let destination = place.destination();
            shortcuts = shortcuts.child(flat(label, move || state.origin.set(destination.clone())));
        }
    }
    let body = rsx! {StackPanel{margin:20.0,spacing:16.0,
        {text("directions",38.0)}
        StackPanel{spacing:7.0,
            {muted("from",16.0)}
            {input("from (current location if empty)").name("MapOrigin").text(state.origin.two_way())}
        }
        StackPanel{spacing:7.0,
            {muted("to",16.0)}
            {input("destination").name("MapDestination").text(state.destination.two_way())}
        }
        {shortcuts}{modes}
        TextBlock{text:"Google Maps will open with these places and your travel mode.",font_size:17.0,text_wrapping:ferroui_base::media::TextWrapping::Wrap,horizontal_alignment:H::Stretch}
    }};
    let go = cx.clone();
    let cancel = cx.session.clone();
    let swap = state.clone();
    page(
        "maps",
        scroll(body),
        app_bar(
            vec![
                action(Icon::Forward, "get directions", move || {
                    commands::launch_directions(&go)
                }),
                action(Icon::Refresh, "swap", move || {
                    let origin = swap.origin.get_untracked();
                    let destination = swap.destination.get_untracked();
                    if !origin.trim().is_empty() {
                        swap.origin.set(destination);
                        swap.destination.set(origin);
                    }
                }),
                action(Icon::Close, "cancel", move || {
                    cancel.state::<MapSession>().directions.set(false)
                }),
            ],
            false,
            vec![],
        ),
    )
}
