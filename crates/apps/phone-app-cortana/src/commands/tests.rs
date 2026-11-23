use super::{find_query, run};
use ferroui_view::prelude::Element;
use phone_controls::Icon;
use phone_domain::{AppId, Contact, Note, OfficeDocument, PhoneData, Route};
use phone_sdk::{session::Session, AppContext, AppManifest, AppRegistration, AppRegistry};
use phone_state::{HostRequest, HostServices, PhoneViewModel};
use std::{cell::RefCell, rc::Rc};

fn empty(_: AppContext) -> Element {
    Element::Empty
}
fn builtin(_: Rc<PhoneViewModel>, _: Rc<Session>) -> Element {
    Element::Empty
}
fn app(id: &'static str, title: &'static str) -> AppRegistration {
    AppRegistration::new(AppManifest::new(id, title, Icon::Note), empty)
}
fn fixture(
    entries: Vec<AppRegistration>,
    data: PhoneData,
) -> (Rc<PhoneViewModel>, Session, Rc<RefCell<Vec<HostRequest>>>) {
    let registry = Rc::new(AppRegistry::new(entries).unwrap());
    let vm = PhoneViewModel::new(data, Rc::new(|_| {}));
    vm.set_registered_apps(
        registry
            .manifests()
            .into_iter()
            .filter(|app| app.builtin.is_none())
            .map(|app| app.id.to_string()),
    );
    let requests = Rc::new(RefCell::new(Vec::new()));
    let recorded = requests.clone();
    vm.set_services(HostServices {
        request: Rc::new(move |request| recorded.borrow_mut().push(request)),
    });
    let ui = Session::new(vm.now(), registry);
    vm.navigate(Route::Search);
    (vm, ui, requests)
}
fn submit(vm: &PhoneViewModel, ui: &Session, value: &str) {
    vm.navigate(Route::Search);
    ui.query.set(value.into());
    run(vm, ui);
}

#[test]
fn find_and_open_registered_apps_use_their_manifest_and_never_implicit_web_search() {
    let (vm, ui, requests) = fixture(
        vec![app("tests.tic-tac-toe", "Tic tac toe")],
        PhoneData::default(),
    );
    let before = vm.data().clone();
    for value in [
        "find Tic tac toe",
        "  OpEn TIC TAC TOE  ",
        "open tests.tic-tac-toe",
        "open tac",
    ] {
        submit(&vm, &ui, value);
        assert_eq!(
            vm.route(),
            Route::Extension {
                app: "tests.tic-tac-toe".into(),
                path: String::new()
            }
        );
    }
    assert_eq!(find_query("  FIND   Tic tac toe  "), "Tic tac toe");
    assert_eq!(find_query("OpEn Tic tac toe"), "Tic tac toe");
    assert_eq!(*vm.data(), before);
    assert!(requests.borrow().is_empty());
}

#[test]
fn unavailable_app_commands_stay_local_without_writes_or_host_requests() {
    let mut hidden = app("tests.hidden", "Hidden game");
    hidden.manifest.listed = false;
    let mut people = AppRegistration::builtin(AppId::People, builtin);
    people.manifest.listed = false;
    let mut data = PhoneData::default();
    data.installed.retain(|id| *id != AppId::OneNote);
    let (vm, ui, requests) = fixture(
        vec![
            app("tests.disabled", "Disabled game"),
            hidden,
            people,
            AppRegistration::builtin(AppId::OneNote, builtin),
        ],
        data,
    );
    vm.set_extension_enabled("tests.disabled", false).unwrap();
    let before = vm.data().clone();
    for value in [
        "open Disabled game",
        "open tests.disabled",
        "open disabled",
        "find Disabled game",
        "open Hidden game",
        "open tests.hidden",
        "open hidden",
        "open contacts",
        "open notes",
        "open missing",
    ] {
        submit(&vm, &ui, value);
        assert_eq!(vm.route(), Route::Search, "{value}");
        assert_eq!(
            vm.status(),
            "0 matches on your phone. Choose a result, or search the web."
        );
        assert!(ui
            .registry()
            .search_local(&vm, find_query(value))
            .is_empty());
    }
    assert_eq!(*vm.data(), before);
    assert!(requests.borrow().is_empty());
}

#[test]
fn ambiguous_titles_require_selection_and_cannot_fall_back_to_legacy_aliases() {
    let (vm, ui, requests) = fixture(
        vec![
            app("tests.notes-a", "Notes"),
            app("tests.notes-b", "Notes"),
            app("tests.paint", "Paint"),
            app("tests.paint3d", "Paint 3D"),
            app("tests.Tool", "Upper tool"),
            app("tests.tool", "Lower tool"),
            AppRegistration::builtin(AppId::OneNote, builtin),
        ],
        PhoneData::default(),
    );
    for value in [
        "open notes",
        "open pai",
        "open tests.Tool",
        "open tests.tool",
    ] {
        submit(&vm, &ui, value);
        assert_eq!(vm.route(), Route::Search);
        assert_eq!(
            vm.status(),
            "2 matches on your phone. Choose a result, or search the web."
        );
    }
    submit(&vm, &ui, "open paint");
    assert_eq!(
        vm.route(),
        Route::Extension {
            app: "tests.paint".into(),
            path: String::new()
        }
    );
    assert!(requests.borrow().is_empty());
}

#[test]
fn legacy_aliases_resolve_registered_builtin_owners_even_when_their_titles_change() {
    let mut people = AppRegistration::builtin(AppId::People, builtin);
    people.manifest.title = "Address book";
    let (vm, ui, requests) = fixture(
        vec![
            people,
            AppRegistration::builtin(AppId::InternetExplorer, builtin),
            AppRegistration::builtin(AppId::Messaging, builtin),
            AppRegistration::builtin(AppId::Outlook, builtin),
            AppRegistration::builtin(AppId::OneNote, builtin),
        ],
        PhoneData::default(),
    );
    for (alias, target) in [
        ("contacts", AppId::People),
        ("browser", AppId::InternetExplorer),
        ("messages", AppId::Messaging),
        ("mail", AppId::Outlook),
        ("notes", AppId::OneNote),
    ] {
        submit(&vm, &ui, &format!("open {alias}"));
        assert_eq!(vm.route(), Route::App(target));
    }
    assert!(requests.borrow().is_empty());
}

#[test]
fn local_contacts_notes_and_documents_remain_actionable() {
    let mut data = PhoneData::default();
    data.contacts.push(Contact {
        id: 101,
        name: "Alex Smith".into(),
        ..Contact::default()
    });
    data.notes.push(Note {
        id: 102,
        title: "Trip checklist".into(),
        ..Note::default()
    });
    data.documents.push(OfficeDocument {
        id: 103,
        name: "Quarterly plan".into(),
        ..OfficeDocument::default()
    });
    let (vm, ui, requests) = fixture(
        vec![
            AppRegistration::builtin(AppId::People, builtin),
            AppRegistration::builtin(AppId::OneNote, builtin),
            AppRegistration::builtin(AppId::Office, builtin),
        ],
        data,
    );
    let before = vm.data().clone();
    for (query, target) in [
        ("find Alex", Route::Contact(101)),
        ("find checklist", Route::Note(102)),
        ("find Quarterly", Route::Document(103)),
    ] {
        submit(&vm, &ui, query);
        assert_eq!(vm.route(), target);
    }
    assert_eq!(*vm.data(), before);
    assert!(requests.borrow().is_empty());
    submit(&vm, &ui, "search FerroUI");
    assert!(
        matches!(requests.borrow().as_slice(), [HostRequest::OpenUrl {url}] if url == "https://www.bing.com/search?q=ferroui")
    );
}
