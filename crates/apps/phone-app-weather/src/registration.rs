use phone_sdk::AppRegistration;

pub fn registration() -> AppRegistration {
    AppRegistration::builtin(phone_domain::AppId::Weather, crate::view)
        .with_tile(crate::tile::view)
        .with_host_weather(|ui, place, temperature, condition| {
            ui.state::<crate::WeatherState>()
                .current
                .set(Some((place, temperature, condition)))
        })
        .with_reset(|ui| ui.state::<crate::WeatherState>().current.set(None))
}
