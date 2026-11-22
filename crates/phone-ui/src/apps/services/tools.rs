use crate::{common::*, session::Session};
use ferroui_base::layout::HorizontalAlignment as H;
use ferroui_view::prelude::*;
use phone_controls::{Icon, action, app_bar, icon, input};
use phone_domain::AppId;
use phone_state::{HostRequest, PhoneViewModel};
use std::rc::Rc;

pub fn view(vm: Rc<PhoneViewModel>, ui: Rc<Session>, app: AppId) -> Element {
    match app {
        AppId::Maps => maps(vm, ui),
        AppId::Weather => weather(vm, ui),
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
