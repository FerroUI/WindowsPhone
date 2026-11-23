//! Code 39 symbol geometry for explicitly selected membership-card barcodes.
//! Encoding table from ZXing Code39Reader (Copyright 2008 ZXing authors),
//! Apache-2.0; see ../LICENSE-APACHE-2.0. Rendering and admission are original.
pub const ALPHABET: &str = "0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ-. $/+%";
const ENCODINGS: [u16; 43] = [
    0x034, 0x121, 0x061, 0x160, 0x031, 0x130, 0x070, 0x025, 0x124, 0x064, 0x109, 0x049, 0x148,
    0x019, 0x118, 0x058, 0x00D, 0x10C, 0x04C, 0x01C, 0x103, 0x043, 0x142, 0x013, 0x112, 0x052,
    0x007, 0x106, 0x046, 0x016, 0x181, 0x0C1, 0x1C0, 0x091, 0x190, 0x0D0, 0x085, 0x184, 0x0C4,
    0x0A8, 0x0A2, 0x08A, 0x02A,
];
const STOP: u16 = 0x094;
/// No automatic case conversion, extended encoding or checksum: the payload is exact.
pub fn code39_modules(value: &str) -> Result<Vec<bool>, String> {
    if value.is_empty()
        || value.len() > 20
        || value
            .bytes()
            .any(|byte| !ALPHABET.as_bytes().contains(&byte))
    {
        return Err(
            "Code 39 accepts 1–20 uppercase letters, digits, spaces and - . $ / + %.".into(),
        );
    }
    let patterns = std::iter::once(STOP)
        .chain(
            value
                .bytes()
                .map(|byte| ENCODINGS[ALPHABET.bytes().position(|v| v == byte).unwrap()]),
        )
        .chain(std::iter::once(STOP));
    let mut output = vec![false; 10];
    for (index, pattern) in patterns.enumerate() {
        if index > 0 {
            output.push(false);
        }
        for bit in 0..9 {
            let width = if pattern & (1 << (8 - bit)) != 0 {
                2
            } else {
                1
            };
            output.extend(std::iter::repeat_n(bit % 2 == 0, width));
        }
    }
    output.extend(std::iter::repeat_n(false, 10));
    Ok(output)
}
pub fn code39(value: &str) -> Result<ferroui_view::prelude::Element, String> {
    use ferroui_view::prelude::*;
    let modules = code39_modules(value)?;
    let width = modules.len() as f64;
    let mut bars = canvas().width(width).height(80.0);
    let mut i = 0;
    while i < modules.len() {
        if !modules[i] {
            i += 1;
            continue;
        }
        let start = i;
        while i < modules.len() && modules[i] {
            i += 1;
        }
        bars=bars.child(rsx!{Border{Canvas.left:start as f64,width:(i-start)as f64,height:80.0,background:"Black"}});
    }
    Ok(
        rsx! {Border{background:"White",padding:(0.0,12.0),horizontal_alignment:ferroui_base::layout::HorizontalAlignment::Center,{bars}}},
    )
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn exact_symbols_have_quiet_zones_and_three_wide_elements() {
        assert_eq!(
            ENCODINGS
                .iter()
                .copied()
                .collect::<std::collections::HashSet<_>>()
                .len(),
            43
        );
        assert!(ENCODINGS.iter().all(|pattern| pattern.count_ones() == 3));
        let bits = code39_modules("A1").unwrap();
        assert_eq!(bits.len(), 20 + 25 + 13 * 2);
        assert!(bits[..10].iter().all(|bit| !*bit));
        assert!(bits[bits.len() - 10..].iter().all(|bit| !*bit));
        let patterns = bits[10..bits.len() - 10]
            .chunks(13)
            .map(|symbol| {
                let symbol = &symbol[..12];
                let mut runs = Vec::new();
                let mut start = 0;
                while start < symbol.len() {
                    let mut end = start + 1;
                    while end < symbol.len() && symbol[end] == symbol[start] {
                        end += 1;
                    }
                    runs.push(end - start);
                    start = end;
                }
                runs.iter()
                    .fold(0u16, |pattern, run| (pattern << 1) | u16::from(*run == 2))
            })
            .collect::<Vec<_>>();
        assert_eq!(patterns, vec![STOP, 0x109, 0x121, STOP]);
    }
    #[test]
    fn rejects_silent_case_changes_and_unsupported_symbols() {
        for value in ["", "abc", "A*B", "123456789012345678901", "é"] {
            assert!(code39_modules(value).is_err());
        }
    }
}
