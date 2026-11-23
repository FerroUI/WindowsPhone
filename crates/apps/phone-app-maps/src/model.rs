use phone_app_support::{geography::Coordinates, urls::encode};
use serde::{Deserialize, Serialize};
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Place {
    pub id: u32,
    pub name: String,
    pub address: String,
    pub coordinates: Option<Coordinates>,
}
impl Place {
    pub fn validate(&self) -> Result<(), String> {
        if self.id == 0
            || self.name.trim().is_empty()
            || self.name.trim() != self.name
            || self.name.chars().count() > 100
            || self.name.chars().any(char::is_control)
        {
            return Err("Give this place a name of 1–100 printable characters.".into());
        }
        validate_address(&self.address)?;
        if self.address.is_empty() && self.coordinates.is_none() {
            return Err("Enter an address or coordinates for this place.".into());
        }
        if let Some(coordinates) = self.coordinates {
            coordinates.validate()?;
        }
        Ok(())
    }
    pub fn destination(&self) -> String {
        self.coordinates
            .map(Coordinates::label)
            .unwrap_or_else(|| self.address.clone())
    }
    pub fn map_url(&self) -> Result<String, String> {
        self.validate()?;
        if let Some(coordinates) = self.coordinates {
            coordinates.osm_url()
        } else {
            Ok(format!(
                "https://www.openstreetmap.org/search?query={}",
                encode(&self.address)
            ))
        }
    }
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TravelMode {
    #[default]
    Driving,
    Walking,
    Bicycling,
    Transit,
}
impl TravelMode {
    pub const ALL: [Self; 4] = [Self::Driving, Self::Walking, Self::Bicycling, Self::Transit];
    pub fn label(self) -> &'static str {
        match self {
            Self::Driving => "driving",
            Self::Walking => "walking",
            Self::Bicycling => "bicycling",
            Self::Transit => "transit",
        }
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Saved {
    pub version: u32,
    pub next_id: u32,
    pub places: Vec<Place>,
    pub home: Option<u32>,
    pub work: Option<u32>,
    pub recent: Vec<String>,
    pub travel_mode: TravelMode,
}
impl Default for Saved {
    fn default() -> Self {
        Self {
            version: 1,
            next_id: 1,
            places: vec![],
            home: None,
            work: None,
            recent: vec![],
            travel_mode: TravelMode::Driving,
        }
    }
}
impl Saved {
    pub fn validate(&self) -> Result<(), String> {
        if self.version != 1 || self.places.len() > 100 || self.recent.len() > 32 {
            return Err("This map collection is unsupported or too large.".into());
        }
        let mut ids = std::collections::HashSet::new();
        let mut names = std::collections::HashSet::new();
        let mut max = 0;
        for place in &self.places {
            place.validate()?;
            if !ids.insert(place.id) || !names.insert(place.name.to_lowercase()) {
                return Err("Saved places need unique names and identifiers.".into());
            }
            max = max.max(place.id);
        }
        if self.next_id <= max
            || self.next_id == u32::MAX
            || [self.home, self.work]
                .into_iter()
                .flatten()
                .any(|id| !ids.contains(&id))
        {
            return Err("A map shortcut refers to a missing place.".into());
        }
        for recent in &self.recent {
            validate_address(recent)?;
            if recent.is_empty() {
                return Err("The recent search is empty.".into());
            }
        }
        Ok(())
    }
    pub fn save(&mut self, mut place: Place) -> Result<u32, String> {
        if place.id == 0 {
            if self.places.len() >= 100 || self.next_id >= u32::MAX - 1 {
                return Err("You can save up to 100 places.".into());
            }
            place.id = self.next_id;
        } else if !self.places.iter().any(|old| old.id == place.id) {
            return Err("This saved place no longer exists.".into());
        }
        place.validate()?;
        if self
            .places
            .iter()
            .any(|old| old.id != place.id && old.name.to_lowercase() == place.name.to_lowercase())
        {
            return Err("A place with this name already exists.".into());
        }
        let id = place.id;
        if let Some(previous) = self.places.iter_mut().find(|old| old.id == id) {
            *previous = place;
        } else {
            self.places.push(place);
            self.next_id += 1;
        }
        Ok(id)
    }
    pub fn remove(&mut self, id: u32) {
        self.places.retain(|place| place.id != id);
        if self.home == Some(id) {
            self.home = None;
        }
        if self.work == Some(id) {
            self.work = None;
        }
    }
    pub fn remember(&mut self, query: &str) -> Result<(), String> {
        let query = query.trim();
        validate_address(query)?;
        if query.is_empty() {
            return Err("Enter a place to search for.".into());
        }
        self.recent
            .retain(|entry| !entry.eq_ignore_ascii_case(query));
        self.recent.insert(0, query.into());
        self.recent.truncate(32);
        Ok(())
    }
}
pub fn validate_address(value: &str) -> Result<(), String> {
    if value.len() > 512 || value.trim() != value || value.chars().any(char::is_control) {
        Err("Use an address or search of up to 512 printable characters.".into())
    } else {
        Ok(())
    }
}
/// Universal map URLs preserve an explicit origin. Empty origin delegates
/// current-location consent to the external maps application.
pub fn directions_url(origin: &str, destination: &str, mode: TravelMode) -> Result<String, String> {
    let origin = origin.trim();
    let destination = destination.trim();
    validate_address(origin)?;
    validate_address(destination)?;
    if destination.is_empty() {
        return Err("Choose a destination first.".into());
    }
    if looks_like_coordinates(origin) {
        Coordinates::parse(origin)?;
    }
    if looks_like_coordinates(destination) {
        Coordinates::parse(destination)?;
    }
    let mut url = format!(
        "https://www.google.com/maps/dir/?api=1&destination={}&travelmode={}",
        encode(destination),
        mode.label()
    );
    if !origin.is_empty() {
        url.push_str("&origin=");
        url.push_str(&encode(origin));
    }
    if url.len() > 2048 {
        return Err("Shorten the origin or destination before opening directions.".into());
    }
    Ok(url)
}
pub(crate) fn looks_like_coordinates(value: &str) -> bool {
    value.split_once(',').is_some_and(|(first, last)| {
        first.trim().parse::<f64>().is_ok() && last.trim().parse::<f64>().is_ok()
    })
}
#[cfg(test)]
mod tests {
    use super::*;
    fn place(name: &str) -> Place {
        Place {
            id: 0,
            name: name.into(),
            address: "Main street 2".into(),
            coordinates: None,
        }
    }
    #[test]
    fn saved_places_keep_stable_identity_and_clear_deleted_shortcuts() {
        let mut saved = Saved::default();
        let id = saved.save(place("Home")).unwrap();
        saved.home = Some(id);
        saved.work = Some(id);
        let mut edited = saved.places[0].clone();
        edited.name = "Apartment".into();
        assert_eq!(saved.save(edited).unwrap(), id);
        assert!(saved.save(place("apartment")).is_err());
        saved.validate().unwrap();
        saved.remove(id);
        assert_eq!(saved.home, None);
        assert_eq!(saved.work, None);
        saved.validate().unwrap();
    }
    #[test]
    fn recent_searches_are_bounded_and_deduplicate() {
        let mut saved = Saved::default();
        for n in 0..40 {
            saved.remember(&format!("City {n}")).unwrap();
        }
        saved.remember("CITY 39").unwrap();
        assert_eq!(saved.recent.len(), 32);
        assert_eq!(saved.recent[0], "CITY 39");
        saved.validate().unwrap();
    }
    #[test]
    fn routing_encodes_user_text_and_validates_coordinate_ranges() {
        let url = directions_url("50.091, 18.219", "A&B #office", TravelMode::Walking).unwrap();
        assert!(url.contains("destination=A%26B%20%23office&travelmode=walking"));
        assert!(url.contains("origin=50.091%2C%2018.219"));
        assert!(!directions_url("", "Office", TravelMode::Transit)
            .unwrap()
            .contains("origin="));
        assert!(directions_url("91,0", "Office", TravelMode::Driving).is_err());
        assert!(directions_url("", "0,NaN", TravelMode::Driving).is_err());
        assert!(directions_url("", "", TravelMode::Driving).is_err());
        assert!(directions_url(&"ą".repeat(256), &"ą".repeat(256), TravelMode::Driving).is_err());
        let value = Place {
            id: 1,
            name: "City".into(),
            address: String::new(),
            coordinates: Some(Coordinates::new(50.091, 18.219).unwrap()),
        };
        assert!(value
            .map_url()
            .unwrap()
            .starts_with("https://www.openstreetmap.org/?mlat="));
    }
}
