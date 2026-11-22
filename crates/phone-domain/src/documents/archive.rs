//! Minimal OPC ZIP transport: bounded central-directory parsing, CRC checks, no filesystem writes.
use flate2::read::DeflateDecoder;
use std::{collections::BTreeMap, io::Read};

const MAX_INPUT: usize = 8 * 1024 * 1024;
const MAX_TOTAL: usize = 16 * 1024 * 1024;
const MAX_ENTRY: usize = 4 * 1024 * 1024;
pub(super) type Package = BTreeMap<String, Vec<u8>>;

pub(super) fn read(bytes: &[u8]) -> Result<Package, String> {
    if bytes.len() > MAX_INPUT {
        return Err("This Office file exceeds the 8 MB import limit.".into());
    }
    let start = bytes.len().saturating_sub(65_557);
    let end = (start..bytes.len().saturating_sub(21))
        .rev()
        .find(|p| {
            bytes.get(*p..*p + 4) == Some(b"PK\x05\x06")
                && u16_at(bytes, *p + 20).is_ok_and(|n| *p + 22 + n as usize == bytes.len())
        })
        .ok_or("This file is not a supported Office ZIP package.")?;
    if u16_at(bytes, end + 4)? != 0 || u16_at(bytes, end + 6)? != 0 {
        return Err("Split ZIP packages are not supported.".into());
    }
    let entries = u16_at(bytes, end + 10)? as usize;
    if entries > 1024 || entries != u16_at(bytes, end + 8)? as usize {
        return Err("This Office package has too many parts.".into());
    }
    let central_size = u32_at(bytes, end + 12)? as usize;
    let mut position = u32_at(bytes, end + 16)? as usize;
    if position.checked_add(central_size) != Some(end) {
        return Err("This ZIP directory is invalid or uses ZIP64.".into());
    }
    let mut total = 0usize;
    let mut result = Package::new();
    for _ in 0..entries {
        if bytes.get(position..position + 4) != Some(b"PK\x01\x02") {
            return Err("This ZIP directory is damaged.".into());
        }
        let flags = u16_at(bytes, position + 8)?;
        let method = u16_at(bytes, position + 10)?;
        if flags & 1 != 0 || ![0, 8].contains(&method) {
            return Err(
                "Encrypted or unsupported compressed Office files cannot be opened.".into(),
            );
        }
        let crc = u32_at(bytes, position + 16)?;
        let compressed = u32_at(bytes, position + 20)? as usize;
        let expanded = u32_at(bytes, position + 24)? as usize;
        let name_len = u16_at(bytes, position + 28)? as usize;
        let extra_len = u16_at(bytes, position + 30)? as usize;
        let comment_len = u16_at(bytes, position + 32)? as usize;
        if u16_at(bytes, position + 34)? != 0 {
            return Err("Split ZIP packages are not supported.".into());
        }
        let local = u32_at(bytes, position + 42)? as usize;
        let name = std::str::from_utf8(slice(bytes, position + 46, name_len)?)
            .map_err(|_| "Office part names must be UTF-8.")?
            .to_owned();
        if name.is_empty()
            || name.starts_with('/')
            || name.contains('\\')
            || name.split('/').any(|p| p == ".." || p == ".")
            || name.contains(':')
            || name.bytes().any(|b| b < 32)
            || result.contains_key(&name)
        {
            return Err("This Office package contains an unsafe or duplicate part name.".into());
        }
        total = total
            .checked_add(expanded)
            .ok_or("This Office package is too large.")?;
        if expanded > MAX_ENTRY
            || total > MAX_TOTAL
            || (expanded > 65536 && expanded > compressed.saturating_mul(200))
        {
            return Err("This Office package exceeds safe expansion limits.".into());
        }
        if bytes.get(local..local + 4) != Some(b"PK\x03\x04")
            || u16_at(bytes, local + 6)? != flags
            || u16_at(bytes, local + 8)? != method
        {
            return Err("This ZIP entry does not match its directory.".into());
        }
        let local_name = u16_at(bytes, local + 26)? as usize;
        let local_extra = u16_at(bytes, local + 28)? as usize;
        if slice(bytes, local + 30, local_name)? != name.as_bytes() {
            return Err("This ZIP entry has an inconsistent name.".into());
        }
        let data_start = local
            .checked_add(30 + local_name + local_extra)
            .ok_or("Invalid ZIP offset.")?;
        if data_start
            .checked_add(compressed)
            .is_none_or(|v| v > u32_at(bytes, end + 16).unwrap_or(0) as usize)
        {
            return Err("This ZIP entry overlaps its directory.".into());
        }
        let encoded = slice(bytes, data_start, compressed)?;
        let content = if method == 0 {
            encoded.to_vec()
        } else {
            let decoder = DeflateDecoder::new(encoded);
            let mut decoded = Vec::with_capacity(expanded.min(65536));
            decoder
                .take(expanded as u64 + 1)
                .read_to_end(&mut decoded)
                .map_err(|_| "This Office part cannot be decompressed.")?;
            decoded
        };
        if content.len() != expanded || crc32(&content) != crc {
            return Err("This Office file failed its content integrity check.".into());
        }
        result.insert(name, content);
        position = position
            .checked_add(46 + name_len + extra_len + comment_len)
            .ok_or("Invalid ZIP directory size.")?;
        if position > end {
            return Err("This ZIP directory is truncated.".into());
        }
    }
    if position != end {
        return Err("This ZIP directory contains unsupported data.".into());
    }
    Ok(result)
}

pub(super) fn write(parts: Vec<(String, String)>) -> Vec<u8> {
    let mut output = Vec::new();
    let mut directory = Vec::new();
    let count = parts.len() as u16;
    for (name, content) in parts {
        let name = name.as_bytes();
        let content = content.as_bytes();
        let offset = output.len() as u32;
        let crc = crc32(content);
        let size = content.len() as u32;
        output.extend(b"PK\x03\x04");
        u16_out(&mut output, 20);
        u16_out(&mut output, 0x800);
        u16_out(&mut output, 0);
        u16_out(&mut output, 0);
        u16_out(&mut output, 33);
        u32_out(&mut output, crc);
        u32_out(&mut output, size);
        u32_out(&mut output, size);
        u16_out(&mut output, name.len() as u16);
        u16_out(&mut output, 0);
        output.extend(name);
        output.extend(content);
        directory.extend(b"PK\x01\x02");
        u16_out(&mut directory, 20);
        u16_out(&mut directory, 20);
        u16_out(&mut directory, 0x800);
        u16_out(&mut directory, 0);
        u16_out(&mut directory, 0);
        u16_out(&mut directory, 33);
        u32_out(&mut directory, crc);
        u32_out(&mut directory, size);
        u32_out(&mut directory, size);
        u16_out(&mut directory, name.len() as u16);
        for _ in 0..4 {
            u16_out(&mut directory, 0);
        }
        u32_out(&mut directory, 0);
        u32_out(&mut directory, offset);
        directory.extend(name);
    }
    let offset = output.len() as u32;
    let length = directory.len() as u32;
    output.extend(directory);
    output.extend(b"PK\x05\x06");
    u16_out(&mut output, 0);
    u16_out(&mut output, 0);
    u16_out(&mut output, count);
    u16_out(&mut output, count);
    u32_out(&mut output, length);
    u32_out(&mut output, offset);
    u16_out(&mut output, 0);
    output
}

pub(super) fn xml<'a>(package: &'a Package, path: &str) -> Result<roxmltree::Document<'a>, String> {
    let content = package
        .get(path)
        .ok_or_else(|| format!("Office package part {path} is missing."))?;
    let source = std::str::from_utf8(content)
        .map_err(|_| "This Office package uses an unsupported XML encoding.")?;
    if source.contains("<!DOCTYPE") || source.contains("<!ENTITY") {
        return Err("XML entities are not supported in Office imports.".into());
    }
    roxmltree::Document::parse(source).map_err(|_| format!("Office XML part {path} is damaged."))
}
pub(super) fn escape(text: &str) -> String {
    text.chars()
        .filter(|c| *c == '\t' || *c == '\n' || *c == '\r' || *c >= ' ')
        .flat_map(|c| match c {
            '&' => "&amp;".chars().collect::<Vec<_>>(),
            '<' => "&lt;".chars().collect(),
            '>' => "&gt;".chars().collect(),
            '"' => "&quot;".chars().collect(),
            '\'' => "&apos;".chars().collect(),
            _ => vec![c],
        })
        .collect()
}
fn slice(bytes: &[u8], start: usize, length: usize) -> Result<&[u8], String> {
    bytes
        .get(start..start.checked_add(length).ok_or("Invalid ZIP length.")?)
        .ok_or_else(|| "This ZIP file is truncated.".into())
}
fn u16_at(bytes: &[u8], p: usize) -> Result<u16, String> {
    let b = slice(bytes, p, 2)?;
    Ok(u16::from_le_bytes([b[0], b[1]]))
}
fn u32_at(bytes: &[u8], p: usize) -> Result<u32, String> {
    let b = slice(bytes, p, 4)?;
    Ok(u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
}
fn u16_out(bytes: &mut Vec<u8>, n: u16) {
    bytes.extend(n.to_le_bytes());
}
fn u32_out(bytes: &mut Vec<u8>, n: u32) {
    bytes.extend(n.to_le_bytes());
}
fn crc32(bytes: &[u8]) -> u32 {
    let mut crc = !0u32;
    for &b in bytes {
        crc ^= b as u32;
        for _ in 0..8 {
            crc = (crc >> 1) ^ ((0u32.wrapping_sub(crc & 1)) & 0xedb88320);
        }
    }
    !crc
}
