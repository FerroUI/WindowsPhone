use serde::{Deserialize, Serialize};
use serde_json::Value;

pub const MAX_CITIES: usize = 10;
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Place {
    pub id: u64,
    pub name: String,
    pub latitude: f64,
    pub longitude: f64,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Hour {
    pub time: String,
    pub temperature: f64,
    pub rain: f64,
    pub code: u64,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Day {
    pub date: String,
    pub low: f64,
    pub high: f64,
    pub rain: f64,
    pub code: u64,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Forecast {
    pub retrieved: i64,
    pub temperature: f64,
    pub feels_like: f64,
    pub humidity: f64,
    pub wind_kmh: f64,
    pub code: u64,
    pub hours: Vec<Hour>,
    pub days: Vec<Day>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct City {
    pub place: Place,
    pub forecast: Option<Forecast>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Saved {
    pub schema: u32,
    pub cities: Vec<City>,
    pub selected: Option<u64>,
    pub fahrenheit: bool,
}
impl Default for Saved {
    fn default() -> Self {
        Self {
            schema: 1,
            cities: Vec::new(),
            selected: None,
            fahrenheit: false,
        }
    }
}
impl Saved {
    pub fn validate(&self) -> Result<(), String> {
        if self.schema != 1 || self.cities.len() > MAX_CITIES {
            return Err("This weather collection is unsupported or too large.".into());
        }
        let mut ids = std::collections::HashSet::new();
        for city in &self.cities {
            city.place.validate()?;
            if !ids.insert(city.place.id) {
                return Err("A city occurs more than once.".into());
            }
            if let Some(forecast) = &city.forecast {
                forecast.validate()?;
            }
        }
        if self.selected.is_some_and(|id| !ids.contains(&id)) {
            return Err("The selected weather city is missing.".into());
        }
        Ok(())
    }
    pub fn add(&mut self, place: Place) -> Result<(), String> {
        place.validate()?;
        if !self.cities.iter().any(|city| city.place.id == place.id) {
            if self.cities.len() >= MAX_CITIES {
                return Err("You can save up to ten cities. Remove one first.".into());
            }
            self.cities.push(City {
                place: place.clone(),
                forecast: None,
            });
        }
        self.selected = Some(place.id);
        Ok(())
    }
    pub fn remove(&mut self, id: u64) {
        self.cities.retain(|city| city.place.id != id);
        if self.selected == Some(id) {
            self.selected = self.cities.first().map(|city| city.place.id);
        }
    }
}
impl Place {
    pub fn validate(&self) -> Result<(), String> {
        if self.id == 0
            || self.id > 9_007_199_254_740_991
            || self.name.trim().is_empty()
            || self.name.len() > 512
            || self.name.chars().any(char::is_control)
            || !self.latitude.is_finite()
            || !(-90.0..=90.0).contains(&self.latitude)
            || !self.longitude.is_finite()
            || !(-180.0..=180.0).contains(&self.longitude)
        {
            Err("The weather place is invalid.".into())
        } else {
            Ok(())
        }
    }
    pub fn forecast_url(&self) -> String {
        format!("https://api.open-meteo.com/v1/forecast?latitude={:.6}&longitude={:.6}&current=temperature_2m,relative_humidity_2m,apparent_temperature,weather_code,wind_speed_10m&hourly=temperature_2m,precipitation_probability,weather_code&daily=weather_code,temperature_2m_max,temperature_2m_min,precipitation_probability_max&forecast_days=7&forecast_hours=24&timezone=auto",self.latitude,self.longitude)
    }
}
impl Forecast {
    pub fn validate(&self) -> Result<(), String> {
        if self.retrieved < 0
            || !temperature(self.temperature)
            || !temperature(self.feels_like)
            || !percent(self.humidity)
            || !self.wind_kmh.is_finite()
            || !(0.0..=500.0).contains(&self.wind_kmh)
            || self.code > 99
            || self.hours.len() > 24
            || self.days.len() > 7
            || self.hours.iter().any(|hour| {
                !temperature(hour.temperature)
                    || !percent(hour.rain)
                    || hour.code > 99
                    || chrono::NaiveDateTime::parse_from_str(&hour.time, "%Y-%m-%dT%H:%M").is_err()
            })
            || self.days.iter().any(|day| {
                !temperature(day.low)
                    || !temperature(day.high)
                    || day.low > day.high
                    || !percent(day.rain)
                    || day.code > 99
                    || chrono::NaiveDate::parse_from_str(&day.date, "%Y-%m-%d").is_err()
            })
        {
            Err("The forecast contains invalid or incomplete readings.".into())
        } else {
            Ok(())
        }
    }
}
fn temperature(v: f64) -> bool {
    v.is_finite() && (-120.0..=90.0).contains(&v)
}
fn percent(v: f64) -> bool {
    v.is_finite() && (0.0..=100.0).contains(&v)
}
pub fn degrees(value: f64, fahrenheit: bool) -> String {
    format!(
        "{:.0}°{}",
        if fahrenheit {
            value * 1.8 + 32.0
        } else {
            value
        },
        if fahrenheit { "F" } else { "C" }
    )
}
pub fn condition(code: u64) -> &'static str {
    match code {
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
    }
}
pub fn places(bytes: &[u8]) -> Result<Vec<Place>, String> {
    phone_app_support::geography::geocoding_places(bytes).map(|places| {
        places
            .into_iter()
            .map(|place| Place {
                id: place.id,
                name: place.name,
                latitude: place.coordinates.latitude,
                longitude: place.coordinates.longitude,
            })
            .collect()
    })
}

fn number(value: &Value) -> Result<f64, String> {
    value
        .as_f64()
        .filter(|v| v.is_finite())
        .ok_or("The forecast is missing a reading.".into())
}
fn code(value: &Value) -> Result<u64, String> {
    value
        .as_u64()
        .ok_or("The forecast is missing a condition code.".into())
}
pub fn forecast(bytes: &[u8], retrieved: i64) -> Result<Forecast, String> {
    let value: Value = serde_json::from_slice(bytes)
        .map_err(|_| "The weather service did not return valid JSON.")?;
    if value["error"].as_bool() == Some(true) {
        return Err("The weather service could not complete this request.".into());
    }
    let current = &value["current"];
    let hourly = &value["hourly"];
    let daily = &value["daily"];
    let mut result = Forecast {
        retrieved,
        temperature: number(&current["temperature_2m"])?,
        feels_like: number(&current["apparent_temperature"])?,
        humidity: number(&current["relative_humidity_2m"])?,
        wind_kmh: number(&current["wind_speed_10m"])?,
        code: code(&current["weather_code"])?,
        hours: Vec::new(),
        days: Vec::new(),
    };
    for (i, time) in hourly["time"]
        .as_array()
        .ok_or("The hourly forecast is missing.")?
        .iter()
        .take(24)
        .enumerate()
    {
        result.hours.push(Hour {
            time: time.as_str().ok_or("Invalid forecast time.")?.into(),
            temperature: number(&hourly["temperature_2m"][i])?,
            rain: number(&hourly["precipitation_probability"][i])?,
            code: code(&hourly["weather_code"][i])?,
        });
    }
    for (i, date) in daily["time"]
        .as_array()
        .ok_or("The daily forecast is missing.")?
        .iter()
        .take(7)
        .enumerate()
    {
        result.days.push(Day {
            date: date.as_str().ok_or("Invalid forecast date.")?.into(),
            low: number(&daily["temperature_2m_min"][i])?,
            high: number(&daily["temperature_2m_max"][i])?,
            rain: number(&daily["precipitation_probability_max"][i])?,
            code: code(&daily["weather_code"][i])?,
        });
    }
    if result.hours.is_empty() || result.days.is_empty() {
        return Err("The forecast contains no hourly or daily readings.".into());
    }
    result.validate()?;
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn places_are_validated_deduplicated_and_units_are_exact() {
        let bytes=br#"{"results":[{"id":1,"name":"Raciborz","latitude":50.1,"longitude":18.2,"country":"Poland"},{"id":1,"name":"Raciborz","latitude":50.1,"longitude":18.2}]}"#;
        let found = places(bytes).unwrap();
        assert_eq!(found.len(), 1);
        let mut saved = Saved::default();
        saved.add(found[0].clone()).unwrap();
        saved.validate().unwrap();
        saved.remove(1);
        assert!(saved.selected.is_none());
        assert_eq!(degrees(0.0, true), "32°F");
        assert_eq!(degrees(-40.0, true), "-40°F");
        assert!(
            places(br#"{"results":[{"id":1,"name":"Invalid","latitude":91,"longitude":18}]}"#)
                .is_err()
        );
    }
    #[test]
    fn incomplete_or_impossible_forecasts_never_become_cached_readings() {
        let mut value = serde_json::json!({"current":{"temperature_2m":10,"apparent_temperature":8,"relative_humidity_2m":75,"wind_speed_10m":12,"weather_code":66},"hourly":{"time":["2026-11-23T10:00"],"temperature_2m":[11],"precipitation_probability":[30],"weather_code":[3]},"daily":{"time":["2026-11-23"],"temperature_2m_min":[2],"temperature_2m_max":[12],"precipitation_probability_max":[40],"weather_code":[3]}});
        let reading = forecast(&serde_json::to_vec(&value).unwrap(), 1).unwrap();
        assert_eq!(condition(reading.code), "freezing rain");
        assert_eq!(reading.days.len(), 1);
        value["daily"]["temperature_2m_min"] = serde_json::json!([50]);
        assert!(forecast(&serde_json::to_vec(&value).unwrap(), 1).is_err());
        value["daily"]["temperature_2m_min"] = serde_json::json!([]);
        assert!(forecast(&serde_json::to_vec(&value).unwrap(), 1).is_err());
    }
}
