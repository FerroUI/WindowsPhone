use super::*;
mod content;
use phone_controls::{action, app_bar, flat, input, pivot_pages, PivotPage};
use phone_domain::DocumentKind;
use phone_state::HostRequest;

pub(super) fn view(vm: Rc<PhoneViewModel>, ui: Rc<Session>) -> Element {
    let state = ui.state::<phone_office_support::OfficeSession>();
    if state.show_new.get() {
        return new_page(vm, ui);
    }
    if state.show_find.get() {
        return search_page(vm, ui);
    }
    let body = pivot_pages(
        "OfficePivot",
        ui.pivot,
        vec![
            PivotPage::new("recent", content::recent(vm.clone(), ui.clone())),
            PivotPage::new("places", content::places(vm.clone())),
        ],
    );
    page("office", body, commands(vm, ui))
}

// Editing belongs outside the horizontal gesture surface. Back closes search
// before traversing the app journal; the selected hub pane remains untouched.
fn search_page(vm: Rc<PhoneViewModel>, ui: Rc<Session>) -> Element {
    let body = rsx! {Grid { name:"OfficeSearchPage",row_definitions:"Auto,Auto,*",
        {title("search")}
        Border { Grid.row:1,margin:(20.0,0.0,20.0,12.0),
            {input("search documents").name("OfficeSearchInput").text(ui.query.two_way())}
        }
        Border { Grid.row:2,{content::recent(vm.clone(), ui.clone())} }
    }};
    page("office", body, commands(vm, ui))
}

fn commands(vm: Rc<PhoneViewModel>, ui: Rc<Session>) -> Element {
    if ui.menu.get() {
        if let Some(document) = ui
            .selected
            .get()
            .and_then(|id| vm.data().document(id).cloned())
        {
            return context_bar(vm, ui, document);
        }
    }
    let new = ui.clone();
    let search = ui.clone();
    app_bar(
        vec![
            action(Icon::Plus, "new", move || {
                new.state::<phone_office_support::OfficeSession>()
                    .show_new
                    .set(true)
            }),
            action(Icon::Search, "search", move || {
                search
                    .state::<phone_office_support::OfficeSession>()
                    .show_find
                    .set(true)
            }),
            action(Icon::Upload, "import", move || {
                vm.request(HostRequest::ImportDocuments { folder: None })
            }),
        ],
        false,
        vec![],
    )
}

fn context_bar(vm: Rc<PhoneViewModel>, ui: Rc<Session>, document: OfficeDocument) -> Element {
    let close = ui.clone();
    let rename_vm = vm.clone();
    let rename_ui = ui.clone();
    let rename_doc = document.clone();
    let pin = vm.clone();
    let id = document.id;
    let export_vm = vm.clone();
    let export_ui = ui.clone();
    let export_doc = document.clone();
    let delete_vm = vm.clone();
    let delete_ui = ui.clone();
    app_bar(
        vec![action(Icon::Close, "close", move || {
            close.menu.set(false);
            close.selected.set(None);
        })],
        true,
        vec![
            flat("pin to start", move || {
                pin.pin_document(id);
                pin.home();
            }),
            flat("rename", move || {
                phone_office_support::commands::rename(
                    rename_vm.clone(),
                    rename_ui.clone(),
                    rename_doc.clone(),
                )
            }),
            flat("save a copy", move || {
                phone_office_support::commands::export(
                    export_vm.clone(),
                    export_ui.clone(),
                    export_doc.clone(),
                )
            }),
            flat("delete", move || {
                phone_office_support::commands::delete(delete_vm.clone(), delete_ui.clone(), id)
            }),
        ],
    )
}
fn new_page(vm: Rc<PhoneViewModel>, ui: Rc<Session>) -> Element {
    let mut body = stack_panel().child(title("new"));
    for (kind, name, detail) in [
        (DocumentKind::Word, "Word document", "A blank document"),
        (DocumentKind::Excel, "Excel workbook", "A blank workbook"),
    ] {
        let model = vm.clone();
        let state = ui.clone();
        let document = OfficeDocument::blank(kind, name.into(), None);
        body = body.child(document_row(&document, detail, move || {
            phone_office_support::commands::create(model.clone(), state.clone(), kind, None)
        }));
    }
    let cancel = ui.clone();
    page(
        "office",
        scroll(body.into()),
        app_bar(
            vec![action(Icon::Back, "cancel", move || {
                cancel
                    .state::<phone_office_support::OfficeSession>()
                    .show_new
                    .set(false)
            })],
            false,
            vec![],
        ),
    )
}
