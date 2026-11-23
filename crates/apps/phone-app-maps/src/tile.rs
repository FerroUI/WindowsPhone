use ferroui_base::{layout::VerticalAlignment as V, media::TextWrapping};
use ferroui_view::prelude::*;
use phone_controls::{icon, Icon};
use phone_domain::TileSize;
pub fn view(cx: phone_sdk::AppTileContext) -> Element {
    let small = cx.size == TileSize::Small;
    let place = crate::registration::place_id(&cx.path).ok().and_then(|id| {
        crate::commands::load(&cx.app)
            .ok()?
            .places
            .into_iter()
            .find(|place| place.id == id)
    });
    if let Some(place) = place.filter(|_| !small) {
        rsx! {StackPanel{margin:(12.0,12.0,12.0,35.0),spacing:10.0,vertical_alignment:V::Top,
            {icon(Icon::Location,32.0,"White")}
            TextBlock{text:if place.address.is_empty(){place.destination()}else{place.address},font_size:if cx.size==TileSize::Wide{20.0}else{16.0},foreground:"White",text_wrapping:TextWrapping::Wrap}
        }}
    } else {
        icon(Icon::Maps, if small { 31.0 } else { 56.0 }, "White")
    }
}
