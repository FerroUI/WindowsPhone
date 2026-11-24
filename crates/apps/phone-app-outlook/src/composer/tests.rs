use super::*;
use phone_domain::{AppId, PhoneData, Route};
use phone_sdk::AppRegistry;
use phone_state::{HostRequest, HostServices};
use std::cell::Cell;

fn fixture() -> (Rc<PhoneViewModel>, Rc<Session>, Rc<Cell<usize>>) {
    let writes = Rc::new(Cell::new(0));
    let count = writes.clone();
    let vm = PhoneViewModel::new(
        PhoneData::demo(),
        Rc::new(move |_| count.set(count.get() + 1)),
    );
    let registry = Rc::new(AppRegistry::new(vec![crate::registration()]).unwrap());
    registry.bind(&vm);
    let ui = Rc::new(Session::new(vm.now(), registry));
    vm.navigate(Route::App(AppId::Outlook));
    (vm, ui, writes)
}

fn fields() -> Fields {
    Fields {
        to: "ada@example.com".into(),
        subject: "Draft test".into(),
        body: "First line\nSecond line".into(),
    }
}

#[test]
fn back_requires_discard_for_unsaved_edits_and_preserves_saved_version() {
    let (vm, ui, writes) = fixture();
    begin(&ui, fields(), None);
    assert!(ui.registry().back(&vm, &ui));
    assert!(ui.dialog.get_untracked().is_some());
    assert!(ui.state::<State>().active.get_untracked());
    assert_eq!(writes.get(), 0);
    ui.dialog.set(None); // The user cancels the shared dialog.
    save(&vm, &ui);
    let saved = drafts::read(&vm).unwrap();
    ui.state::<State>().body.set("Unsaved replacement".into());
    assert!(ui.registry().back(&vm, &ui));
    let dialog = ui.dialog.get_untracked().unwrap();
    assert_eq!(dialog.action_label, "discard");
    (dialog.save)(vec![]);
    ui.dialog.set(None);
    assert!(!ui.state::<State>().active.get_untracked());
    assert_eq!(drafts::read(&vm).unwrap(), saved);
    assert_eq!(writes.get(), 1);
    let draft = saved.items[0].clone();
    begin(&ui, draft.fields.clone(), Some(draft));
    assert!(ui.registry().back(&vm, &ui));
    assert!(
        ui.dialog.get_untracked().is_none(),
        "An unchanged saved draft closes directly"
    );
}

#[test]
fn handoff_retains_editor_and_saved_draft_without_claiming_sent_mail() {
    let (vm, ui, writes) = fixture();
    let requests = Rc::new(RefCell::new(Vec::new()));
    let received = requests.clone();
    vm.set_services(HostServices {
        request: Rc::new(move |request| received.borrow_mut().push(request)),
    });
    begin(&ui, fields(), None);
    save(&vm, &ui);
    let before = vm.data().clone();
    let count = writes.get();
    ui.state::<State>()
        .body
        .set("Unsaved\ntext handed to host".into());
    open_email(&vm, &ui);
    assert_eq!(*vm.data(), before);
    assert_eq!(writes.get(), count);
    assert!(ui.state::<State>().active.get_untracked());
    assert!(
        matches!(requests.borrow().first(), Some(HostRequest::Email { body, .. })
        if body == "Unsaved\ntext handed to host")
    );
    ui.state::<State>().to.set("missing address".into());
    open_email(&vm, &ui);
    assert_eq!(
        requests.borrow().len(),
        1,
        "Invalid recipients never reach the host"
    );
    assert!(!ui.state::<State>().error.get_untracked().is_empty());
}

#[test]
fn app_switch_keeps_unsaved_editor_but_document_reset_clears_it() {
    let (vm, ui, writes) = fixture();
    begin(&ui, fields(), None);
    let registry = ui.registry();
    registry.deactivate(&vm.route(), &ui);
    assert!(ui.state::<State>().active.get_untracked());
    assert_eq!(ui.state::<State>().fields(), fields());
    assert_eq!(registry.pivot_count(&vm.route(), &ui), 0);
    assert_eq!(writes.get(), 0);
    registry.reset_app(&vm, &ui, drafts::APP_ID);
    assert!(!ui.state::<State>().active.get_untracked());
    assert_eq!(ui.state::<State>().fields(), Fields::default());
}

#[test]
fn local_drafts_return_to_the_previous_mailbox_even_when_it_is_named_drafts() {
    let (vm, ui, writes) = fixture();
    let mail = ui.state::<crate::MailState>();
    mail.folder.set("drafts".into());
    assert_eq!(ui.registry().pivot_count(&vm.route(), &ui), 3);
    mail.saved_drafts.set(true);
    assert_eq!(ui.registry().pivot_count(&vm.route(), &ui), 0);
    assert!(ui.registry().back(&vm, &ui));
    assert!(!mail.saved_drafts.get_untracked());
    assert_eq!(mail.folder.get_untracked(), "drafts");
    assert_eq!(ui.registry().pivot_count(&vm.route(), &ui), 3);
    assert_eq!(writes.get(), 0);
}
