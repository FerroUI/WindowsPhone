mod state;
use ferroui_base::layout::HorizontalAlignment as H;
use ferroui_view::prelude::*;
use phone_controls::{action, app_bar, icon, Icon};
use phone_sdk::{common::*, session::Session};
use phone_state::{HostRequest, PhoneViewModel};
pub use state::WeatherState;
use std::rc::Rc;

pub fn view(vm: Rc<PhoneViewModel>, ui: Rc<Session>) -> Element {
    let current = ui.state::<crate::WeatherState>().current.get();
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

mod registration;
pub use registration::registration;

mod tile;
