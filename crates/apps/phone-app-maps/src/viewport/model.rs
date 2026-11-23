//! Web Mercator / XYZ tiles, following OpenStreetMap's slippy-map convention.
//! https://wiki.openstreetmap.org/wiki/Slippy_map_tilenames
use phone_app_support::geography::Coordinates;
use serde::{Deserialize, Serialize};
use std::f64::consts::PI;

pub const TILE_PIXELS: f64 = 256.0;
pub const MAX_LATITUDE: f64 = 85.051_128_779_806_6;
pub const MAX_DIMENSION: f64 = 768.0;
pub const MAX_VISIBLE: usize = 16;
pub const MAX_ZOOM: f64 = phone_sdk::host::maps::MAX_ZOOM as f64;

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Camera {
    pub center: Coordinates,
    pub zoom: f64,
}
impl Default for Camera {
    fn default() -> Self {
        Self {
            center: Coordinates {
                latitude: 20.0,
                longitude: 0.0,
            },
            zoom: 2.0,
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct TileKey {
    pub zoom: u8,
    pub x: u32,
    pub y: u32,
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Placement {
    pub key: TileKey,
    pub x: f64,
    pub y: f64,
    pub size: f64,
}

impl Camera {
    pub fn validate(self) -> Result<(), String> {
        self.center.validate()?;
        if self.center.latitude.abs() > MAX_LATITUDE
            || !self.zoom.is_finite()
            || !(0.0..=MAX_ZOOM).contains(&self.zoom)
        {
            return Err("The saved map viewport is outside its supported range.".into());
        }
        Ok(())
    }
    pub fn at(center: Coordinates, zoom: f64) -> Self {
        if center.validate().is_err() || !zoom.is_finite() {
            return Self::default();
        }
        Self {
            center: Coordinates {
                latitude: center.latitude.clamp(-MAX_LATITUDE, MAX_LATITUDE),
                longitude: wrap_longitude(center.longitude),
            },
            zoom: zoom.clamp(0.0, MAX_ZOOM),
        }
    }
    pub fn world(self) -> f64 {
        TILE_PIXELS * 2.0_f64.powf(self.zoom)
    }
    pub fn center_pixels(self) -> (f64, f64) {
        project(self.center, self.world())
    }
    fn from_pixels(self, x: f64, y: f64, size: (f64, f64)) -> Self {
        let world = self.world();
        let (_, height) = dimensions(size);
        let y = if height >= world {
            world / 2.0
        } else {
            y.clamp(height / 2.0, world - height / 2.0)
        };
        Self {
            center: unproject(x, y, world),
            ..self
        }
    }
    /// Pointer travel moves the map beneath the finger; east is positive x.
    pub fn pan(self, dx: f64, dy: f64, size: (f64, f64)) -> Self {
        if !dx.is_finite() || !dy.is_finite() {
            return self;
        }
        let (x, y) = self.center_pixels();
        self.from_pixels(x - dx, y - dy, size)
    }
    pub fn zoom_at(self, zoom: f64, point: (f64, f64), size: (f64, f64)) -> Self {
        if !zoom.is_finite() || !point.0.is_finite() || !point.1.is_finite() {
            return self;
        }
        let size = dimensions(size);
        let zoom = zoom.clamp(0.0, MAX_ZOOM);
        let (x, y) = self.center_pixels();
        let factor = 2.0_f64.powf(zoom - self.zoom);
        let dx = point.0 - size.0 / 2.0;
        let dy = point.1 - size.1 / 2.0;
        Self { zoom, ..self }.from_pixels((x + dx) * factor - dx, (y + dy) * factor - dy, size)
    }
    pub fn coordinate(self, point: (f64, f64), size: (f64, f64)) -> Coordinates {
        let size = dimensions(size);
        let (x, y) = self.center_pixels();
        unproject(
            x + point.0 - size.0 / 2.0,
            y + point.1 - size.1 / 2.0,
            self.world(),
        )
    }
    pub fn position(self, coordinates: Coordinates, size: (f64, f64)) -> Option<(f64, f64)> {
        if coordinates.validate().is_err() || coordinates.latitude.abs() > MAX_LATITUDE {
            return None;
        }
        let world = self.world();
        let size = dimensions(size);
        let (x, y) = project(coordinates, world);
        let (cx, cy) = self.center_pixels();
        let dx = (x - cx + world / 2.0).rem_euclid(world) - world / 2.0;
        Some((size.0 / 2.0 + dx, size.1 / 2.0 + y - cy))
    }
    /// Intersect exactly the visible viewport; no border prefetch or tile scan.
    pub fn visible(self, size: (f64, f64)) -> Vec<Placement> {
        let size = dimensions(size);
        let zoom = self.zoom.floor() as u8;
        let count = 1_i64 << zoom;
        let pixels = TILE_PIXELS * 2.0_f64.powf(self.zoom - f64::from(zoom));
        let (cx, cy) = self.center_pixels();
        let left = cx - size.0 / 2.0;
        let top = cy - size.1 / 2.0;
        let x0 = (left / pixels).floor() as i64;
        let x1 = ((left + size.0) / pixels).ceil() as i64;
        let y0 = (top / pixels).floor().max(0.0) as i64;
        let y1 = ((top + size.1) / pixels).ceil().min(count as f64) as i64;
        let mut result = Vec::new();
        for y in y0..y1 {
            for x in x0..x1 {
                result.push(Placement {
                    key: TileKey {
                        zoom,
                        x: x.rem_euclid(count) as u32,
                        y: y as u32,
                    },
                    x: x as f64 * pixels - left,
                    y: y as f64 * pixels - top,
                    size: pixels,
                });
            }
        }
        debug_assert!(result.len() <= MAX_VISIBLE);
        result
    }
    pub fn scale(self) -> (f64, String) {
        let meters_per_pixel =
            40_075_016.686 * self.center.latitude.to_radians().cos() / self.world();
        let target = meters_per_pixel * 90.0;
        let magnitude = 10.0_f64.powf(target.log10().floor());
        let meters = [5.0, 2.0, 1.0]
            .into_iter()
            .map(|value| value * magnitude)
            .find(|value| *value <= target)
            .unwrap_or(magnitude);
        let label = if meters >= 1000.0 {
            format!("{} km", (meters / 1000.0).round())
        } else {
            format!("{} m", meters.round())
        };
        (meters / meters_per_pixel, label)
    }
}
pub fn dimensions(size: (f64, f64)) -> (f64, f64) {
    let clamp = |value: f64| {
        if value.is_finite() {
            value.clamp(1.0, MAX_DIMENSION)
        } else {
            1.0
        }
    };
    (clamp(size.0), clamp(size.1))
}
fn wrap_longitude(value: f64) -> f64 {
    (value + 180.0).rem_euclid(360.0) - 180.0
}
fn project(coordinates: Coordinates, world: f64) -> (f64, f64) {
    let x = (wrap_longitude(coordinates.longitude) + 180.0) / 360.0 * world;
    let latitude = coordinates
        .latitude
        .clamp(-MAX_LATITUDE, MAX_LATITUDE)
        .to_radians();
    let y = (1.0 - latitude.tan().asinh() / PI) / 2.0 * world;
    (x, y.clamp(0.0, world))
}
fn unproject(x: f64, y: f64, world: f64) -> Coordinates {
    Coordinates {
        longitude: x.rem_euclid(world) / world * 360.0 - 180.0,
        latitude: (PI * (1.0 - 2.0 * y.clamp(0.0, world) / world))
            .sinh()
            .atan()
            .to_degrees()
            .clamp(-MAX_LATITUDE, MAX_LATITUDE),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn near(a: f64, b: f64) {
        assert!((a - b).abs() < 1e-7, "{a} != {b}");
    }
    #[test]
    fn projection_roundtrips_and_wraps_the_date_line() {
        for point in [(0.0, 0.0), (50.091, 18.219), (-33.8, 151.2), (85.0, -179.9)] {
            let camera = Camera::at(Coordinates::new(point.0, point.1).unwrap(), 14.0);
            let found = camera.coordinate((200.0, 150.0), (400.0, 300.0));
            near(found.latitude, point.0);
            near(found.longitude, point.1);
        }
        let camera =
            Camera::at(Coordinates::new(0.0, 179.9).unwrap(), 3.0).pan(-20.0, 0.0, (400.0, 300.0));
        assert!(camera.center.longitude < 0.0);
        assert!(Camera::at(Coordinates::new(90.0, 0.0).unwrap(), 3.0)
            .validate()
            .is_ok());
        assert!(Camera {
            center: Coordinates::new(0.0, 0.0).unwrap(),
            zoom: f64::NAN
        }
        .validate()
        .is_err());
    }
    #[test]
    fn zoom_preserves_the_geographic_anchor() {
        let camera = Camera::at(Coordinates::new(50.091, 18.219).unwrap(), 12.0);
        let size = (390.0, 320.0);
        let pointer = (80.0, 100.0);
        let before = camera.coordinate(pointer, size);
        let after = camera
            .zoom_at(14.75, pointer, size)
            .coordinate(pointer, size);
        near(before.latitude, after.latitude);
        near(before.longitude, after.longitude);
        assert_eq!(camera.pan(f64::NAN, 0.0, size), camera);
    }
    #[test]
    fn tile_requests_intersect_only_visible_pixels_and_remain_bounded() {
        for zoom in [0.0, 0.5, 2.0, 8.75, 19.0] {
            for longitude in [-179.99, 0.0, 179.99] {
                for latitude in [-MAX_LATITUDE, 0.0, MAX_LATITUDE] {
                    let camera = Camera::at(Coordinates::new(latitude, longitude).unwrap(), zoom);
                    let size = (768.0, 768.0);
                    let tiles = camera.visible(size);
                    assert!(!tiles.is_empty() && tiles.len() <= MAX_VISIBLE);
                    for tile in tiles {
                        phone_sdk::host::maps::validate(tile.key.zoom, tile.key.x, tile.key.y)
                            .unwrap();
                        assert!(
                            tile.x < size.0
                                && tile.x + tile.size > 0.0
                                && tile.y < size.1
                                && tile.y + tile.size > 0.0
                        );
                    }
                }
            }
        }
        let world = Camera::at(Coordinates::new(0.0, 0.0).unwrap(), 0.0);
        let keys = world
            .visible((768.0, 300.0))
            .into_iter()
            .map(|tile| tile.key)
            .collect::<std::collections::BTreeSet<_>>();
        assert_eq!(keys.len(), 1, "Repeated worlds reuse one tile download");
    }
}
