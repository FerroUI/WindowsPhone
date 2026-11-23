use ferroui_base::input::Key;
use ferroui_base::layout::HorizontalAlignment as H;
use ferroui_view::prelude::*;
use phone_controls::{action, app_bar, icon, input, Icon};
use phone_domain::{AppId, Route};
use phone_sdk::{common::*, session::Session};
use phone_state::{HostRequest, PhoneViewModel};
use std::rc::Rc;

pub fn view(vm: Rc<PhoneViewModel>, ui: Rc<Session>) -> Element {
    let query = ui.query.get();
    let submitted = vm.clone();
    let text_ui = ui.clone();
    let field = input("Ask me anything")
        .name("CortanaInput")
        .text(ui.query.two_way())
        .on_key_down(move |event| {
            if event.key == Key::Return {
                event.set_handled(true);
                run(&submitted, &text_ui);
            }
        });
    let mut body = stack_panel().spacing(16.0).margin((20.0, 10.0));
    body=body.child(rsx!{Border{width:80.0,height:80.0,horizontal_alignment:H::Left,margin:(0.0,8.0,0.0,5.0),{icon(Icon::Cortana,72.0,&accent(&vm))}}})
        .child(text("What can I do\nfor you?",35.0)).child(field);
    if query.trim().is_empty() {
        body = body.child(muted(
            "Open an app, find a contact or note, or search the web.",
            20.0,
        ));
        for command in ["open camera", "find Alex", "open calendar"] {
            let state = ui.clone();
            let model = vm.clone();
            body = body.child(row(command, "", move || {
                state.query.set(command.into());
                run(&model, &state);
            }));
        }
    } else {
        let search = find_query(&query);
        for result in vm.search(search) {
            let model = vm.clone();
            let route = result.route;
            body = body.child(row(&result.title, &result.subtitle, move || {
                model.navigate(route.clone())
            }));
        }
        let web = vm.clone();
        let phrase = query.clone();
        body = body.child(row("search the web", &query, move || {
            web.request(HostRequest::OpenUrl {
                url: format!(
                    "https://www.bing.com/search?q={}",
                    phone_app_support::urls::encode(&phrase)
                ),
            })
        }));
    }
    let run_vm = vm.clone();
    let run_ui = ui.clone();
    page(
        "cortana",
        scroll(body.into()),
        app_bar(
            vec![action(Icon::Search, "search", move || {
                run(&run_vm, &run_ui)
            })],
            false,
            vec![],
        ),
    )
}

fn run(vm: &PhoneViewModel, ui: &Session) {
    let value = ui.query.get_untracked();
    let value = value.trim();
    let lower = value.to_lowercase();
    if let Some(name) = lower.strip_prefix("open ") {
        let found = AppId::ALL
            .iter()
            .copied()
            .find(|app| app.label().eq_ignore_ascii_case(name))
            .or(match name {
                "contacts" => Some(AppId::People),
                "browser" => Some(AppId::InternetExplorer),
                "messages" => Some(AppId::Messaging),
                "mail" => Some(AppId::Outlook),
                "notes" => Some(AppId::OneNote),
                _ => None,
            })
            .or_else(|| AppId::search(name).first().copied());
        if let Some(app) = found {
            vm.navigate(Route::App(app));
            return;
        }
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
    let search = find_query(value);
    let results = vm.search(search);
    if results.len() == 1 {
        vm.navigate(results[0].route.clone());
    } else if !value.is_empty() {
        vm.set_status(format!(
            "{} matches on your phone. Choose a result, or search the web.",
            results.len()
        ));
    }
}

fn find_query(value: &str) -> &str {
    if value
        .get(..5)
        .is_some_and(|prefix| prefix.eq_ignore_ascii_case("find "))
    {
        &value[5..]
    } else {
        value
    }
}

mod registration;
pub use registration::registration;
