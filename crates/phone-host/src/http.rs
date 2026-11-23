//! Shared admission rules for explicitly requested, credential-free HTTPS reads.
use std::net::IpAddr;

pub const MAX_RESPONSE_BYTES: u32 = 2 * 1024 * 1024;
pub const TIMEOUT_SECONDS: u64 = 15;

pub fn validate_url(value: &str) -> Result<String, String> {
    if value.len() > 4096 || value.chars().any(char::is_control) {
        return Err("The HTTPS address is too long or contains control characters.".into());
    }
    let mut url = url::Url::parse(value.trim()).map_err(|_| "Enter a complete HTTPS address.")?;
    if url.scheme() != "https" || !url.username().is_empty() || url.password().is_some() {
        return Err("Use a public HTTPS address without a username or password.".into());
    }
    if url.port().is_some_and(|port| port != 443) {
        return Err("HTTPS requests use the standard port 443.".into());
    }
    let host = url
        .host_str()
        .ok_or("The HTTPS address needs a host name.")?;
    let domain = host.trim_end_matches('.').to_ascii_lowercase();
    if !domain.contains('.')
        || [
            "localhost",
            "local",
            "internal",
            "lan",
            "home",
            "test",
            "invalid",
        ]
        .iter()
        .any(|suffix| domain == *suffix || domain.ends_with(&format!(".{suffix}")))
        || domain.parse::<IpAddr>().is_ok()
        || domain.starts_with('[')
    {
        return Err("Use a public internet host name, not a local address.".into());
    }
    url.set_fragment(None);
    Ok(url.into())
}

pub fn validate_request(value: &str, max_bytes: u32) -> Result<String, String> {
    if max_bytes == 0 || max_bytes > MAX_RESPONSE_BYTES {
        return Err("Choose a response limit between one byte and 2 MiB.".into());
    }
    validate_url(value)
}

/// Native resolution is checked before connecting, including mapped IPv4.
/// Browsers enforce their own network and private-network access policies.
pub fn is_public_address(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(ip) => {
            let [a, b, c, _] = ip.octets();
            !ip.is_private()
                && !ip.is_loopback()
                && !ip.is_link_local()
                && !ip.is_multicast()
                && !ip.is_broadcast()
                && !ip.is_documentation()
                && a != 0
                && a < 224
                && !(a == 100 && (64..=127).contains(&b))
                && !(a == 198 && (b == 18 || b == 19))
                && !(a == 192 && b == 0 && c == 0)
        }
        IpAddr::V6(ip) => {
            if let Some(ip) = ip.to_ipv4_mapped() {
                return is_public_address(IpAddr::V4(ip));
            }
            let segments = ip.segments();
            // Global unicast only; exclude documentation and transition ranges.
            segments[0] & 0xe000 == 0x2000
                && !(segments[0] == 0x2001 && (segments[1] < 0x0200 || segments[1] == 0x0db8))
                && segments[0] != 0x2002
                && !(segments[0] == 0x3fff && segments[1] < 0x1000)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rejects_local_credentials_unsafe_schemes_and_limits() {
        for value in [
            "http://example.org",
            "https://user:pass@example.org",
            "https://127.0.0.1",
            "https://[::1]",
            "https://localhost",
            "https://name.local",
            "https://a.example.org:8080",
            "data:text/plain,test",
            "https://example.org/\npath",
        ] {
            assert!(validate_request(value, 4096).is_err(), "{value}");
        }
        assert!(validate_request("https://example.org/feed", 0).is_err());
        assert!(validate_request("https://example.org/feed", MAX_RESPONSE_BYTES + 1).is_err());
        assert_eq!(
            validate_request("https://example.org/feed#section", 4096).unwrap(),
            "https://example.org/feed"
        );
    }
    #[test]
    fn rejects_private_reserved_and_transition_dns_results() {
        for ip in [
            "127.0.0.1",
            "10.1.2.3",
            "172.31.0.1",
            "192.168.0.1",
            "169.254.169.254",
            "100.64.0.1",
            "198.18.0.1",
            "192.0.2.1",
            "0.0.0.0",
            "255.255.255.255",
            "224.0.0.1",
            "::1",
            "::ffff:127.0.0.1",
            "fe80::1",
            "fc00::1",
            "2001:db8::1",
            "2002:7f00:1::1",
        ] {
            assert!(!is_public_address(ip.parse().unwrap()), "{ip}");
        }
        for ip in ["1.1.1.1", "8.8.8.8", "2606:4700:4700::1111"] {
            assert!(is_public_address(ip.parse().unwrap()), "{ip}");
        }
    }
}
