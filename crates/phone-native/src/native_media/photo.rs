//! Normalize actual camera pixels because the pinned renderers ignore EXIF orientation.
use std::io::{self, Write};
use zune_core::{bytestream::ZCursor, colorspace::ColorSpace, options::DecoderOptions};

const PIXEL_LIMIT: usize = 16 * 1024 * 1024;
const ENCODED_LIMIT: usize = crate::asset_store::IMAGE_LIMIT;

pub fn oriented_jpeg(input: &[u8], rotation: u16, mirrored: bool) -> Result<Vec<u8>, String> {
    if !matches!(rotation, 0 | 90 | 180 | 270) {
        return Err("Camera returned an unsupported photo orientation.".into());
    }
    if input.len() > ENCODED_LIMIT {
        return Err("Camera photo exceeds the encoded image limit.".into());
    }
    let options = DecoderOptions::default()
        .jpeg_set_out_colorspace(ColorSpace::RGB)
        .set_max_width(16_384)
        .set_max_height(16_384);
    let mut decoder = zune_jpeg::JpegDecoder::new_with_options(ZCursor::new(input), options);
    decoder
        .decode_headers()
        .map_err(|error| format!("Camera JPEG: {error}"))?;
    let (width, height) = decoder
        .dimensions()
        .ok_or("Camera JPEG has no dimensions.")?;
    if width == 0
        || height == 0
        || width
            .checked_mul(height)
            .is_none_or(|pixels| pixels > PIXEL_LIMIT)
    {
        return Err("Camera photo exceeds the decoded image limit.".into());
    }
    // Keep original compression and metadata when pixel normalization is unnecessary.
    if rotation == 0 && !mirrored {
        return Ok(input.to_vec());
    }
    let pixels = decoder
        .decode()
        .map_err(|error| format!("Camera JPEG: {error}"))?;
    if decoder.output_colorspace() != Some(ColorSpace::RGB) || pixels.len() != width * height * 3 {
        return Err("Camera JPEG could not be decoded to RGB.".into());
    }
    let (pixels, width, height) = orient_rgb(&pixels, width, height, rotation, mirrored);
    let mut output = BoundedOutput(Vec::new());
    jpeg_encoder::Encoder::new(&mut output, 92)
        .encode(
            &pixels,
            width as u16,
            height as u16,
            jpeg_encoder::ColorType::Rgb,
        )
        .map_err(|error| format!("Camera photo could not be normalized: {error}"))?;
    Ok(output.0)
}

fn orient_rgb(
    source: &[u8],
    width: usize,
    height: usize,
    rotation: u16,
    mirrored: bool,
) -> (Vec<u8>, usize, usize) {
    let (target_width, target_height) = if rotation == 90 || rotation == 270 {
        (height, width)
    } else {
        (width, height)
    };
    let mut target = vec![0; source.len()];
    for y in 0..height {
        for x in 0..width {
            let (mut dx, dy) = match rotation {
                90 => (height - 1 - y, x),
                180 => (width - 1 - x, height - 1 - y),
                270 => (y, width - 1 - x),
                _ => (x, y),
            };
            if mirrored {
                dx = target_width - 1 - dx;
            }
            let source_offset = (y * width + x) * 3;
            let destination_offset = (dy * target_width + dx) * 3;
            target[destination_offset..destination_offset + 3]
                .copy_from_slice(&source[source_offset..source_offset + 3]);
        }
    }
    (target, target_width, target_height)
}

struct BoundedOutput(Vec<u8>);
impl Write for BoundedOutput {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        if self
            .0
            .len()
            .checked_add(bytes.len())
            .is_none_or(|length| length > ENCODED_LIMIT)
        {
            return Err(io::Error::other("Normalized camera photo exceeds 16 MiB."));
        }
        self.0.extend_from_slice(bytes);
        Ok(bytes.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rotation_and_mirroring_match_all_eight_native_exif_orientations() {
        let source: Vec<u8> = [1, 2, 3, 4, 5, 6]
            .into_iter()
            .flat_map(|value| [value; 3])
            .collect();
        let cases = [
            (0, false, [1, 2, 3, 4, 5, 6]),
            (0, true, [3, 2, 1, 6, 5, 4]),
            (180, false, [6, 5, 4, 3, 2, 1]),
            (180, true, [4, 5, 6, 1, 2, 3]),
            (90, true, [1, 4, 2, 5, 3, 6]),
            (90, false, [4, 1, 5, 2, 6, 3]),
            (270, true, [6, 3, 5, 2, 4, 1]),
            (270, false, [3, 6, 2, 5, 1, 4]),
        ];
        for (rotation, mirrored, expected) in cases {
            let (pixels, width, height) = orient_rgb(&source, 3, 2, rotation, mirrored);
            assert_eq!(
                pixels
                    .chunks_exact(3)
                    .map(|pixel| pixel[0])
                    .collect::<Vec<_>>(),
                expected
            );
            assert_eq!(
                (width, height),
                if rotation == 90 || rotation == 270 {
                    (2, 3)
                } else {
                    (3, 2)
                }
            );
        }
    }
    #[test]
    fn captured_jpeg_is_valid_rotated_and_unrotated_bytes_are_preserved() {
        let source = vec![128; 3 * 2 * 3];
        let mut jpeg = Vec::new();
        jpeg_encoder::Encoder::new(&mut jpeg, 92)
            .encode(&source, 3, 2, jpeg_encoder::ColorType::Rgb)
            .unwrap();
        assert_eq!(oriented_jpeg(&jpeg, 0, false).unwrap(), jpeg);
        let rotated = oriented_jpeg(&jpeg, 90, false).unwrap();
        let mut decoder = zune_jpeg::JpegDecoder::new(ZCursor::new(rotated.as_slice()));
        decoder.decode_headers().unwrap();
        assert_eq!(decoder.dimensions(), Some((2, 3)));
        assert!(oriented_jpeg(&jpeg, 45, false).is_err());
        assert!(oriented_jpeg(b"invalid JPEG", 0, false).is_err());
    }
}
