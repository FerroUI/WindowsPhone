pub fn encode(value: &str) -> String {
    let mut result = String::new();
    for byte in value.bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b'~') {
            result.push(byte as char);
        } else {
            use std::fmt::Write;
            let _ = write!(&mut result, "%{byte:02X}");
        }
    }
    result
}

pub fn web_url(value: &str) -> Result<String, String> {
    let value = value.trim();
    if value.is_empty() {
        return Err("Enter a web address or search term.".into());
    }
    if value.starts_with("https://") || value.starts_with("http://") {
        return Ok(value.into());
    }
    if value.contains("://")
        || value.to_lowercase().starts_with("javascript:")
        || value.to_lowercase().starts_with("data:")
    {
        return Err("Use an http or https web address.".into());
    }
    if !value.contains(char::is_whitespace) && value.contains('.') {
        Ok(format!("https://{value}"))
    } else {
        Ok(format!("https://www.bing.com/search?q={}", encode(value)))
    }
}
