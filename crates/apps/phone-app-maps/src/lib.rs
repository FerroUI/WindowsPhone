use ferroui_view::prelude::*;
use phone_controls::{action, app_bar, icon, input, Icon};
use phone_sdk::{common::*, session::Session};
use phone_state::{HostRequest, PhoneViewModel};
use std::rc::Rc;

pub fn view(vm: Rc<PhoneViewModel>, ui: Rc<Session>) -> Element {
    let search = vm.clone();
    let state = ui.clone();
    let locate = vm.clone();
    let key_vm = vm.clone();
    let key_ui = ui.clone();
    let field = input("Search for a place")
        .text(ui.query.two_way())
        .on_key_down(move |event| {
            if event.key == ferroui_base::input::Key::Return {
                event.set_handled(true);
                find_place(&key_vm, &key_ui);
            }
        });
    let body = rsx! {StackPanel{spacing:24.0,margin:20.0,
        {text("maps",48.0)}{field}
        Border{height:180.0,{icon(Icon::Maps,100.0,&accent(&vm))}}
        {text("Where do you want to go?",28.0)}
        {muted("Find a place, explore its map, or use your current location.",20.0)}
    }};
    page(
        "maps",
        scroll(body),
        app_bar(
            vec![
                action(Icon::Search, "search", move || {
                    find_place(&search, &state);
                }),
                action(Icon::Location, "my location", move || {
                    locate.request(HostRequest::Locate)
                }),
            ],
            false,
            vec![],
        ),
    )
}

fn find_place(vm: &PhoneViewModel, ui: &Session) {
    let query = ui.query.get_untracked();
    let query = query.trim();
    if query.is_empty() {
        vm.set_status("Enter a place to search for.".into());
        return;
    }
    vm.request(HostRequest::OpenUrl {
        url: format!(
            "https://www.openstreetmap.org/search?query={}",
            phone_app_support::urls::encode(query)
        ),
    });
}

mod registration;
pub use registration::registration;
