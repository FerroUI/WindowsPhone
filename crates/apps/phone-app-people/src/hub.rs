//! The existing local/device sections are adaptations, not retired social feeds.
use super::*;
use ferroui_controls::TextBox;
use phone_controls::{PanoramaPage, input, minimized_app_bar, panorama_pages};
use phone_sdk::common::{empty, panorama_page, row, title};

mod contacts;
mod state;
#[cfg(test)]
mod tests;
pub(crate) use state::{SearchScope, State};

pub(super) fn view(vm: Rc<PhoneViewModel>, ui: Rc<Session>) -> Element {
    let state = ui.state::<State>();
    let epoch = vm.document_epoch();
    if state.epoch.get() != Some(epoch) {
        state.reset();
        state.epoch.set(Some(epoch));
        let device = ui.state::<device::State>();
        device.selected.set(None);
        device.query.set(String::new());
    }
    let native = vm
        .platform_state()
        .supports(phone_sdk::host::platform::HostFeature::Contacts);
    ui.state::<device::State>().available.set(native);
    let mode = ui.pivot.get().min(if native { 2 } else { 1 });
    if native && mode == 2 {
        if let Some(detail) = device::selected_detail(vm.clone(), ui.clone()) {
            return detail;
        }
    }
    if let Some(scope) = state.search.get() {
        if scope != SearchScope::Device || native {
            return search_page(vm, ui, scope);
        }
        state.search.set(None);
    }
    let mut pages = vec![
        PanoramaPage::new("all", contacts::pane(vm.clone(), ui.clone(), false, false)),
        PanoramaPage::new(
            "favorites",
            contacts::pane(vm.clone(), ui.clone(), true, false),
        ),
    ];
    if native {
        pages.push(PanoramaPage::new(
            "device",
            device::pane(vm.clone(), ui.clone(), false),
        ));
    }
    let body = rsx! {Grid {name:"PeopleRoot",
        {panorama_pages("PeoplePanorama","people",ui.pivot,pages)}
        if ui.alphabet.get() && mode != 2 {Border {background:phone_controls::palette().background,
            {alphabet(vm.clone(),ui.clone(),mode == 1,false)}
        }}
    }};
    panorama_page(body, commands(vm, ui, mode, true))
}

fn search_page(vm: Rc<PhoneViewModel>, ui: Rc<Session>, scope: SearchScope) -> Element {
    let state = ui.state::<State>();
    let device = scope == SearchScope::Device;
    let query = if device {
        ui.state::<crate::device::State>().query
    } else {
        state.query
    };
    let focus_vm = vm.clone();
    let focus_ui = ui.clone();
    let epoch = vm.document_epoch();
    let body = rsx! {Grid {name:"PeopleSearchPage",row_definitions:"Auto,Auto,*",
        {title("search")}
        Border {Grid.row:1,margin:(20.0,0.0,20.0,12.0),
            {input(if device {"search device contacts"} else {"search contacts"})
                .name(if device {"DevicePeopleSearch"} else {"PeopleSearchInput"})
                .text(query.two_way()).on_loaded(move |event| {
                    if focus_vm.document_epoch() == epoch && focus_vm.route() == Route::App(AppId::People)
                        && !focus_ui.locked.get_untracked()
                        && focus_ui.state::<State>().search.get_untracked() == Some(scope)
                    {
                        if let Some(editor) = event.source().and_then(|source|source.cast::<TextBox>()) {editor.focus();}
                    }
                })}
        }
        Border {Grid.row:2,
            {if device {crate::device::pane(vm.clone(),ui.clone(),true)}
                else {contacts::pane(vm.clone(),ui.clone(),scope == SearchScope::Favorites,true)}}
        }
        if ui.alphabet.get() && !device {Border {Grid.row_span:3,background:phone_controls::palette().background,
            {alphabet(vm.clone(),ui.clone(),scope == SearchScope::Favorites,true)}
        }}
    }};
    page(
        "people",
        body,
        commands(
            vm,
            ui,
            if device {
                2
            } else if scope == SearchScope::Favorites {
                1
            } else {
                0
            },
            false,
        ),
    )
}

fn commands(vm: Rc<PhoneViewModel>, ui: Rc<Session>, mode: usize, minimized: bool) -> Element {
    let mut actions = if mode == 2 {
        device::actions(vm.clone(), ui.clone())
    } else {
        let add_vm = vm.clone();
        let add_ui = ui.clone();
        let jump = ui.alphabet;
        vec![
            action(Icon::Plus, "new", move || {
                super::edit(add_vm.clone(), add_ui.clone(), Contact::default())
            }),
            action(Icon::List, "jump", move || jump.set(true)),
        ]
    };
    if ui.state::<State>().search.get().is_none() {
        let search = ui.clone();
        actions.push(action(Icon::Search, "search", move || {
            search.alphabet.set(false);
            search.state::<device::State>().query.set(String::new());
            search.state::<State>().begin_search(mode);
        }));
    }
    let all_vm = vm.clone();
    let mut secondary = if mode == 2 {
        Vec::new()
    } else {
        phone_app_support::pim::contacts(vm, ui.clone(), None)
    };
    let all = ui.clone();
    secondary.push(flat("show all contacts", move || {
        close_search(&all_vm, &all);
        all.state::<State>().search.set(None);
        all.state::<State>().query.set(String::new());
        all.pivot.set(0);
        all.alphabet.set(false);
        all.menu.set(false);
    }));
    if minimized {
        minimized_app_bar(actions, ui.menu.get(), secondary)
    } else {
        app_bar(actions, ui.menu.get(), secondary)
    }
}

pub(crate) fn close_search(vm: &PhoneViewModel, ui: &Session) -> bool {
    let state = ui.state::<State>();
    let Some(scope) = state.search.get_untracked() else {
        return false;
    };
    state.search.set(None);
    state.query.set(String::new());
    ui.alphabet.set(false);
    if scope == SearchScope::Device {
        let device = ui.state::<device::State>();
        let filtered = !device.query.get_untracked().is_empty();
        device.query.set(String::new());
        // Refreshing while searching can have fetched a provider-filtered set.
        // Leaving search restores the full permitted device list through its API.
        if filtered {
            device::refresh(vm, ui);
        }
    }
    true
}

fn alphabet(vm: Rc<PhoneViewModel>, ui: Rc<Session>, favorites: bool, searching: bool) -> Element {
    let state = ui.state::<State>();
    let query = if searching {
        state.query.get().trim().to_lowercase()
    } else {
        String::new()
    };
    let available = contacts::letters(&vm, favorites, &query);
    let memory = if searching {
        &state.local_search
    } else if favorites {
        &state.favorites
    } else {
        &state.all
    };
    let mut letters = grid()
        .column_definitions("*,*,*,*")
        .row_definitions("60,60,60,60,60,60,60")
        .row_spacing(8.0)
        .column_spacing(8.0)
        .margin(20.0);
    for (index, letter) in std::iter::once('#').chain('a'..='z').enumerate() {
        let close = ui.alphabet;
        let groups = memory.scroll.clone();
        letters=letters.child(rsx!{Button {Grid.row:(index/4)as i32,Grid.column:(index%4)as i32,
            class:"phone-button",horizontal_alignment:H::Stretch,vertical_alignment:V::Stretch,
            content:letter.to_string(),font_size:30.0,is_enabled:available.contains(&letter),
            automation_properties_name:format!("Jump to {letter}"),
            on_click:move |event|{groups.jump(&letter.to_string());close.set(false);event.set_handled(true);}
        }});
    }
    scroll(letters.into())
}
