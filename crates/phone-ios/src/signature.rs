//! Read the executing image's signed entitlement slot using public file APIs.
//! iOS validates the executable signature at launch. A loose plist, build flag,
//! provisioning profile, or environment variable is never a capability grant.
//! Unknown containers and signatures without the XML slot fail closed. The
//! actual telephony framework still authorizes every operation independently.

const MAX_COMMANDS: usize = 1024 * 1024;
const MAX_SIGNATURE: usize = 4 * 1024 * 1024;
const DIALER: u32 = 1;
const CARRIER: u32 = 2;

fn u32_at(data: &[u8], offset: usize, big_endian: bool) -> Option<u32> {
    let bytes: [u8; 4] = data.get(offset..offset.checked_add(4)?)?.try_into().ok()?;
    Some(if big_endian {
        u32::from_be_bytes(bytes)
    } else {
        u32::from_le_bytes(bytes)
    })
}

fn signature_location(header: &[u8], commands: &[u8], file_size: u64) -> Option<(u64, usize)> {
    // Packaged iOS executables are thin, little-endian Mach-O 64 images. A fat
    // container is rejected rather than interpreting the wrong architecture.
    if u32_at(header, 0, false)? != 0xfeed_facf {
        return None;
    }
    let count = usize::try_from(u32_at(header, 16, false)?).ok()?;
    let size = usize::try_from(u32_at(header, 20, false)?).ok()?;
    if count > 65_535 || size != commands.len() || size > MAX_COMMANDS {
        return None;
    }
    let mut cursor = 0_usize;
    let mut result = None;
    for _ in 0..count {
        let command = u32_at(commands, cursor, false)?;
        let size = usize::try_from(u32_at(commands, cursor.checked_add(4)?, false)?).ok()?;
        let end = cursor.checked_add(size)?;
        if size < 8 || size % 8 != 0 || end > commands.len() {
            return None;
        }
        if command == 0x1d {
            if size != 16 || result.is_some() {
                return None;
            }
            let offset = u64::from(u32_at(commands, cursor + 8, false)?);
            let length = usize::try_from(u32_at(commands, cursor + 12, false)?).ok()?;
            if !(12..=MAX_SIGNATURE).contains(&length)
                || offset < 32 + commands.len() as u64
                || offset.checked_add(length as u64)? > file_size
            {
                return None;
            }
            result = Some((offset, length));
        }
        cursor = end;
    }
    if cursor != commands.len() {
        return None;
    }
    result
}

fn signed_entitlements(signature: &[u8]) -> Option<u32> {
    if signature.len() > MAX_SIGNATURE || u32_at(signature, 0, true)? != 0xfade_0cc0 {
        return None;
    }
    let length = usize::try_from(u32_at(signature, 4, true)?).ok()?;
    let count = usize::try_from(u32_at(signature, 8, true)?).ok()?;
    let table_end = 12_usize.checked_add(count.checked_mul(8)?)?;
    if length > signature.len()
        || signature.len() - length > 16_384
        || signature[length..].iter().any(|value| *value != 0)
        || count > 4096
        || table_end > length
    {
        return None;
    }
    let mut ranges = Vec::new();
    let mut slots = std::collections::BTreeSet::new();
    let mut xml = None;
    for index in 0..count {
        let kind = u32_at(signature, 12 + index * 8, true)?;
        if !slots.insert(kind) {
            return None;
        }
        let offset = usize::try_from(u32_at(signature, 16 + index * 8, true)?).ok()?;
        if offset < table_end || offset.checked_add(8)? > length {
            return None;
        }
        let blob_length = usize::try_from(u32_at(signature, offset + 4, true)?).ok()?;
        let end = offset.checked_add(blob_length)?;
        if blob_length < 8 || end > length {
            return None;
        }
        if ranges
            .iter()
            .any(|&(start, finish)| offset < finish && start < end)
        {
            return None;
        }
        ranges.push((offset, end));
        if kind == 5 {
            if xml.is_some()
                || blob_length > 65_536
                || u32_at(signature, offset, true)? != 0xfade_7171
            {
                return None;
            }
            xml = Some(std::str::from_utf8(&signature[offset + 8..end]).ok()?);
        }
    }
    let xml = xml?;
    let document = roxmltree::Document::parse_with_options(
        xml,
        roxmltree::ParsingOptions {
            // Apple emits the standard plist DOCTYPE. roxmltree never fetches an
            // external DTD; reject entity declarations to keep this a plain plist.
            allow_dtd: true,
            nodes_limit: 4096,
        },
    )
    .ok()?;
    if xml.contains("<!ENTITY") {
        return None;
    }
    let root = document.root_element();
    if root.tag_name().name() != "plist" {
        return None;
    }
    let mut children = root.children().filter(|node| node.is_element());
    let dictionary = children.next()?;
    if children.next().is_some() || dictionary.tag_name().name() != "dict" {
        return None;
    }
    let mut items = dictionary.children().filter(|node| node.is_element());
    let mut seen = std::collections::BTreeSet::new();
    let mut capabilities = 0;
    while let Some(key) = items.next() {
        if key.tag_name().name() != "key" || key.children().any(|node| node.is_element()) {
            return None;
        }
        let key = key.text()?;
        if !seen.insert(key) {
            return None;
        }
        let value = items.next()?;
        let bit = match key {
            "com.apple.developer.dialing-app" => DIALER,
            "com.apple.developer.carrier-messaging-app" => CARRIER,
            _ => continue,
        };
        if value.tag_name().name() == "true"
            && !value
                .children()
                .any(|node| node.is_element() || node.is_text())
        {
            capabilities |= bit;
        } else if value.tag_name().name() != "false" {
            return None;
        }
    }
    Some(capabilities)
}

#[cfg(all(target_os = "ios", feature = "ios26-telephony"))]
fn current_image_entitlements() -> Option<u32> {
    use std::io::{Read, Seek, SeekFrom};
    let path = std::env::current_exe().ok()?;
    let mut file = std::fs::File::open(path).ok()?;
    let file_size = file.metadata().ok()?.len();
    let mut header = [0_u8; 32];
    file.read_exact(&mut header).ok()?;
    let size = usize::try_from(u32_at(&header, 20, false)?).ok()?;
    if size > MAX_COMMANDS {
        return None;
    }
    let mut commands = vec![0; size];
    file.read_exact(&mut commands).ok()?;
    let (offset, length) = signature_location(&header, &commands, file_size)?;
    file.seek(SeekFrom::Start(offset)).ok()?;
    let mut signature = vec![0; length];
    file.read_exact(&mut signature).ok()?;
    signed_entitlements(&signature)
}

#[cfg(all(target_os = "ios", feature = "ios26-telephony"))]
#[unsafe(no_mangle)]
pub extern "C" fn wp_ios_signed_capabilities() -> u32 {
    static OBSERVED: std::sync::OnceLock<u32> = std::sync::OnceLock::new();
    *OBSERVED.get_or_init(|| current_image_entitlements().unwrap_or(0))
}

#[cfg(test)]
mod tests {
    use super::*;
    fn signed(xml: &str) -> Vec<u8> {
        let mut bytes = Vec::new();
        for word in [
            0xfade_0cc0,
            28 + xml.len() as u32,
            1,
            5,
            20,
            0xfade_7171,
            8 + xml.len() as u32,
        ] {
            bytes.extend_from_slice(&word.to_be_bytes());
        }
        bytes.extend_from_slice(xml.as_bytes());
        bytes
    }
    #[test]
    fn grants_come_only_from_boolean_values_in_the_signed_root_dictionary() {
        assert_eq!(
            signed_entitlements(&signed(
                r#"<plist><dict><key>com.apple.developer.dialing-app</key><true/><key>com.apple.developer.carrier-messaging-app</key><false/></dict></plist>"#
            )),
            Some(DIALER)
        );
        assert_eq!(
            signed_entitlements(&signed(
                r#"<plist><dict><key>other</key><dict><key>com.apple.developer.dialing-app</key><true/></dict></dict></plist>"#
            )),
            Some(0)
        );
        assert!(signed_entitlements(&signed(r#"<plist><dict><key>com.apple.developer.dialing-app</key><string>true</string></dict></plist>"#)).is_none());
        assert!(signed_entitlements(&signed(r#"<plist><dict><key>com.apple.developer.dialing-app</key><true/><key>com.apple.developer.dialing-app</key><true/></dict></plist>"#)).is_none());
    }
    #[test]
    fn malformed_or_overlapping_signature_ranges_fail_closed() {
        let good = signed("<plist><dict/></plist>");
        for length in 0..good.len() {
            assert!(signed_entitlements(&good[..length]).is_none());
        }
        let mut padded = good.clone();
        padded.extend_from_slice(&[0; 16]);
        assert_eq!(signed_entitlements(&padded), Some(0));
        let mut corrupted = good;
        corrupted[16..20].copy_from_slice(&4_u32.to_be_bytes());
        assert!(signed_entitlements(&corrupted).is_none());
        let mut header = [0_u8; 32];
        for (offset, value) in [(0, 0xfeed_facf_u32), (16, 1), (20, 16)] {
            header[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
        }
        let mut commands = Vec::new();
        for value in [0x1d_u32, 16, 64, 32] {
            commands.extend_from_slice(&value.to_le_bytes());
        }
        assert_eq!(signature_location(&header, &commands, 96), Some((64, 32)));
        assert!(signature_location(&header, &commands, 95).is_none());
        commands[8..12].copy_from_slice(&32_u32.to_le_bytes());
        assert!(signature_location(&header, &commands, 96).is_none());
    }
}
