//! Identical URI validation for all native handoffs. Payloads are data, never
//! shell commands; recipient data cannot introduce query/header parameters.
use url::Url;

pub fn web_url(value: &str) -> Result<String, String> {
    let value = value.trim();
    if value.is_empty() || value.len() > 16_384 {
        return Err("Enter a valid web address.".into());
    }
    let candidate = if value.contains("://")
        || value.split(':').next().is_some_and(|s| {
            s.eq_ignore_ascii_case("javascript")
                || s.eq_ignore_ascii_case("file")
                || s.eq_ignore_ascii_case("data")
        }) {
        value.to_owned()
    } else {
        format!("https://{value}")
    };
    let url = Url::parse(&candidate).map_err(|_| "Enter a valid web address.")?;
    if !matches!(url.scheme(), "https" | "http")
        || url.host_str().is_none()
        || !url.username().is_empty()
        || url.password().is_some()
    {
        return Err("Use a normal HTTP or HTTPS address.".into());
    }
    Ok(url.into())
}

pub fn dial_url(number: &str) -> Result<String, String> {
    let value = number.trim();
    if value.is_empty()
        || value.len() > 60
        || !value
            .bytes()
            .all(|c| c.is_ascii_digit() || b"+ ()*#.-".contains(&c))
    {
        return Err("Enter a valid phone number.".into());
    }
    Ok(format!("tel:{}", escape(&value.replace(' ', ""))))
}

pub fn sms_url(number: &str, body: &str) -> Result<String, String> {
    let tel = dial_url(number)?;
    if body.len() > 1_048_576 {
        return Err("The message is too large.".into());
    }
    Ok(format!("sms:{}?body={}", &tel[4..], escape(body)))
}

pub fn mail_url(to: &str, subject: &str, body: &str) -> Result<String, String> {
    if to.len() > 512
        || to.chars().any(char::is_control)
        || subject.len() > 4096
        || body.len() > 1_048_576
    {
        return Err("Enter a valid email recipient and message.".into());
    }
    Ok(format!(
        "mailto:{}?subject={}&body={}",
        escape(to),
        escape(subject),
        escape(body)
    ))
}

fn escape(value: &str) -> String {
    let mut escaped = String::with_capacity(value.len());
    use std::fmt::Write;
    for byte in value.bytes() {
        if byte.is_ascii_alphanumeric() || b"-._~".contains(&byte) {
            escaped.push(char::from(byte));
        } else {
            let _ = write!(escaped, "%{byte:02X}");
        }
    }
    escaped
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn handoffs_reject_recipient_injection_and_encode_payloads() {
        assert!(dial_url("123?body=other").is_err());
        assert!(mail_url("a@example.com\r\nBcc:other", "", "").is_err());
        assert_eq!(
            sms_url(" +48 123 ", "a&b\n✓").unwrap(),
            "sms:%2B48123?body=a%26b%0A%E2%9C%93"
        );
        assert_eq!(
            web_url("example.com/a b").unwrap(),
            "https://example.com/a%20b"
        );
        for value in [
            "javascript:alert(1)",
            "file:///tmp/example",
            "https://user:pass@example.com",
            "data:text/plain,x",
        ] {
            assert!(web_url(value).is_err(), "{value}");
        }
    }
}
