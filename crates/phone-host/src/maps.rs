//! The built-in raster service is replaceable through HostService::Network.
//! Only explicitly visible Web Mercator tiles are requested by the Maps app.
pub const MAX_ZOOM: u8 = 19;
pub const MAX_TILE_BYTES: u32 = 512 * 1024;
pub const CACHE_SECONDS: u64 = 7 * 24 * 60 * 60;
pub const ATTRIBUTION: &str = "© OpenStreetMap contributors";
pub const ATTRIBUTION_URL: &str = "https://www.openstreetmap.org/copyright";
pub fn validate(zoom: u8, x: u32, y: u32) -> Result<(), String> {
    if zoom > MAX_ZOOM || x >= (1_u32 << zoom) || y >= (1_u32 << zoom) {
        Err("Invalid map tile coordinate.".into())
    } else {
        Ok(())
    }
}
pub fn url(zoom: u8, x: u32, y: u32) -> Result<String, String> {
    validate(zoom, x, y)?;
    Ok(format!("https://tile.openstreetmap.org/{zoom}/{x}/{y}.png"))
}
pub fn validate_png(bytes: &[u8]) -> Result<(), String> {
    if bytes.len() < 33
        || bytes.len() > MAX_TILE_BYTES as usize
        || &bytes[..8] != b"\x89PNG\r\n\x1a\n"
        || &bytes[12..16] != b"IHDR"
        || bytes[8..12] != [0, 0, 0, 13]
        || u32::from_be_bytes(bytes[16..20].try_into().unwrap()) != 256
        || u32::from_be_bytes(bytes[20..24].try_into().unwrap()) != 256
    {
        Err("The map service returned an invalid or oversized 256-pixel PNG tile.".into())
    } else {
        Ok(())
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn coordinates_are_bounded_before_shift_or_network() {
        assert!(validate(255, 0, 0).is_err());
        assert!(validate(0, 1, 0).is_err());
        assert!(validate(19, 1 << 19, 0).is_err());
        assert_eq!(
            url(0, 0, 0).unwrap(),
            "https://tile.openstreetmap.org/0/0/0.png"
        );
        assert!(validate_png(b"not an image").is_err());
    }
}
