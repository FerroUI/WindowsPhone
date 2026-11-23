//! Bounded Open-Meteo current weather for an explicitly obtained host location.
use super::location::Position;
use std::{sync::OnceLock, time::Duration};

#[derive(Clone, Debug, PartialEq)]
pub struct CurrentWeather {
    pub temperature_celsius: f64,
    pub condition: &'static str,
}

pub fn current(position: Position) -> Result<CurrentWeather, String> {
    let position = position.validate()?;
    static AGENT: OnceLock<ureq::Agent> = OnceLock::new();
    let agent = AGENT.get_or_init(|| {
        ureq::Agent::config_builder()
            .https_only(true)
            .max_redirects(0)
            .timeout_global(Some(Duration::from_secs(15)))
            .build()
            .into()
    });
    let url = format!(
        "https://api.open-meteo.com/v1/forecast?latitude={:.6}&longitude={:.6}&current=temperature_2m,weather_code&timezone=auto",
        position.latitude, position.longitude
    );
    let text = agent
        .get(&url)
        .header("Accept", "application/json")
        .call()
        .map_err(|error| format!("Weather request failed: {error}"))?
        .body_mut()
        .with_config()
        .limit(64 * 1024)
        .read_to_string()
        .map_err(|error| format!("Weather response failed: {error}"))?;
    parse(&text)
}

fn parse(text: &str) -> Result<CurrentWeather, String> {
    if text.len() > 64 * 1024 {
        return Err("Weather response is too large.".into());
    }
    let data: serde_json::Value =
        serde_json::from_str(text).map_err(|_| "Weather response is not valid JSON.")?;
    let current = &data["current"];
    let temperature = current["temperature_2m"]
        .as_f64()
        .filter(|v| v.is_finite() && (-100.0..=80.0).contains(v))
        .ok_or("Weather response has no valid temperature.")?;
    let code = current["weather_code"]
        .as_u64()
        .ok_or("Weather response has no condition code.")?;
    let condition = match code {
        0 => "clear sky",
        1 => "mainly clear",
        2 => "partly cloudy",
        3 => "overcast",
        45 | 48 => "fog",
        51 | 53 | 55 => "drizzle",
        56 | 57 => "freezing drizzle",
        61 | 63 | 65 => "rain",
        66 | 67 => "freezing rain",
        71 | 73 | 75 => "snow",
        77 => "snow grains",
        80..=82 => "rain showers",
        85 | 86 => "snow showers",
        95 => "thunderstorms",
        96 | 99 => "thunderstorms with hail",
        _ => "conditions unavailable",
    };
    Ok(CurrentWeather {
        temperature_celsius: temperature,
        condition,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn weather_requires_real_bounded_readings_and_maps_wmo_codes() {
        let result = parse(r#"{"current":{"temperature_2m":-3.5,"weather_code":66}}"#).unwrap();
        assert_eq!(result.temperature_celsius, -3.5);
        assert_eq!(result.condition, "freezing rain");
        assert!(parse(r#"{"current":{"temperature_2m":null,"weather_code":0}}"#).is_err());
        assert!(parse(r#"{"current":{"temperature_2m":999,"weather_code":0}}"#).is_err());
        assert!(parse(&" ".repeat(64 * 1024 + 1)).is_err());
    }
}
