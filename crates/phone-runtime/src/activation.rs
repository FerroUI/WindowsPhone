use phone_domain::{validate_app_key, validate_app_path};
#[derive(Debug, PartialEq, Eq)]
pub enum Activation {
    Home,
    App { id: String, path: String },
}
pub fn parse(uri: &str) -> Result<Activation, String> {
    if uri.len() > 8192 || uri.chars().any(char::is_control) {
        return Err("Invalid activation URI.".into());
    }
    let uri = url::Url::parse(uri).map_err(|_| "Invalid activation URI.")?;
    if uri.scheme() != "ferroui-phone"
        || !uri.username().is_empty()
        || uri.password().is_some()
        || uri.port().is_some()
        || uri.fragment().is_some()
    {
        return Err("Unsupported activation URI.".into());
    }
    match uri.host_str() {
        Some("home") if (uri.path().is_empty() || uri.path() == "/") && uri.query().is_none() => {
            Ok(Activation::Home)
        }
        Some("app") => {
            let id = uri
                .path()
                .strip_prefix('/')
                .ok_or("Missing app identity.")?;
            validate_app_key(id)?;
            let mut path = None;
            for (key, value) in uri.query_pairs() {
                if key != "path" || path.is_some() {
                    return Err("Unsupported activation parameter.".into());
                }
                path = Some(value.into_owned());
            }
            let path = path.unwrap_or_default();
            validate_app_path(&path)?;
            Ok(Activation::App {
                id: id.into(),
                path,
            })
        }
        _ => Err("Unsupported activation target.".into()),
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rejects_privileged_or_ambiguous_routes() {
        assert_eq!(parse("ferroui-phone://home").unwrap(), Activation::Home);
        assert!(parse("file:///etc/passwd").is_err());
        assert!(parse("ferroui-phone://user@home").is_err());
        assert!(parse("ferroui-phone://app/org.ferroui.wallet?path=a&path=b").is_err());
        assert!(parse("ferroui-phone://home?launch=anything").is_err());
    }
}
