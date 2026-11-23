use super::{
    navigation::{browse, change, show},
    session::BrowserView,
};
use ferroui_base::layout::{HorizontalAlignment as H, VerticalAlignment as V};
use ferroui_base::media::{FontWeight, TextWrapping};
use ferroui_view::prelude::*;
use phone_controls::{flat, icon, Icon};
use phone_domain::{Action, AddressButton, BrowserAction};
use phone_sdk::{common::*, session::Session};
use phone_state::PhoneViewModel;
use std::rc::Rc;

pub(super) fn tabs(vm: Rc<PhoneViewModel>, ui: Rc<Session>) -> Element {
    let data = vm.data();
    let tabs = data.browser.tabs.clone();
    let active = data.browser.active;
    drop(data);
    let mut cards = uniform_grid().columns(2).margin((14.0, 0.0));
    for tab in tabs {
        let id = tab.id;
        let open = vm.clone();
        let open_ui = ui.clone();
        let close = vm.clone();
        let close_ui = ui.clone();
        let host = tab.title();
        let url = tab.url().to_owned();
        let border = if active == id { accent(&vm) } else { fg(&vm) };
        cards=cards.child(rsx!{Grid{key:format!("browser-tab-{id}"),margin:5.0,height:172.0,
            Button{name:format!("BrowserTab-{id}"),class:"phone-flat",automation_properties_name:format!("Switch to tab {host}"),
                on_click:move|_|change(&open,&open_ui,BrowserAction::ActivateTab(id)),
                Border{border_brush:border.as_str(),border_thickness:2.0,padding:(12.0,32.0,10.0,10.0),background:phone_controls::palette().surface,
                    StackPanel{spacing:9.0,{icon(Icon::Explorer,33.0,&accent(&vm))},
                        TextBlock{text:host,font_size:18.0,text_wrapping:TextWrapping::Wrap,font_weight:FontWeight::SemiBold},
                        TextBlock{text:url,font_size:12.0,text_wrapping:TextWrapping::Wrap,foreground:phone_controls::secondary_color()}
                    }
                }
            }
            Button{name:format!("CloseBrowserTab-{id}"),class:"phone-flat",width:38.0,height:38.0,horizontal_alignment:H::Right,vertical_alignment:V::Top,automation_properties_name:"Close browser tab",
                on_click:move|event|{event.set_handled(true);close.dispatch(Action::Browser(BrowserAction::CloseTab(id)));super::navigation::sync(&close,&close_ui,true);show(&close_ui,BrowserView::Tabs);},
                {icon(Icon::Close,16.0,&fg(&vm))}}
        }});
    }
    let new = vm.clone();
    let new_ui = ui.clone();
    scroll(
        stack_panel()
            .spacing(8.0)
            .child(title("tabs"))
            .child(cards)
            .child(border().margin(20.0).child(flat("+ new tab", move || {
                change(&new, &new_ui, BrowserAction::NewTab)
            })))
            .into(),
    )
}

pub(super) fn favorites(vm: Rc<PhoneViewModel>, ui: Rc<Session>) -> Element {
    let bookmarks = vm.data().bookmarks.clone();
    let mut rows = stack_panel().child(title("favorites"));
    if bookmarks.is_empty() {
        rows = rows.child(empty(
            "save your favorite sites",
            "Open a page, tap …, then add to favorites.",
        ));
    }
    for bookmark in bookmarks {
        let open = vm.clone();
        let state = ui.clone();
        let url = bookmark.url.clone();
        let remove = vm.clone();
        let id = bookmark.id;
        let edit = vm.clone();
        let edit_ui = ui.clone();
        let original = bookmark.clone();
        rows=rows.child(rsx!{Grid{column_definitions:"*,44",
            Button{name:format!("Context-Bookmark-{id}"),class:"phone-flat",padding:(20.0,12.0),horizontal_content_alignment:H::Stretch,automation_properties_name:bookmark.title.clone(),
                on_click:move|_|browse(&open,&state,&url),
                on_context_requested:move|event|{event.set_handled(true);let save=edit.clone();let sheet=edit_ui.clone();edit_ui.form("edit favorite",vec![("name",original.title.clone()),("address",original.url.clone())],Rc::new(move|values|{
                    match phone_app_support::urls::web_url(&values[1]).and_then(|url|phone_domain::canonical_url(&url)){
                        Ok(url)=>{save.dispatch(Action::SaveBookmark(phone_domain::Bookmark{id,title:values[0].clone(),url}));sheet.dialog.set(None);},
                        Err(error)=>sheet.form_error.set(error),
                    }
                }));},
                StackPanel{spacing:4.0,{text(&bookmark.title,25.0)}{muted(&bookmark.url,14.0)}}
            }
            Button{Grid.column:1,class:"phone-flat",automation_properties_name:format!("Delete favorite {}",bookmark.title),on_click:move|_|remove.delete_bookmark(id),{icon(Icon::Close,16.0,&fg(&vm))}}
        }});
    }
    scroll(rows.into())
}

pub(super) fn history(vm: Rc<PhoneViewModel>, ui: Rc<Session>) -> Element {
    let history = vm.data().browser.history.clone();
    let mut rows = stack_panel().child(title("history"));
    if history.is_empty() {
        rows = rows.child(empty(
            "no history yet",
            "Pages opened through the address bar and favorites appear here.",
        ));
    }
    let mut previous = String::new();
    for visit in history {
        let date = chrono::DateTime::from_timestamp(visit.visited + vm.local_now() - vm.now(), 0)
            .unwrap_or_default()
            .format("%A, %d %B")
            .to_string();
        if date != previous {
            rows=rows.child(rsx!{TextBlock{text:date.clone(),font_size:19.0,foreground:accent(&vm).as_str(),margin:(20.0,16.0,20.0,4.0)}});
            previous = date;
        }
        let open = vm.clone();
        let state = ui.clone();
        let url = visit.url.clone();
        let remove = vm.clone();
        let remove_url = visit.url.clone();
        rows=rows.child(rsx!{Grid{column_definitions:"*,44",{row(&visit.title,&visit.url,move||browse(&open,&state,&url))}
            Button{Grid.column:1,class:"phone-flat",automation_properties_name:format!("Remove history {}",visit.title),on_click:move|_|remove.dispatch(Action::Browser(BrowserAction::RemoveHistory(remove_url.clone()))),{icon(Icon::Close,16.0,&fg(&vm))}}
        }});
    }
    let clear = vm.clone();
    let confirm = ui.clone();
    rows=rows.child(rsx!{Border{margin:20.0,{flat("delete history",move||delete_history(clear.clone(),confirm.clone()))}}});
    scroll(rows.into())
}

pub(super) fn settings(vm: Rc<PhoneViewModel>, ui: Rc<Session>) -> Element {
    let selected = vm.data().browser.address_button;
    let mut choices = stack_panel().spacing(5.0);
    for (kind, label) in [
        (AddressButton::Tabs, "tabs"),
        (AddressButton::Favorites, "favorites"),
        (AddressButton::Refresh, "refresh"),
    ] {
        let set = vm.clone();
        choices = choices.child(phone_controls::command(
            &format!("{}{}", if selected == kind { "✓ " } else { "" }, label),
            move || set.dispatch(Action::Browser(BrowserAction::SetAddressButton(kind))),
        ));
    }
    let clear = vm.clone();
    let confirm = ui.clone();
    scroll(rsx! {StackPanel{margin:(20.0,10.0),spacing:18.0,
        TextBlock{text:"INTERNET EXPLORER",font_size:13.0},TextBlock{text:"settings",font_size:46.0,font_weight:FontWeight::Light},
        {text("Use address bar button for",22.0)}{choices}
        {phone_controls::command("delete history",move||delete_history(clear.clone(),confirm.clone()))}
        {muted("Tabs, favorites and address-bar history are saved on this device. Clearing this history does not clear website cookies or your browser's history.",17.0)}
    }})
}

pub(super) fn frequent(vm: Rc<PhoneViewModel>, ui: Rc<Session>) -> Element {
    let data = vm.data();
    let mut sites = data.browser.history.clone();
    sites.sort_by_key(|visit| std::cmp::Reverse((visit.visits, visit.visited)));
    let mut rows = stack_panel().spacing(3.0);
    if sites.is_empty() {
        for bookmark in data.bookmarks.iter().take(6) {
            let open = vm.clone();
            let state = ui.clone();
            let url = bookmark.url.clone();
            rows = rows.child(row(&bookmark.title, &bookmark.url, move || {
                browse(&open, &state, &url)
            }));
        }
        if data.bookmarks.is_empty() {
            rows = rows.child(muted(
                "Search or enter an address below to start browsing.",
                22.0,
            ));
        }
    } else {
        for site in sites.into_iter().take(6) {
            let open = vm.clone();
            let state = ui.clone();
            let url = site.url.clone();
            rows = rows.child(row(&site.title, &site.url, move || {
                browse(&open, &state, &url)
            }));
        }
    }
    rows.into()
}
fn delete_history(vm: Rc<PhoneViewModel>, ui: Rc<Session>) {
    let sheet = ui.clone();
    ui.confirm(
        "Delete address-bar history?",
        Rc::new(move || {
            vm.dispatch(Action::Browser(BrowserAction::ClearHistory));
            sheet.dialog.set(None);
        }),
    );
}
