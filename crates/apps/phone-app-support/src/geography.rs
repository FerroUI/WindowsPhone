//! Shared bounded city lookup and coordinate contracts for location-based apps.
use serde::{Deserialize, Serialize};
use serde_json::Value;
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Coordinates {
    pub latitude: f64,
    pub longitude: f64,
}
impl Coordinates {
    pub fn new(latitude: f64, longitude: f64) -> Result<Self, String> {
        let value = Self {
            latitude,
            longitude,
        };
        value.validate()?;
        Ok(value)
    }
    pub fn validate(self) -> Result<(), String> {
        if self.latitude.is_finite()
            && self.longitude.is_finite()
            && (-90.0..=90.0).contains(&self.latitude)
            && (-180.0..=180.0).contains(&self.longitude)
        {
            Ok(())
        } else {
            Err("Use a latitude from −90 to 90 and a longitude from −180 to 180.".into())
        }
    }
    pub fn parse(value: &str) -> Result<Self, String> {
        let (latitude, longitude) = value
            .split_once(',')
            .ok_or("Enter latitude, longitude, for example 50.091, 18.219.")?;
        let latitude = latitude
            .trim()
            .parse::<f64>()
            .map_err(|_| "Enter a numeric latitude.".to_owned())?;
        let longitude = longitude
            .trim()
            .parse::<f64>()
            .map_err(|_| "Enter a numeric longitude.".to_owned())?;
        Self::new(latitude, longitude)
    }
    pub fn label(self) -> String {
        format!("{:.6}, {:.6}", self.latitude, self.longitude)
    }
    pub fn osm_url(self) -> Result<String, String> {
        self.validate()?;
        Ok(format!(
            "https://www.openstreetmap.org/?mlat={:.6}&mlon={:.6}#map=16/{:.6}/{:.6}",
            self.latitude, self.longitude, self.latitude, self.longitude
        ))
    }
}
#[derive(Clone, Debug, PartialEq)]
pub struct PlaceMatch {
    pub id: u64,
    pub name: String,
    pub coordinates: Coordinates,
}

pub fn geocoding_url(query: &str) -> Result<String, String> {
    let query = query.trim();
    if query.chars().count() < 2 || query.len() > 200 || query.chars().any(char::is_control) {
        return Err("Enter a city name of 2–200 characters.".into());
    }
    Ok(format!(
        "https://geocoding-api.open-meteo.com/v1/search?name={}&count=10&language=en&format=json",
        crate::urls::encode(query)
    ))
}
pub fn geocoding_places(bytes: &[u8]) -> Result<Vec<PlaceMatch>, String> {
    if bytes.len() > 64 * 1024 {
        return Err("The city search response is too large.".into());
    }
    let root: Value = serde_json::from_slice(bytes)
        .map_err(|_| "The city search returned invalid data.".to_owned())?;
    if !root.is_object() {
        return Err("The city search returned invalid data.".into());
    }
    if root.get("error").and_then(Value::as_bool) == Some(true) {
        return Err("The city search service could not complete the request.".into());
    }
    let Some(results) = root.get("results") else {
        return Ok(Vec::new());
    };
    let results = results
        .as_array()
        .ok_or("The city search returned invalid places.")?;
    if results.len() > 10 {
        return Err("The city search returned too many places.".into());
    }
    let mut places = Vec::new();
    let mut ids = std::collections::HashSet::new();
    for value in results {
        let id = value
            .get("id")
            .and_then(Value::as_u64)
            .filter(|id| *id > 0 && *id <= 9_007_199_254_740_991)
            .ok_or("A city has an invalid identifier.")?;
        let name = value
            .get("name")
            .and_then(Value::as_str)
            .ok_or("A city has no name.")?;
        let mut parts = vec![name];
        for field in ["admin1", "country"] {
            if let Some(value) = value.get(field).and_then(Value::as_str) {
                if !value.trim().is_empty()
                    && !parts.iter().any(|part| part.eq_ignore_ascii_case(value))
                {
                    parts.push(value);
                }
            }
        }
        let name = parts.join(", ");
        if name.trim().is_empty() || name.len() > 512 || name.chars().any(char::is_control) {
            return Err("A city has an invalid name.".into());
        }
        let coordinates = Coordinates::new(
            value
                .get("latitude")
                .and_then(Value::as_f64)
                .ok_or("A city has no latitude.")?,
            value
                .get("longitude")
                .and_then(Value::as_f64)
                .ok_or("A city has no longitude.")?,
        )?;
        if ids.insert(id) {
            places.push(PlaceMatch {
                id,
                name,
                coordinates,
            });
        }
    }
    Ok(places)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn coordinates_and_query_parameters_are_bounded_and_encoded() {
        assert!(Coordinates::parse("50.091, 18.219")
            .unwrap()
            .osm_url()
            .unwrap()
            .contains("mlat=50.091000&mlon=18.219000"));
        for bad in ["NaN,10", "91,0", "0,181", "0,1,2", "latitude,longitude"] {
            assert!(Coordinates::parse(bad).is_err(), "{bad}");
        }
        let query = geocoding_url("A&B #city").unwrap();
        assert!(query.contains("A%26B%20%23city"));
        assert!(geocoding_url("\n").is_err());
        assert!(geocoding_url(&"x".repeat(201)).is_err());
    }
    #[test]
    fn city_search_validates_responses_and_returns_real_coordinates() {
        let bytes=br#"{"results":[{"id":12,"name":"Raciborz","admin1":"Silesia","country":"Poland","latitude":50.091,"longitude":18.219}]}"#;
        let places = geocoding_places(bytes).unwrap();
        assert_eq!(places[0].id, 12);
        assert_eq!(places[0].name, "Raciborz, Silesia, Poland");
        assert_eq!(
            places[0].coordinates,
            Coordinates {
                latitude: 50.091,
                longitude: 18.219
            }
        );
        assert!(geocoding_places(
            br#"{"results":[{"id":1,"name":"Invalid","latitude":300,"longitude":0}]}"#
        )
        .is_err());
        assert!(geocoding_places(br#"{"results":null}"#).is_err());
        assert!(geocoding_places(br#"{"error":true}"#).is_err());
        assert!(geocoding_places(br#"{}"#).unwrap().is_empty());
    }
}
