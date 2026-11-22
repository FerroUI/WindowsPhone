//! Read dimensions before a native codec can allocate its full-size decode buffer.
pub(super) const MAX_SOURCE_PIXELS: u64 = 16 * 1024 * 1024;
pub(super) const MAX_ENCODED_BYTES: usize = 16 * 1024 * 1024;

#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct Dimensions {
    pub width: u32,
    pub height: u32,
}

pub(super) fn dimensions(bytes: &[u8]) -> Result<Dimensions, String> {
    if bytes.len() > MAX_ENCODED_BYTES {
        return Err("Image exceeds the 16 MB limit.".into());
    }
    let dimensions =
        if bytes.starts_with(b"\x89PNG\r\n\x1a\n") && bytes.get(12..16) == Some(b"IHDR") {
            Some((be32(bytes, 16)?, be32(bytes, 20)?))
        } else if bytes.starts_with(b"GIF87a") || bytes.starts_with(b"GIF89a") {
            Some((le16(bytes, 6)? as u32, le16(bytes, 8)? as u32))
        } else if bytes.starts_with(&[0xff, 0xd8]) {
            jpeg(bytes)
        } else if bytes.get(..4) == Some(b"RIFF") && bytes.get(8..12) == Some(b"WEBP") {
            webp(bytes)
        } else {
            None
        };
    let (width, height) = dimensions.ok_or("Image dimensions could not be read safely.")?;
    if width == 0
        || height == 0
        || width > 16384
        || height > 16384
        || u64::from(width) * u64::from(height) > MAX_SOURCE_PIXELS
    {
        return Err("Image exceeds the safe 16 megapixel decode limit.".into());
    }
    Ok(Dimensions { width, height })
}

fn be32(bytes: &[u8], at: usize) -> Result<u32, String> {
    bytes
        .get(at..at + 4)
        .and_then(|s| s.try_into().ok())
        .map(u32::from_be_bytes)
        .ok_or_else(|| "Incomplete image header.".into())
}
fn le16(bytes: &[u8], at: usize) -> Result<u16, String> {
    bytes
        .get(at..at + 2)
        .and_then(|s| s.try_into().ok())
        .map(u16::from_le_bytes)
        .ok_or_else(|| "Incomplete image header.".into())
}
fn le24(bytes: &[u8], at: usize) -> Option<u32> {
    let s = bytes.get(at..at + 3)?;
    Some(u32::from(s[0]) | (u32::from(s[1]) << 8) | (u32::from(s[2]) << 16))
}
fn jpeg(bytes: &[u8]) -> Option<(u32, u32)> {
    let mut at = 2;
    while at < bytes.len() {
        if bytes[at] != 0xff {
            return None;
        }
        while bytes.get(at) == Some(&0xff) {
            at += 1;
        }
        let marker = *bytes.get(at)?;
        at += 1;
        if matches!(marker, 0x01 | 0xd0..=0xd8) {
            continue;
        }
        if matches!(marker, 0xd9 | 0xda | 0x00) {
            return None;
        }
        let length = u16::from_be_bytes(bytes.get(at..at + 2)?.try_into().ok()?) as usize;
        if length < 2 || at.checked_add(length)? > bytes.len() {
            return None;
        }
        if matches!(marker, 0xc0..=0xc3 | 0xc5..=0xc7 | 0xc9..=0xcb | 0xcd..=0xcf) {
            if length < 8 {
                return None;
            }
            let height = u16::from_be_bytes(bytes.get(at + 3..at + 5)?.try_into().ok()?);
            let width = u16::from_be_bytes(bytes.get(at + 5..at + 7)?.try_into().ok()?);
            return Some((width as u32, height as u32));
        }
        at += length;
    }
    None
}
fn webp(bytes: &[u8]) -> Option<(u32, u32)> {
    let mut at: usize = 12;
    while at.checked_add(8)? <= bytes.len() {
        let kind = bytes.get(at..at + 4)?;
        let size = u32::from_le_bytes(bytes.get(at + 4..at + 8)?.try_into().ok()?) as usize;
        let start = at + 8;
        let end = start.checked_add(size)?;
        let data = bytes.get(start..end)?;
        match kind {
            b"VP8X" if data.len() >= 10 => return Some((le24(data, 4)? + 1, le24(data, 7)? + 1)),
            b"VP8 " if data.len() >= 10 && data.get(3..6) == Some(&[0x9d, 0x01, 0x2a]) => {
                return Some((
                    (le16(data, 6).ok()? & 0x3fff) as u32,
                    (le16(data, 8).ok()? & 0x3fff) as u32,
                ));
            }
            b"VP8L" if data.len() >= 5 && data[0] == 0x2f => {
                let bits = u32::from_le_bytes(data[1..5].try_into().ok()?);
                return Some(((bits & 0x3fff) + 1, ((bits >> 14) & 0x3fff) + 1));
            }
            _ => {}
        }
        at = end.checked_add(size % 2)?;
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    fn png(w: u32, h: u32) -> Vec<u8> {
        let mut data = b"\x89PNG\r\n\x1a\n\0\0\0\rIHDR".to_vec();
        data.extend(w.to_be_bytes());
        data.extend(h.to_be_bytes());
        data
    }
    #[test]
    fn image_headers_are_bounded_before_decode() {
        assert_eq!(
            dimensions(&png(600, 400)).unwrap(),
            Dimensions {
                width: 600,
                height: 400
            }
        );
        assert!(dimensions(&png(1, 100000)).is_err());
        assert!(dimensions(&png(8192, 8192)).is_err());
        assert!(dimensions(&png(0, 1)).is_err());
        assert!(dimensions(b"\x89PNG\r\n\x1a\n").is_err());
        assert_eq!(
            dimensions(b"GIF89a\x20\0\x10\0").unwrap(),
            Dimensions {
                width: 32,
                height: 16
            }
        );
    }
    #[test]
    fn jpeg_segments_and_webp_canvas_are_checked() {
        let jpeg = [
            0xff, 0xd8, 0xff, 0xe0, 0, 4, 1, 2, 0xff, 0xc0, 0, 8, 8, 0, 100, 0, 200, 3,
        ];
        assert_eq!(
            dimensions(&jpeg).unwrap(),
            Dimensions {
                width: 200,
                height: 100
            }
        );
        assert!(dimensions(&jpeg[..15]).is_err());
        let mut webp = b"RIFF\x16\0\0\0WEBPVP8X\x0a\0\0\0\0\0\0\0".to_vec();
        webp.extend([99, 0, 0, 49, 0, 0]);
        assert_eq!(
            dimensions(&webp).unwrap(),
            Dimensions {
                width: 100,
                height: 50
            }
        );
    }
}
