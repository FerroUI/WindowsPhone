use phone_domain::{AppId, Route};
use phone_sdk::{common::clock, session::Session, AppManifest, AppRegistry};
use phone_state::{HostRequest, PhoneViewModel};

pub(super) fn run(vm: &PhoneViewModel, ui: &Session) {
    let value = ui.query.get_untracked();
    let value = value.trim();
    let lower = value.to_lowercase();
    let registry = ui.registry();
    if let Some(name) = lower.strip_prefix("open ") {
        if let Some(app) = resolve_app(&registry, vm, name) {
            vm.navigate(app.route(""));
        } else {
            report_matches(vm, registry.search_local(vm, name).len());
        }
        return;
    }
    if let Some(name) = lower.strip_prefix("call ") {
        let contacts = vm
            .data()
            .contacts
            .iter()
            .filter(|person| person.name.to_lowercase().contains(name))
            .cloned()
            .collect::<Vec<_>>();
        if contacts.len() == 1 {
            let person = &contacts[0];
            vm.record_call(person.name.clone(), person.phone.clone());
            return;
        } else if contacts.len() > 1 {
            ui.query.set(name.into());
            vm.set_status("Choose the person you want to call.".into());
            return;
        }
    }
    if let Some(value) = lower.strip_prefix("search ") {
        vm.request(HostRequest::OpenUrl {
            url: format!(
                "https://www.bing.com/search?q={}",
                phone_app_support::urls::encode(value)
            ),
        });
        return;
    }
    if lower == "what time is it" || lower == "what time is it?" || lower == "time" {
        vm.set_status(format!("It's {}.", clock(vm)));
        return;
    }
    let results = registry.search_local(vm, find_query(value));
    if results.len() == 1 {
        vm.navigate(results[0].route.clone());
    } else if !value.is_empty() {
        report_matches(vm, results.len());
    }
}

pub(super) fn find_query(value: &str) -> &str {
    let value = value.trim();
    if value.get(..5).is_some_and(|prefix| {
        prefix.eq_ignore_ascii_case("find ") || prefix.eq_ignore_ascii_case("open ")
    }) {
        value[5..].trim()
    } else {
        value
    }
}

fn resolve_app(registry: &AppRegistry, vm: &PhoneViewModel, name: &str) -> Option<AppManifest> {
    let name = name.trim();
    if let Some(app) = registry.resolve_app(vm, name) {
        return Some(app);
    }
    // Legacy spoken aliases apply only when the actual registered titles have
    // no matches. They cannot bypass ambiguity or an owner's availability.
    if !registry.search_apps(vm, name).is_empty() {
        return None;
    }
    let builtin = match name {
        "contacts" => AppId::People,
        "browser" => AppId::InternetExplorer,
        "messages" => AppId::Messaging,
        "mail" => AppId::Outlook,
        "notes" => AppId::OneNote,
        _ => return None,
    };
    let owner = registry.for_route(&Route::App(builtin))?;
    registry
        .search_apps(vm, owner.manifest.id)
        .into_iter()
        .find(|manifest| manifest.builtin == Some(builtin))
}

fn report_matches(vm: &PhoneViewModel, count: usize) {
    vm.set_status(format!(
        "{count} matches on your phone. Choose a result, or search the web."
    ));
}

#[cfg(test)]
mod tests;
