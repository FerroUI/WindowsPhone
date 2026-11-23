use crate::{
    commands,
    model::{self, City, Saved},
    WeatherState,
};
use phone_sdk::prelude::*;
pub fn welcome(cx: &AppContext, saved: &Saved) -> Element {
    let current = cx.session.state::<WeatherState>().current.get();
    if let Some((place, temperature, condition)) = current {
        return rsx! {StackPanel{margin:20.0,spacing:16.0,{text(&place,32.0)}{text(&model::degrees(temperature, saved.fahrenheit),80.0)}{text(&condition,24.0)}{muted("Add a city for hourly and seven-day forecasts.",18.0)}}};
    }
    let cx = cx.clone();
    rsx! {StackPanel{margin:20.0,spacing:18.0,{text("your weather",38.0)}{phone_controls::icon(Icon::Weather,80.0,&accent(&cx.vm))}{muted("Save a city to see current conditions, the next 24 hours and a seven-day forecast.",20.0)}{flat("add a city",move||cx.session.pivot.set(3))}}}
}
pub fn forecast(cx: &AppContext, city: &City, saved: &Saved) -> Element {
    let mut body = stack_panel()
        .margin(20.0)
        .spacing(16.0)
        .child(text(&city.place.name, 30.0));
    let Some(forecast) = &city.forecast else {
        let place = city.place.clone();
        let cx = cx.clone();
        return body
            .child(muted(
                "No weather has been downloaded for this city yet.",
                20.0,
            ))
            .child(flat("get forecast", move || {
                commands::refresh(cx.clone(), place.clone())
            }))
            .into();
    };
    match cx.session.pivot.get() {
        1 => {
            for hour in &forecast.hours {
                let time = chrono::NaiveDateTime::parse_from_str(&hour.time, "%Y-%m-%dT%H:%M")
                    .map(|time| time.format("%H:%M").to_string())
                    .unwrap_or_else(|_| hour.time.clone());
                body=body.child(rsx!{StackPanel{spacing:4.0,{text(&format!("{}    {}",time,model::degrees(hour.temperature,saved.fahrenheit)),26.0)}{muted(&format!("{} · {:.0}% rain",model::condition(hour.code),hour.rain),17.0)}}});
            }
        }
        2 => {
            for day in &forecast.days {
                let date = chrono::NaiveDate::parse_from_str(&day.date, "%Y-%m-%d")
                    .map(|date| date.format("%a, %e %b").to_string())
                    .unwrap_or(day.date.clone());
                body=body.child(rsx!{StackPanel{spacing:4.0,{text(&date,26.0)}{text(&format!("{} / {}",model::degrees(day.high,saved.fahrenheit),model::degrees(day.low,saved.fahrenheit)),30.0)}{muted(&format!("{} · {:.0}% rain",model::condition(day.code),day.rain),17.0)}}});
            }
        }
        _ => {
            body = body
                .child(text(
                    &model::degrees(forecast.temperature, saved.fahrenheit),
                    80.0,
                ))
                .child(text(model::condition(forecast.code), 27.0))
                .child(muted(
                    &format!(
                        "feels like {}",
                        model::degrees(forecast.feels_like, saved.fahrenheit)
                    ),
                    21.0,
                ))
                .child(text(
                    &format!(
                        "humidity  {:.0}%\nwind  {:.0} km/h",
                        forecast.humidity, forecast.wind_kmh
                    ),
                    23.0,
                ));
        }
    }
    let age = (cx.vm.now() - forecast.retrieved).max(0);
    body.child(muted(
        &if age < 60 {
            "updated just now".into()
        } else if age < 3600 {
            format!("saved forecast · updated {} min ago", age / 60)
        } else {
            format!("saved forecast · updated {} hours ago", age / 3600)
        },
        14.0,
    ))
    .into()
}
pub fn places(cx: &AppContext, saved: &Saved) -> Element {
    let state = cx.session.state::<WeatherState>();
    let search = cx.clone();
    let enter = cx.clone();
    let field = input("City name")
        .name("WeatherCitySearch")
        .text(state.query.two_way())
        .on_key_down(move |event| {
            if event.key == ferroui_base::input::Key::Return {
                event.set_handled(true);
                commands::search(enter.clone());
            }
        });
    let mut body = stack_panel()
        .margin(20.0)
        .spacing(12.0)
        .child(field)
        .child(flat("search cities", move || {
            commands::search(search.clone())
        }));
    for place in state.results.get() {
        let cx = cx.clone();
        let name = place.name.clone();
        body = body.child(row(&name, "add this city", move || {
            commands::add(cx.clone(), place.clone())
        }));
    }
    body = body.child(text("saved places", 30.0));
    if saved.cities.is_empty() {
        body = body.child(muted("Search for a city above to get started.", 19.0));
    }
    for city in &saved.cities {
        let select = cx.clone();
        let id = city.place.id;
        let remove = cx.clone();
        let title = city.place.name.clone();
        let prompt = title.clone();
        let detail = city
            .forecast
            .as_ref()
            .map(|forecast| {
                format!(
                    "{} · {}",
                    model::degrees(forecast.temperature, saved.fahrenheit),
                    model::condition(forecast.code)
                )
            })
            .unwrap_or("tap to open".into());
        body = body
            .child(row(&title, &detail, move || {
                if commands::change(&select, |saved| {
                    saved.selected = Some(id);
                    Ok(())
                }) {
                    select.session.pivot.set(0);
                }
            }))
            .child(flat("remove city", move || {
                let confirmed = remove.clone();
                remove.session.confirm(
                    &format!("Remove {prompt} from your weather places?"),
                    Rc::new(move || {
                        commands::change(&confirmed, |saved| {
                            saved.remove(id);
                            Ok(())
                        });
                        confirmed.session.dialog.set(None);
                    }),
                );
            }));
    }
    body.into()
}
