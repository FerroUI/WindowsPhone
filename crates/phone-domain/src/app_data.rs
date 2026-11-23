use std::collections::BTreeMap;

pub const MAX_APP_NAMESPACES: usize = 256;
pub const MAX_APP_STATE_BYTES: usize = 256 * 1024;

pub fn validate_app_key(value: &str) -> Result<(), String> {
    if value.is_empty()
        || value.len() > 128
        || !value.as_bytes()[0].is_ascii_alphanumeric()
        || value
            .bytes()
            .any(|b| !(b.is_ascii_alphanumeric() || matches!(b, b'.' | b'-' | b'_')))
    {
        return Err("Application identifiers must use 1–128 ASCII letters, digits, dots, dashes or underscores.".into());
    }
    Ok(())
}

/// App JSON must survive the browser's JSON and structured-clone transport
/// without losing integer precision. Encode larger identifiers as strings.
pub fn validate_app_value(value: &serde_json::Value) -> Result<(), String> {
    const MAX_SAFE: u64 = 9_007_199_254_740_991;
    let mut pending = vec![(value, 0usize)];
    let mut nodes = 0usize;
    while let Some((value, depth)) = pending.pop() {
        nodes += 1;
        if nodes > 16_384 {
            return Err("App data contains too many JSON values (limit 16384).".into());
        }
        if depth > 32 {
            return Err("App data nesting exceeds 32 levels.".into());
        }
        match value {
            serde_json::Value::Number(number) => {
                let unsafe_integer = number.as_u64().is_some_and(|value| value > MAX_SAFE)
                    || number
                        .as_i64()
                        .is_some_and(|value| value < -(MAX_SAFE as i64));
                if unsafe_integer {
                    return Err("App data integers must fit JavaScript's exact integer range; encode larger values as strings.".into());
                }
            }
            serde_json::Value::Array(values) => {
                if nodes + pending.len() + values.len() > 16_384 {
                    return Err("App data contains too many JSON values (limit 16384).".into());
                }
                pending.extend(values.iter().map(|value| (value, depth + 1)));
            }
            serde_json::Value::Object(values) => {
                if nodes + pending.len() + values.len() > 16_384 {
                    return Err("App data contains too many JSON values (limit 16384).".into());
                }
                pending.extend(values.values().map(|value| (value, depth + 1)));
            }
            _ => {}
        }
    }
    if serde_json::to_vec(value)
        .map_err(|error| error.to_string())?
        .len()
        > MAX_APP_STATE_BYTES
    {
        return Err("Application data exceeds 256 KiB.".into());
    }
    Ok(())
}

pub(crate) fn validate_app_data(
    values: &BTreeMap<String, serde_json::Value>,
) -> Result<(), String> {
    if values.len() > MAX_APP_NAMESPACES {
        return Err("Too many app data namespaces.".into());
    }
    for (id, value) in values {
        validate_app_key(id)?;
        validate_app_value(value).map_err(|error| format!("Invalid data for {id}: {error}"))?;
    }
    Ok(())
}

/// Persisted secondary launch target; the registry remains authoritative for code.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct AppLink {
    pub id: crate::Id,
    pub app: String,
    pub path: String,
    pub title: String,
}
pub fn validate_app_path(path: &str) -> Result<(), String> {
    if path.len() > 2048 || path.chars().any(char::is_control) {
        return Err(
            "Application paths must be at most 2048 bytes without control characters.".into(),
        );
    }
    Ok(())
}
pub fn validate_app_link(app: &str, path: &str, title: &str) -> Result<(), String> {
    validate_app_key(app)?;
    validate_app_path(path)?;
    if title.trim().is_empty() || title.chars().count() > 80 || title.chars().any(char::is_control)
    {
        return Err("Give this app tile a title of 1–80 printable characters.".into());
    }
    Ok(())
}
pub(crate) fn validate_app_links(
    links: &[AppLink],
    disabled: &std::collections::BTreeSet<String>,
) -> Result<(), String> {
    if links.len() > 512 || disabled.len() > MAX_APP_NAMESPACES {
        return Err("Too many application links or disabled apps.".into());
    }
    let mut routes = std::collections::BTreeSet::new();
    for link in links {
        validate_app_link(&link.app, &link.path, &link.title)?;
        if disabled.contains(&link.app) {
            return Err("A disabled app cannot retain a Start tile link.".into());
        }
        if !routes.insert((&link.app, &link.path)) {
            return Err("Duplicate application launch target.".into());
        }
    }
    for id in disabled {
        validate_app_key(id)?;
    }
    Ok(())
}
