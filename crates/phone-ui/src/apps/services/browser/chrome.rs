use super::{
    navigation::{browse, change, show},
    session::BrowserView,
};
use crate::{common::*, session::Session};
use ferroui_base::input::{
    Key,
    text_input::{TextInputContentType, TextInputReturnKeyType},
};
use ferroui_base::layout::{HorizontalAlignment as H, VerticalAlignment as V};
use ferroui_view::prelude::*;
use phone_controls::{Icon, action, app_bar, flat, icon, input};
use phone_domain::{Action, AddressButton, BrowserAction};
use phone_state::{HostRequest, PhoneViewModel};
use std::rc::Rc;

pub(super) fn view(vm: Rc<PhoneViewModel>, ui: Rc<Session>) -> Element {
    let data = vm.data();
    let tab = data.browser.current().clone();
    let count = data.browser.tabs.len();
    let shortcut = data.browser.address_button;
    drop(data);
    let typed = ui.clone();
    let go = vm.clone();
    let field = input("Search or enter an address")
        .name("AddressInput")
        .text(ui.entry.two_way())
        .font_size(18.0)
        .min_height(42.0)
        .text_input_options_content_type(TextInputContentType::Url)
        .text_input_options_return_key_type(TextInputReturnKeyType::Go)
        .text_input_options_auto_capitalization(false)
        .on_key_down(move |event| {
            if event.key == Key::Return {
                event.set_handled(true);
                browse(&go, &typed, &typed.entry.get_untracked());
            }
        });
    let shortcut_vm = vm.clone();
    let shortcut_ui = ui.clone();
    let menu = ui.clone();
    let nav = vm.clone();
    let nav_ui = ui.clone();
    let shortcut_name = match shortcut {
        AddressButton::Tabs => format!("Tabs, {count} open"),
        AddressButton::Favorites => "Favorites".into(),
        AddressButton::Refresh => "Refresh page".into(),
    };
    let symbol = match shortcut {
        AddressButton::Tabs => {
            rsx! {Border{width:25.0,height:25.0,border_brush:fg(&vm).as_str(),border_thickness:2.0,TextBlock{text:count.to_string(),font_size:16.0,horizontal_alignment:H::Center,vertical_alignment:V::Center}}}
        }
        AddressButton::Favorites => icon(Icon::Star, 23.0, &fg(&vm)),
        AddressButton::Refresh => icon(Icon::Refresh, 23.0, &fg(&vm)),
    };
    let controls = rsx! {Border{padding:(8.0,8.0),background:phone_controls::palette().surface,
        Grid{column_definitions:"40,*,38,34",column_spacing:4.0,
            Button{name:"BrowserShortcut",class:"phone-flat",automation_properties_name:shortcut_name,on_click:move|_|match shortcut{
                AddressButton::Tabs=>show(&shortcut_ui,BrowserView::Tabs),AddressButton::Favorites=>show(&shortcut_ui,BrowserView::Favorites),
                AddressButton::Refresh=>{let url=shortcut_vm.data().browser.url().to_owned();if !url.is_empty(){shortcut_vm.request(HostRequest::Browse{url});}}
            },{symbol}}
            Border{Grid.column:1,{field}}
            Button{Grid.column:2,class:"phone-flat",automation_properties_name:"Go to address",on_click:move|_|browse(&nav,&nav_ui,&nav_ui.entry.get_untracked()),{icon(Icon::Forward,20.0,&fg(&vm))}}
            Button{Grid.column:3,class:"phone-flat",automation_properties_name:"Browser menu",on_click:move|_|menu.menu.update(|value|*value=!*value),{icon(Icon::More,23.0,&fg(&vm))}}
        }
    }};
    let mut bottom = stack_panel().child(controls);
    if ui.menu.get() {
        let back = vm.clone();
        let back_ui = ui.clone();
        let forward = vm.clone();
        let forward_ui = ui.clone();
        let tabs = ui.clone();
        let favorites = ui.clone();
        let new = vm.clone();
        let new_ui = ui.clone();
        let history = ui.clone();
        let settings = ui.clone();
        let add = vm.clone();
        let add_ui = ui.clone();
        let pin = vm.clone();
        let pin_ui = ui.clone();
        let external = vm.clone();
        let url = tab.url().to_owned();
        let return_ui = ui.clone();
        let mut items = vec![
            flat("new tab", move || {
                change(&new, &new_ui, BrowserAction::NewTab)
            }),
            flat("history", move || show(&history, BrowserView::History)),
            flat("add to favorites", move || {
                favorite(add.clone(), add_ui.clone())
            }),
            flat("pin to start", move || pin_page(&pin, &pin_ui)),
            flat("settings", move || show(&settings, BrowserView::Settings)),
        ];
        if !url.is_empty() {
            items.push(flat("open in browser", move || {
                external.request(HostRequest::OpenUrl { url: url.clone() })
            }));
        }
        items.push(flat("return to page", move || {
            show(&return_ui, BrowserView::Page)
        }));
        bottom=bottom.child(app_bar(vec![
            rsx!{Border{opacity:if tab.can_back(){1.0}else{0.35},is_enabled:tab.can_back(),{action(Icon::Back,"back",move||change(&back,&back_ui,BrowserAction::Back))}}},
            rsx!{Border{opacity:if tab.can_forward(){1.0}else{0.35},is_enabled:tab.can_forward(),{action(Icon::Forward,"forward",move||change(&forward,&forward_ui,BrowserAction::Forward))}}},
            action(Icon::List,"tabs",move||show(&tabs,BrowserView::Tabs)),action(Icon::Star,"favorites",move||show(&favorites,BrowserView::Favorites))
        ],true,items));
    }
    // Expanded commands can scroll on compact landscape/software-keyboard layouts.
    border()
        .max_height((crate::shell::phone_size(&vm).1 - 80.0).clamp(96.0, 360.0))
        .child(scroll(bottom.into()))
        .into()
}
fn favorite(vm: Rc<PhoneViewModel>, ui: Rc<Session>) {
    let url = vm.data().browser.url().to_owned();
    if url.is_empty() {
        vm.set_status("Open a page to add it to favorites.".into());
        return;
    }
    let save = vm.clone();
    let sheet = ui.clone();
    ui.form(
        "add to favorites",
        vec![("name", phone_domain::page_title(&url)), ("address", url)],
        Rc::new(move |values| {
            match super::super::web_url(&values[1])
                .and_then(|url| phone_domain::canonical_url(&url))
            {
                Ok(url) => {
                    save.add_bookmark(values[0].clone(), url);
                    sheet.dialog.set(None);
                }
                Err(error) => sheet.form_error.set(error),
            }
        }),
    );
}
fn pin_page(vm: &PhoneViewModel, ui: &Session) {
    let url = vm.data().browser.url().to_owned();
    if url.is_empty() {
        vm.set_status("Open a page before pinning it to Start.".into());
        return;
    }
    let existing = vm
        .data()
        .bookmarks
        .iter()
        .find(|bookmark| bookmark.url == url)
        .map(|bookmark| bookmark.id);
    let id = existing.unwrap_or_else(|| {
        let id = vm.next_id();
        vm.dispatch(Action::SaveBookmark(phone_domain::Bookmark {
            id,
            title: phone_domain::page_title(&url),
            url,
        }));
        id
    });
    vm.pin_bookmark(id);
    ui.menu.set(false);
}
