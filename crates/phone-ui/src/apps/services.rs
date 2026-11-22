mod browser;
mod search;
mod tools;

use crate::session::Session;
use ferroui_view::prelude::*;
use phone_domain::AppId;
use phone_state::PhoneViewModel;
use std::rc::Rc;

pub fn view(vm: Rc<PhoneViewModel>, ui: Rc<Session>, app: AppId) -> Element {
    match app {
        AppId::InternetExplorer => browser::view(vm, ui),
        AppId::Cortana => search::view(vm, ui),
        AppId::Maps
        | AppId::Weather
        | AppId::Files
        | AppId::Office
        | AppId::BatterySaver
        | AppId::DataSense => tools::view(vm, ui, app),
        _ => unreachable!("service route"),
    }
}

pub(crate) fn encode(value: &str) -> String {
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

fn web_url(value: &str) -> Result<String, String> {
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
