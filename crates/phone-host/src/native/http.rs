//! Network reads run on the host worker, never the FerroUI dispatcher.
use std::{sync::OnceLock, time::Duration};
use ureq::unversioned::{
    resolver::{DefaultResolver, ResolvedSocketAddrs, Resolver},
    transport::{DefaultConnector, NextTimeout},
};

#[derive(Debug, Default)]
struct PublicResolver(DefaultResolver);
impl Resolver for PublicResolver {
    fn resolve(
        &self,
        uri: &ureq::http::Uri,
        config: &ureq::config::Config,
        timeout: NextTimeout,
    ) -> Result<ResolvedSocketAddrs, ureq::Error> {
        let addresses = self.0.resolve(uri, config, timeout)?;
        if addresses
            .iter()
            .any(|address| !crate::http::is_public_address(address.ip()))
        {
            return Err(ureq::Error::Io(std::io::Error::new(
                std::io::ErrorKind::PermissionDenied,
                "The address resolves to a non-public network.",
            )));
        }
        Ok(addresses)
    }
}

fn agent() -> &'static ureq::Agent {
    static AGENT: OnceLock<ureq::Agent> = OnceLock::new();
    AGENT.get_or_init(|| {
        let config = ureq::Agent::config_builder()
            .https_only(true)
            .max_redirects(0)
            .proxy(None)
            .timeout_global(Some(Duration::from_secs(crate::http::TIMEOUT_SECONDS)))
            .build();
        ureq::Agent::with_parts(
            config,
            DefaultConnector::default(),
            PublicResolver::default(),
        )
    })
}
pub fn get(url: &str, max_bytes: u32) -> Result<Vec<u8>, String> {
    let url = crate::http::validate_request(url, max_bytes)?;
    let mut response = agent().get(&url).header("User-Agent", "FerroUI-WindowsPhone/0.1 (+https://github.com/FerroUI/WindowsPhone)").header("Accept", "application/json, application/rss+xml, application/atom+xml, application/xml, text/xml, text/plain;q=0.8")
        .call().map_err(|error| format!("HTTPS request failed: {error}"))?;
    if !response.status().is_success() {
        return Err(format!(
            "The server returned {}. Use the final HTTPS address if this feed has moved.",
            response.status()
        ));
    }
    response
        .body_mut()
        .with_config()
        .limit(max_bytes as u64)
        .read_to_vec()
        .map_err(|error| format!("Could not read the bounded HTTPS response: {error}"))
}

#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct ImageCacheHeaders {
    pub max_age: Option<u64>,
    pub expires: Option<String>,
    pub etag: Option<String>,
    pub last_modified: Option<String>,
    pub no_store: bool,
}
pub struct ImageResponse {
    pub bytes: Option<Vec<u8>>,
    pub headers: ImageCacheHeaders,
}
/// Conditional binary read for the visible-tile cache; shares HTTPS/DNS/timeout
/// admission with ordinary SDK reads and never carries account credentials.
pub fn image(url: &str, previous: Option<&ImageCacheHeaders>) -> Result<ImageResponse, String> {
    let url = crate::http::validate_request(url, crate::maps::MAX_TILE_BYTES)?;
    let mut request = agent()
        .get(&url)
        .header(
            "User-Agent",
            "FerroUI-WindowsPhone/0.1 (+https://github.com/FerroUI/WindowsPhone)",
        )
        .header("Accept", "image/png");
    if let Some(previous) = previous {
        if let Some(value) = previous.etag.as_deref().filter(|v| valid_header(v)) {
            request = request.header("If-None-Match", value);
        }
        if let Some(value) = previous
            .last_modified
            .as_deref()
            .filter(|v| valid_header(v))
        {
            request = request.header("If-Modified-Since", value);
        }
    }
    let mut response = request
        .call()
        .map_err(|error| format!("Map tile request failed: {error}"))?;
    let header = |name: &str| {
        response
            .headers()
            .get(name)
            .and_then(|v| v.to_str().ok())
            .filter(|v| valid_header(v))
            .map(str::to_owned)
    };
    let control = header("cache-control")
        .unwrap_or_default()
        .to_ascii_lowercase();
    let age = header("age")
        .and_then(|v| v.parse::<u64>().ok())
        .unwrap_or(0);
    let max_age = control
        .split(',')
        .find_map(|part| {
            part.trim()
                .strip_prefix("max-age=")
                .and_then(|v| v.trim_matches('"').parse::<u64>().ok())
        })
        .map(|v| v.saturating_sub(age).min(366 * 86400));
    let headers = ImageCacheHeaders {
        max_age: if control.split(',').any(|v| v.trim() == "no-cache") {
            Some(0)
        } else {
            max_age
        },
        expires: header("expires"),
        etag: header("etag"),
        last_modified: header("last-modified"),
        no_store: control.split(',').any(|v| v.trim() == "no-store"),
    };
    let bytes = if response.status().as_u16() == 304 {
        None
    } else {
        if !response.status().is_success() {
            return Err(format!("Map service returned HTTP {}.", response.status()));
        }
        let bytes = response
            .body_mut()
            .with_config()
            .limit(crate::maps::MAX_TILE_BYTES as u64)
            .read_to_vec()
            .map_err(|e| format!("Map tile could not be read: {e}"))?;
        crate::maps::validate_png(&bytes)?;
        Some(bytes)
    };
    Ok(ImageResponse { bytes, headers })
}
fn valid_header(value: &str) -> bool {
    value.len() <= 1024 && !value.chars().any(char::is_control)
}
