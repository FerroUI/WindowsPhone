use super::AppRegistry;
use crate::AppManifest;
use phone_domain::Route;
use phone_state::PhoneViewModel;

/// A discoverable app or a local document owned by a discoverable app.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LocalSearchResult {
    pub app_id: &'static str,
    pub title: String,
    pub subtitle: String,
    pub route: Route,
}

impl AppRegistry {
    /// Listed, available applications matching a title substring or exact ID.
    /// Exact matches precede title prefixes and substrings. Equal ranks sort by
    /// title and stable ID, independently of registration order.
    pub fn search_apps(&self, vm: &PhoneViewModel, query: &str) -> Vec<AppManifest> {
        let query = query.trim().to_lowercase();
        if query.is_empty() {
            return Vec::new();
        }
        let mut matches = self
            .entries
            .iter()
            .map(|entry| entry.manifest)
            .filter(|manifest| discoverable(vm, manifest))
            .filter_map(|manifest| {
                let title = manifest.title.to_lowercase();
                let rank = if manifest.id.eq_ignore_ascii_case(&query) {
                    0
                } else if title == query {
                    1
                } else if title.starts_with(&query) {
                    2
                } else if title.contains(&query) {
                    3
                } else {
                    return None;
                };
                Some((rank, title, manifest))
            })
            .collect::<Vec<_>>();
        matches.sort_by(|a, b| (&a.0, &a.1, a.2.id).cmp(&(&b.0, &b.1, b.2.id)));
        matches
            .into_iter()
            .map(|(_, _, manifest)| manifest)
            .collect()
    }

    /// Resolve an explicit app-opening command. A unique case-insensitive ID
    /// match wins, followed by a unique exact title, then a unique substring.
    /// Ambiguity never opens an arbitrary first result. Hidden, uninstalled and
    /// disabled apps are absent.
    pub fn resolve_app(&self, vm: &PhoneViewModel, query: &str) -> Option<AppManifest> {
        let query = query.trim().to_lowercase();
        let matches = self.search_apps(vm, &query);
        let mut ids = matches
            .iter()
            .filter(|manifest| manifest.id.eq_ignore_ascii_case(&query));
        if let Some(manifest) = ids.next() {
            return ids.next().is_none().then_some(*manifest);
        }
        let mut exact = matches
            .iter()
            .filter(|manifest| manifest.title.to_lowercase() == query);
        if let Some(manifest) = exact.next() {
            return exact.next().is_none().then_some(*manifest);
        }
        (matches.len() == 1).then(|| matches[0])
    }

    /// Bounded local discovery shared by shell/search clients. App identities
    /// come from this registry; contact, note and Office routes retain their
    /// existing domain searches and require a listed, available route owner.
    pub fn search_local(&self, vm: &PhoneViewModel, query: &str) -> Vec<LocalSearchResult> {
        let apps = self
            .search_apps(vm, query)
            .into_iter()
            .map(|manifest| LocalSearchResult {
                app_id: manifest.id,
                title: manifest.title.into(),
                subtitle: "app".into(),
                route: manifest.route(""),
            });
        let content = vm.search(query).into_iter().filter_map(|result| {
            // Domain search still serves data-only clients. Its built-in roots
            // are superseded by the registry catalog, including renamed apps.
            if matches!(result.route, Route::App(_)) {
                return None;
            }
            let owner = self.for_route(&result.route)?.manifest;
            discoverable(vm, &owner).then_some(LocalSearchResult {
                app_id: owner.id,
                title: result.title,
                subtitle: result.subtitle,
                route: result.route,
            })
        });
        apps.chain(content).take(100).collect()
    }
}

fn discoverable(vm: &PhoneViewModel, manifest: &AppManifest) -> bool {
    manifest.listed
        && manifest.builtin.map_or_else(
            || vm.extension_enabled(manifest.id),
            |app| vm.data().installed.contains(&app),
        )
}
