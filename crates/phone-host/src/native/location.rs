//! Explicit one-shot geolocation. Never called for background observation.
#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "macos")]
mod macos;
#[cfg(windows)]
mod windows;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Position {
    pub latitude: f64,
    pub longitude: f64,
    pub accuracy_metres: f64,
}

impl Position {
    pub fn validate(self) -> Result<Self, String> {
        if !self.latitude.is_finite()
            || !self.longitude.is_finite()
            || !self.accuracy_metres.is_finite()
            || self.latitude.abs() > 90.0
            || self.longitude.abs() > 180.0
            || self.accuracy_metres < 0.0
        {
            return Err("The location provider returned an invalid position.".into());
        }
        Ok(self)
    }
    pub fn map_url(self) -> Result<String, String> {
        let value = self.validate()?;
        Ok(format!(
            "https://www.openstreetmap.org/?mlat={:.6}&mlon={:.6}#map=15/{:.6}/{:.6}",
            value.latitude, value.longitude, value.latitude, value.longitude
        ))
    }
}

/// Poll this on the UI dispatcher: Windows access consent must be initiated on
/// the foreground UI thread. Blocking Unix APIs use the separate system worker.
pub async fn locate() -> Result<Position, String> {
    #[cfg(windows)]
    {
        windows::locate().await?.validate()
    }
    #[cfg(target_os = "macos")]
    {
        super::background::run_system(macos::locate)
            .await?
            .validate()
    }
    #[cfg(target_os = "linux")]
    {
        super::background::run_system(linux::locate)
            .await?
            .validate()
    }
    #[cfg(not(any(windows, target_os = "macos", target_os = "linux")))]
    {
        Err("Native location is unavailable on this platform.".into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn coordinates_are_finite_bounded_and_encoded_without_user_markup() {
        let position = Position {
            latitude: 50.09,
            longitude: 18.22,
            accuracy_metres: 12.0,
        };
        assert!(position
            .map_url()
            .unwrap()
            .contains("mlat=50.090000&mlon=18.220000"));
        assert!(Position {
            latitude: f64::NAN,
            ..position
        }
        .validate()
        .is_err());
        assert!(Position {
            longitude: 181.0,
            ..position
        }
        .validate()
        .is_err());
        assert!(Position {
            accuracy_metres: -1.0,
            ..position
        }
        .validate()
        .is_err());
    }
}
