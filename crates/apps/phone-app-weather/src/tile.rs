use ferroui_base::{
    layout::VerticalAlignment as V,
    media::{FontWeight, TextWrapping},
};
use ferroui_view::prelude::*;
use phone_controls::{icon, Icon};
use phone_domain::TileSize;

pub fn view(cx: phone_sdk::AppTileContext) -> Element {
    if cx.size == TileSize::Small {
        return icon(Icon::Weather, 31.0, "White");
    }
    let saved = crate::commands::read_saved(cx.app.vm.clone());
    let fahrenheit = saved.as_ref().is_some_and(|saved| saved.fahrenheit);
    let weather = saved
        .as_ref()
        .and_then(|saved| {
            saved
                .cities
                .iter()
                .find(|city| Some(city.place.id) == saved.selected)
        })
        .and_then(|city| {
            city.forecast.as_ref().map(|forecast| {
                (
                    city.place.name.clone(),
                    forecast.temperature,
                    crate::model::condition(forecast.code).into(),
                )
            })
        })
        .or_else(|| cx.app.session.state::<crate::WeatherState>().current.get());
    rsx! {Grid{margin:(14.0,12.0,14.0,32.0),column_definitions:"70,*",
        {icon(Icon::Weather,55.0,"White")}
        StackPanel{Grid.column:1,spacing:3.0,vertical_alignment:V::Center,
            TextBlock{text:weather.as_ref().map(|(_,degrees,_)|crate::model::degrees(*degrees,fahrenheit)).unwrap_or_else(||"Weather".into()),font_size:if weather.is_some(){42.0}else{29.0},foreground:"White",font_weight:FontWeight::Light}
            TextBlock{text:weather.as_ref().map(|(location,_,description)|format!("{location}\n{description}")).unwrap_or_else(||"tap for your forecast".into()),font_size:14.0,foreground:"White",text_wrapping:TextWrapping::Wrap}
        }
    }}
}
