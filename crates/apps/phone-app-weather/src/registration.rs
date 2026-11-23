use phone_sdk::AppRegistration;

pub fn registration() -> AppRegistration {
    AppRegistration::builtin(phone_domain::AppId::Weather, crate::view)
        .with_tile(crate::tile::view)
        .with_pivot_count(|_| 4)
        .with_host_weather(|ui, place, temperature, condition| {
            ui.state::<crate::WeatherState>()
                .current
                .set(Some((place, temperature, condition)))
        })
        .with_reset(|ui| {
            let state = ui.state::<crate::WeatherState>();
            state.current.set(None);
            state.results.set(Vec::new());
            state.query.set(String::new());
            state.busy.set(false);
            state.generation.set(state.generation.get().wrapping_add(1));
        })
}
