use ferroui_view::prelude::Element;
use phone_controls::Icon;
use phone_domain::{AppId, Contact, Note, OfficeDocument, PhoneData, Route};
use phone_sdk::{session::Session, AppContext, AppManifest, AppRegistration, AppRegistry};
use phone_state::PhoneViewModel;
use std::rc::Rc;

fn empty(_: AppContext) -> Element {
    Element::Empty
}
fn builtin(_: Rc<PhoneViewModel>, _: Rc<Session>) -> Element {
    Element::Empty
}
fn app(id: &'static str, title: &'static str) -> AppRegistration {
    AppRegistration::new(AppManifest::new(id, title, Icon::Note), empty)
}
fn fixture(entries: Vec<AppRegistration>, data: PhoneData) -> (AppRegistry, Rc<PhoneViewModel>) {
    let registry = AppRegistry::new(entries).unwrap();
    let vm = PhoneViewModel::new(data, Rc::new(|_| {}));
    vm.set_registered_apps(
        registry
            .manifests()
            .into_iter()
            .filter(|manifest| manifest.builtin.is_none())
            .map(|manifest| manifest.id.to_string()),
    );
    (registry, vm)
}
fn ids(registry: &AppRegistry, vm: &PhoneViewModel, query: &str) -> Vec<&'static str> {
    registry
        .search_apps(vm, query)
        .into_iter()
        .map(|manifest| manifest.id)
        .collect()
}

#[test]
fn discovery_ranks_titles_deterministically_and_resolves_only_unambiguous_apps() {
    let (registry, vm) = fixture(
        vec![
            app("tests.archive", "My Paint archive"),
            app("tests.paint", "Paint"),
            app("tests.paint-three", "Paint 3D"),
            app("tests.alpha-notes", "Notes"),
            app("tests.beta-notes", "Notes"),
            app("tests.Tool", "Upper tool"),
            app("tests.tool", "Lower tool"),
        ],
        PhoneData::default(),
    );
    let before = vm.data().clone();
    assert_eq!(
        ids(&registry, &vm, "  pAiNt  "),
        ["tests.paint", "tests.paint-three", "tests.archive"]
    );
    assert_eq!(
        registry.resolve_app(&vm, "PAINT").unwrap().id,
        "tests.paint"
    );
    assert_eq!(
        registry.resolve_app(&vm, "archive").unwrap().id,
        "tests.archive"
    );
    assert_eq!(
        registry.resolve_app(&vm, "TESTS.PAINT-THREE").unwrap().id,
        "tests.paint-three"
    );
    assert!(registry.resolve_app(&vm, "pai").is_none());
    assert!(registry.resolve_app(&vm, "Notes").is_none());
    assert!(registry.resolve_app(&vm, "tests.Tool").is_none());
    assert!(registry.resolve_app(&vm, "tests.tool").is_none());
    assert_eq!(
        ids(&registry, &vm, "TESTS.TOOL"),
        ["tests.tool", "tests.Tool"]
    );
    assert_eq!(
        ids(&registry, &vm, "notes"),
        ["tests.alpha-notes", "tests.beta-notes"]
    );
    assert!(ids(&registry, &vm, "tests.").is_empty());
    assert!(registry.search_local(&vm, "  ").is_empty());
    assert!(registry.resolve_app(&vm, "").is_none());
    assert_eq!(*vm.data(), before);
}

#[test]
fn hidden_disabled_uninstalled_and_unregistered_apps_have_no_search_backdoor() {
    let mut hidden = app("tests.hidden", "Hidden game");
    hidden.manifest.listed = false;
    let mut absent = PhoneData::default();
    absent.installed.retain(|id| *id != AppId::Camera);
    let (registry, vm) = fixture(
        vec![
            hidden,
            app("tests.disabled", "Disabled game"),
            AppRegistration::builtin(AppId::Camera, builtin),
        ],
        absent,
    );
    vm.set_extension_enabled("tests.disabled", false).unwrap();
    let before = vm.data().clone();
    for query in [
        "Hidden game",
        "tests.hidden",
        "hidden",
        "Disabled game",
        "tests.disabled",
        "disabled",
        "Camera",
        "phone.camera",
        "cam",
        "tests.missing",
    ] {
        assert!(registry.search_apps(&vm, query).is_empty(), "{query}");
        assert!(registry.resolve_app(&vm, query).is_none(), "{query}");
        assert!(registry.search_local(&vm, query).is_empty(), "{query}");
    }
    assert_eq!(*vm.data(), before);
    vm.set_extension_enabled("tests.disabled", true).unwrap();
    assert_eq!(ids(&registry, &vm, "game"), ["tests.disabled"]);
    vm.set_registered_apps([]);
    assert!(registry.resolve_app(&vm, "tests.disabled").is_none());
}

#[test]
fn local_search_keeps_content_routes_and_uses_registered_app_metadata_without_duplicates() {
    let mut data = PhoneData::default();
    data.contacts.push(Contact {
        id: 101,
        name: "Project contact".into(),
        ..Contact::default()
    });
    data.notes.push(Note {
        id: 102,
        title: "Project note".into(),
        ..Note::default()
    });
    data.documents.push(OfficeDocument {
        id: 103,
        name: "Project document".into(),
        ..OfficeDocument::default()
    });
    let mut people = AppRegistration::builtin(AppId::People, builtin);
    people.manifest.title = "Project contacts";
    let (registry, vm) = fixture(
        vec![
            app("tests.project", "Project"),
            people,
            AppRegistration::builtin(AppId::OneNote, builtin),
            AppRegistration::builtin(AppId::Office, builtin),
        ],
        data,
    );
    let results = registry.search_local(&vm, "project");
    assert_eq!(
        results
            .iter()
            .map(|row| row.route.clone())
            .collect::<Vec<_>>(),
        [
            Route::Extension {
                app: "tests.project".into(),
                path: String::new()
            },
            Route::App(AppId::People),
            Route::Contact(101),
            Route::Note(102),
            Route::Document(103),
        ]
    );
    assert_eq!(results[2].app_id, "phone.people");
    assert_eq!(results[3].app_id, "phone.onenote");
    assert_eq!(results[4].app_id, "phone.office");
    assert_eq!(registry.search_local(&vm, "onenote").len(), 1);
    // The old built-in title does not reappear beside renamed registry metadata.
    assert!(registry.search_local(&vm, "people").is_empty());
}

#[test]
fn local_content_is_not_discovered_without_a_listed_available_owner() {
    let mut data = PhoneData::default();
    data.contacts.push(Contact {
        id: 101,
        name: "Project contact".into(),
        ..Contact::default()
    });
    data.notes.push(Note {
        id: 102,
        title: "Project note".into(),
        ..Note::default()
    });
    data.documents.push(OfficeDocument {
        id: 103,
        name: "Project document".into(),
        ..OfficeDocument::default()
    });
    data.installed.retain(|id| *id != AppId::OneNote);
    let mut people = AppRegistration::builtin(AppId::People, builtin);
    people.manifest.listed = false;
    let (registry, vm) = fixture(
        vec![people, AppRegistration::builtin(AppId::OneNote, builtin)],
        data,
    );
    assert!(registry.search_local(&vm, "project").is_empty());
}

#[test]
fn local_results_keep_the_existing_one_hundred_row_bound() {
    let mut data = PhoneData::default();
    data.notes = (1..=150)
        .map(|id| Note {
            id,
            title: format!("Project {id}"),
            ..Note::default()
        })
        .collect();
    let (registry, vm) = fixture(
        vec![
            app("tests.project", "Project"),
            AppRegistration::builtin(AppId::OneNote, builtin),
        ],
        data,
    );
    assert_eq!(registry.search_local(&vm, "project").len(), 100);
}
