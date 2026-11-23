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

pub fn get(url: &str, max_bytes: u32) -> Result<Vec<u8>, String> {
    let url = crate::http::validate_request(url, max_bytes)?;
    static AGENT: OnceLock<ureq::Agent> = OnceLock::new();
    let agent = AGENT.get_or_init(|| {
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
    });
    let mut response = agent.get(&url).header("Accept", "application/json, application/rss+xml, application/atom+xml, application/xml, text/xml, text/plain;q=0.8")
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
