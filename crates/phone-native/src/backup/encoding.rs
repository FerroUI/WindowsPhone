//! Strict RFC 4648 base64 for the existing portable JSON backup format.
const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

pub fn encode(bytes: &[u8]) -> String {
    let mut output = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let a = chunk[0];
        let b = chunk.get(1).copied().unwrap_or(0);
        let c = chunk.get(2).copied().unwrap_or(0);
        output.push(ALPHABET[(a >> 2) as usize] as char);
        output.push(ALPHABET[(((a & 3) << 4) | (b >> 4)) as usize] as char);
        output.push(if chunk.len() > 1 {
            ALPHABET[(((b & 15) << 2) | (c >> 6)) as usize] as char
        } else {
            '='
        });
        output.push(if chunk.len() > 2 {
            ALPHABET[(c & 63) as usize] as char
        } else {
            '='
        });
    }
    output
}

pub fn decode(text: &str, limit: usize) -> Result<Vec<u8>, String> {
    if text.len() % 4 != 0 || text.len() / 4 > limit.div_ceil(3) {
        return Err("Invalid or oversized base64 media data.".into());
    }
    let mut output = Vec::with_capacity((text.len() / 4 * 3).min(limit));
    let chunks = text.as_bytes().chunks_exact(4);
    let count = chunks.len();
    for (index, chunk) in chunks.enumerate() {
        let a = value(chunk[0])?;
        let b = value(chunk[1])?;
        let c = if chunk[2] == b'=' {
            0
        } else {
            value(chunk[2])?
        };
        let d = if chunk[3] == b'=' {
            0
        } else {
            value(chunk[3])?
        };
        let padding = if chunk[2] == b'=' {
            2
        } else if chunk[3] == b'=' {
            1
        } else {
            0
        };
        if padding > 0 && (index + 1 != count || chunk[3] != b'=')
            || padding == 2 && b & 15 != 0
            || padding == 1 && c & 3 != 0
            || output.len() + 3 - padding > limit
        {
            return Err("Invalid or oversized base64 media data.".into());
        }
        output.push(a << 2 | b >> 4);
        if padding < 2 {
            output.push(b << 4 | c >> 2);
        }
        if padding == 0 {
            output.push(c << 6 | d);
        }
    }
    Ok(output)
}

fn value(byte: u8) -> Result<u8, String> {
    match byte {
        b'A'..=b'Z' => Ok(byte - b'A'),
        b'a'..=b'z' => Ok(byte - b'a' + 26),
        b'0'..=b'9' => Ok(byte - b'0' + 52),
        b'+' => Ok(62),
        b'/' => Ok(63),
        _ => Err("Invalid base64 media data.".into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn standard_vectors_and_all_byte_values_roundtrip() {
        for (raw, encoded) in [
            ("", ""),
            ("f", "Zg=="),
            ("fo", "Zm8="),
            ("foo", "Zm9v"),
            ("foob", "Zm9vYg=="),
            ("fooba", "Zm9vYmE="),
            ("foobar", "Zm9vYmFy"),
        ] {
            assert_eq!(encode(raw.as_bytes()), encoded);
            assert_eq!(decode(encoded, raw.len()).unwrap(), raw.as_bytes());
        }
        let bytes: Vec<_> = (0..=255).collect();
        assert_eq!(decode(&encode(&bytes), bytes.len()).unwrap(), bytes);
    }
    #[test]
    fn rejects_junk_overflow_and_noncanonical_padding() {
        for invalid in [
            "Zg", "Zg===", "Zg==AAAA", "====", "AA=A", "Zh==", "Zm9=", "Zg==\n", "🔥🔥",
        ] {
            assert!(decode(invalid, 100).is_err(), "{invalid}");
        }
        assert!(decode("Zm9v", 2).is_err());
        assert_eq!(decode("Zg==", 1).unwrap(), b"f");
    }
}
