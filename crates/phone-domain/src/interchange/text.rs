use super::MAX_INTERCHANGE_BYTES;
use std::collections::BTreeMap;

pub(super) struct Property {
    pub name: String,
    pub params: BTreeMap<String, String>,
    pub value: String,
}

/// Unfold bytes first, so even an external producer's mid-code-point fold can
/// be recovered without accepting invalid UTF-8 or unbounded logical lines.
pub(super) fn lines(bytes: &[u8]) -> Result<Vec<String>, String> {
    if bytes.is_empty() || bytes.len() > MAX_INTERCHANGE_BYTES {
        return Err("Choose a nonempty contact or calendar file no larger than 2 MB.".into());
    }
    let bytes = bytes.strip_prefix(&[0xef, 0xbb, 0xbf]).unwrap_or(bytes);
    let mut result: Vec<Vec<u8>> = Vec::new();
    for physical in bytes.split(|byte| *byte == b'\n') {
        let physical = physical.strip_suffix(b"\r").unwrap_or(physical);
        if physical.contains(&b'\r') || physical.contains(&0) {
            return Err("The file contains an invalid line ending or control character.".into());
        }
        if physical
            .first()
            .is_some_and(|byte| matches!(byte, b' ' | b'\t'))
        {
            let previous = result
                .last_mut()
                .ok_or("A folded line has no preceding property.")?;
            previous.extend_from_slice(&physical[1..]);
            if previous.len() > 262_144 {
                return Err("A contact or calendar property is too long.".into());
            }
        } else if !physical.is_empty() {
            if physical.len() > 262_144 || result.len() >= 32_768 {
                return Err("This contact or calendar file contains too much data.".into());
            }
            result.push(physical.to_vec());
        }
    }
    result
        .into_iter()
        .map(|bytes| {
            let value =
                String::from_utf8(bytes).map_err(|_| "Use a UTF-8 contact or calendar file.")?;
            if value.chars().any(|ch| ch.is_control() && ch != '\t') {
                return Err("The file contains an unsupported control character.".into());
            }
            Ok(value)
        })
        .collect()
}

fn split_quoted(value: &str, delimiter: char) -> Result<Vec<&str>, String> {
    let mut quoted = false;
    let mut start = 0;
    let mut values = Vec::new();
    for (index, ch) in value.char_indices() {
        if ch == '"' {
            quoted = !quoted;
        } else if ch == delimiter && !quoted {
            values.push(&value[start..index]);
            start = index + ch.len_utf8();
        }
    }
    if quoted {
        return Err("A property parameter has an unterminated quote.".into());
    }
    values.push(&value[start..]);
    Ok(values)
}

pub(super) fn property(line: &str) -> Result<Property, String> {
    let mut quoted = false;
    let colon = line
        .char_indices()
        .find_map(|(index, ch)| {
            if ch == '"' {
                quoted = !quoted;
            }
            (ch == ':' && !quoted).then_some(index)
        })
        .ok_or("A contact or calendar property is missing its value separator.")?;
    let parts = split_quoted(&line[..colon], ';')?;
    let name = parts[0]
        .rsplit('.')
        .next()
        .unwrap_or("")
        .to_ascii_uppercase();
    if name.is_empty() || !name.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-') {
        return Err("The file contains an invalid property name.".into());
    }
    let mut params = BTreeMap::new();
    for part in &parts[1..] {
        let (key, value) = part
            .split_once('=')
            .ok_or("Unsupported property parameter syntax.")?;
        let key = key.to_ascii_uppercase();
        let value = value
            .strip_prefix('"')
            .and_then(|v| v.strip_suffix('"'))
            .unwrap_or(value);
        if key.is_empty() || params.insert(key, value.to_owned()).is_some() {
            return Err("The file contains a duplicate property parameter.".into());
        }
    }
    Ok(Property {
        name,
        params,
        value: line[colon + 1..].into(),
    })
}

pub(super) fn allowed_params(value: &Property, names: &[&str]) -> Result<(), String> {
    if let Some(name) = value
        .params
        .keys()
        .find(|name| !names.contains(&name.as_str()))
    {
        return Err(format!(
            "{} parameter {name} cannot be preserved by this phone.",
            value.name
        ));
    }
    Ok(())
}

pub(super) fn unescape(value: &str) -> Result<String, String> {
    let mut output = String::new();
    let mut chars = value.chars();
    while let Some(ch) = chars.next() {
        if ch != '\\' {
            output.push(ch);
            continue;
        }
        output.push(match chars.next() {
            Some('n' | 'N') => '\n',
            Some(ch @ ('\\' | ',' | ';')) => ch,
            _ => return Err("The file contains an unsupported text escape.".into()),
        });
    }
    Ok(output)
}

pub(super) fn escape(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace("\r\n", "\n")
        .replace('\r', "\n")
        .replace('\n', "\\n")
        .replace(';', "\\;")
        .replace(',', "\\,")
}

pub(super) fn fields(value: &str, delimiter: char) -> Result<Vec<String>, String> {
    let mut escaped = false;
    let mut start = 0;
    let mut output = Vec::new();
    for (index, ch) in value.char_indices() {
        if ch == delimiter && !escaped {
            output.push(unescape(&value[start..index])?);
            start = index + ch.len_utf8();
        }
        escaped = ch == '\\' && !escaped;
    }
    output.push(unescape(&value[start..])?);
    Ok(output)
}

/// RFC 6350 / RFC 5545: 75-octet physical lines, UTF-8 code points intact.
pub(super) fn push_line(output: &mut String, value: &str) {
    let mut remaining = value;
    let mut width = 75;
    while remaining.len() > width {
        let mut end = width;
        while !remaining.is_char_boundary(end) {
            end -= 1;
        }
        output.push_str(&remaining[..end]);
        output.push_str("\r\n ");
        remaining = &remaining[end..];
        width = 74;
    }
    output.push_str(remaining);
    output.push_str("\r\n");
}

pub(super) fn once(slot: &mut Option<String>, value: String, name: &str) -> Result<(), String> {
    if slot.replace(value).is_some() {
        Err(format!(
            "Multiple {name} values cannot be preserved by this phone."
        ))
    } else {
        Ok(())
    }
}
