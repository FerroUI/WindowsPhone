use super::Position;
use windows::{
    Devices::Geolocation::{GeolocationAccessStatus, Geolocator, PositionAccuracy},
    Foundation::TimeSpan,
    Win32::System::WinRT::{RoInitialize, RoUninitialize, RO_INIT_SINGLETHREADED},
};

struct Runtime(bool);
impl Drop for Runtime {
    fn drop(&mut self) {
        if self.0 {
            unsafe {
                RoUninitialize();
            }
        }
    }
}

pub(super) async fn locate() -> Result<Position, String> {
    let runtime = match unsafe { RoInitialize(RO_INIT_SINGLETHREADED) } {
        Ok(()) => Runtime(true),
        Err(error) if error.code().0 == 0x80010106u32 as i32 => Runtime(false),
        Err(error) => return Err(error.to_string()),
    };
    let access = Geolocator::RequestAccessAsync()
        .map_err(|e| e.to_string())?
        .await
        .map_err(|e| e.to_string())?;
    if access != GeolocationAccessStatus::Allowed {
        return Err("Location access was not allowed by Windows.".into());
    }
    let locator = Geolocator::new().map_err(|e| e.to_string())?;
    locator
        .SetDesiredAccuracy(PositionAccuracy::High)
        .map_err(|e| e.to_string())?;
    let position = locator
        .GetGeopositionAsyncWithAgeAndTimeout(
            TimeSpan {
                Duration: 60 * 10_000_000,
            },
            TimeSpan {
                Duration: 25 * 10_000_000,
            },
        )
        .map_err(|e| e.to_string())?
        .await
        .map_err(|e| e.to_string())?;
    let coordinate = position.Coordinate().map_err(|e| e.to_string())?;
    let result = Position {
        latitude: coordinate.Latitude().map_err(|e| e.to_string())?,
        longitude: coordinate.Longitude().map_err(|e| e.to_string())?,
        accuracy_metres: coordinate.Accuracy().map_err(|e| e.to_string())?,
    };
    drop(runtime);
    Ok(result)
}
