mod commands;
mod model;
mod registration;
mod state;
mod tile;
mod views;
use phone_sdk::prelude::*;
pub use registration::registration;
pub use state::WeatherState;

pub fn view(vm: Rc<PhoneViewModel>, ui: Rc<Session>) -> Element {
    let cx = commands::context(vm.clone(), ui.clone());
    let saved = match commands::load(&cx) {
        Ok(saved) => saved,
        Err(error) => {
            return page(
                "weather",
                empty("Weather unavailable", &error),
                Element::Empty,
            );
        }
    };
    let state = ui.state::<WeatherState>();
    let mut tabs = stack_panel()
        .orientation(ferroui_base::layout::Orientation::Horizontal)
        .margin((20.0, 0.0));
    for (index, label) in ["today", "hourly", "daily", "places"]
        .into_iter()
        .enumerate()
    {
        let ui = ui.clone();
        tabs = tabs.child(pivot(label, ui.pivot.get() == index, move || {
            ui.pivot.set(index)
        }));
    }
    let city = saved
        .cities
        .iter()
        .find(|city| Some(city.place.id) == saved.selected);
    let content = if ui.pivot.get() == 3 {
        views::places(&cx, &saved)
    } else if let Some(city) = city {
        views::forecast(&cx, city, &saved)
    } else {
        views::welcome(&cx, &saved)
    };
    let tabs = phone_controls::pivot_header(
        tabs.into(),
        ["today", "hourly", "daily", "places"][ui.pivot.get().min(3)],
    );
    let body = stack_panel().child(tabs).child(content);
    let mut actions = Vec::new();
    if let Some(city) = city {
        let cx = cx.clone();
        let place = city.place.clone();
        actions.push(action(Icon::Refresh, "refresh", move || {
            commands::refresh(cx.clone(), place.clone())
        }));
    }
    let places = ui.clone();
    actions.push(action(Icon::Plus, "add city", move || places.pivot.set(3)));
    let units = cx.clone();
    actions.push(action(
        Icon::Settings,
        if saved.fahrenheit {
            "celsius"
        } else {
            "fahrenheit"
        },
        move || {
            commands::change(&units, |saved| {
                saved.fahrenheit = !saved.fahrenheit;
                Ok(())
            });
        },
    ));
    let locate = cx.clone();
    let busy = state.busy.get();
    let mut footer = stack_panel().margin((20.0, 0.0, 20.0, 20.0)).spacing(16.0);
    if busy {
        footer = footer.child(
            phone_controls::progress("Loading weather")
                .name("WeatherProgress")
                .is_indeterminate(true),
        );
    }
    footer = footer.child(muted(
        if busy {
            "loading weather…"
        } else {
            "Weather data: Open-Meteo · displayed times are local to each city"
        },
        if busy { 18.0 } else { 12.0 },
    ));
    footer = footer.child(flat("use my current location", move || {
        if commands::change(&locate, |saved| {
            saved.selected = None;
            Ok(())
        }) {
            let state = locate.session.state::<WeatherState>();
            state.generation.set(state.generation.get().wrapping_add(1));
            state.busy.set(false);
            state.current.set(None);
            locate.session.pivot.set(0);
            locate.vm.request(phone_state::HostRequest::FetchWeather);
        }
    }));
    page(
        "weather",
        scroll(body.child(footer).into()),
        app_bar(actions, false, vec![]),
    )
}
