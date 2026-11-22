use crate::{common::*, session::Session};
use ferroui_base::input::{
    Key,
    text_input::{TextInputContentType, TextInputReturnKeyType},
};
use ferroui_base::layout::{HorizontalAlignment as H, VerticalAlignment as V};
use ferroui_view::prelude::*;
use phone_controls::{Icon, action, app_bar, flat, icon, input};
use phone_state::{HostRequest, PhoneViewModel};
use std::rc::Rc;

pub fn view(vm: Rc<PhoneViewModel>, ui: Rc<Session>) -> Element {
    let url = ui.browser_url.get();
    let bookmarks = ui.pivot.get() == 1;
    let content = if bookmarks {
        favorites(vm.clone(), ui.clone())
    } else if url.is_empty() {
        rsx! {StackPanel{margin:24.0,spacing:24.0,
            Border{height:160.0,{icon(Icon::Explorer,92.0,&accent(&vm))}}
            {text("Internet Explorer",32.0)}
            {muted("Enter an address below, or open a favorite.",20.0)}
            {favorites(vm.clone(),ui.clone())}
        }}
    } else {
        rsx! {Border{name:"BrowserViewport",background:"#FFFFFF",min_height:200.0,
            StackPanel{vertical_alignment:V::Center,horizontal_alignment:H::Center,spacing:15.0,
                {icon(Icon::Globe,60.0,"#999999")}
                TextBlock{text:"opening page…",foreground:"#777777",font_size:20.0}
            }
        }}
    };
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
    let navigate = vm.clone();
    let entry = ui.clone();
    let menu = ui.clone();
    let favorite = vm.clone();
    let favorite_ui = ui.clone();
    let favorites_ui = ui.clone();
    let external = vm.clone();
    let external_url = url.clone();
    let refresh = vm.clone();
    let refresh_url = url.clone();
    let pin = vm.clone();
    let pin_ui = ui.clone();
    let controls = rsx! {Border{padding:(10.0,8.0),background:phone_controls::palette().surface,
        Grid{column_definitions:"*,48,40",column_spacing:6.0,
        {field}
        Border{Grid.column:1,{flat("→",move||browse(&navigate,&entry,&entry.entry.get_untracked()))}}
        Border{Grid.column:2,{flat("…",move||menu.menu.update(|v|*v=!*v))}}
    }}};
    let mut bottom = stack_panel().child(controls);
    if ui.menu.get() {
        bottom = bottom.child(app_bar(
            vec![
                action(Icon::Refresh, "refresh", move || {
                    if !refresh_url.is_empty() {
                        refresh.request(HostRequest::Browse {
                            url: refresh_url.clone(),
                        });
                    }
                }),
                action(Icon::Star, "favorites", move || {
                    favorites_ui.pivot.set(1);
                    favorites_ui.menu.set(false);
                }),
            ],
            true,
            vec![
                flat("pin to start", move || {
                    let url = pin_ui.browser_url.get_untracked();
                    if url.is_empty() {
                        pin.set_status("Open a page before pinning it to Start.".into());
                        return;
                    }
                    let existing = pin
                        .data()
                        .bookmarks
                        .iter()
                        .find(|bookmark| bookmark.url == url)
                        .map(|bookmark| bookmark.id);
                    let id = existing.unwrap_or_else(|| {
                        let id = pin.next_id();
                        pin.dispatch(phone_domain::Action::SaveBookmark(phone_domain::Bookmark {
                            id,
                            title: url.clone(),
                            url,
                        }));
                        id
                    });
                    pin.pin_bookmark(id);
                    pin_ui.menu.set(false);
                }),
                flat("add to favorites", move || {
                    let url = favorite_ui.browser_url.get_untracked();
                    if url.is_empty() {
                        favorite.set_status("Open a page to add it to your favorites.".into());
                        return;
                    }
                    let save = favorite.clone();
                    let sheet = favorite_ui.clone();
                    favorite_ui.form(
                        "add to favorites",
                        vec![("name", url.clone()), ("address", url)],
                        Rc::new(move |values| {
                            if let Ok(url) =
                                super::web_url(values.get(1).map(String::as_str).unwrap_or(""))
                            {
                                save.add_bookmark(
                                    values.first().cloned().unwrap_or_else(|| url.clone()),
                                    url,
                                );
                                sheet.dialog.set(None);
                            }
                        }),
                    );
                }),
                flat("open in browser", move || {
                    if !external_url.is_empty() {
                        external.request(HostRequest::OpenUrl {
                            url: external_url.clone(),
                        });
                    }
                }),
            ],
        ));
    }
    rsx! {Grid{row_definitions:"*,Auto",Border{clip_to_bounds:true,{content}}Border{Grid.row:1,{bottom}}}}
}

fn favorites(vm: Rc<PhoneViewModel>, ui: Rc<Session>) -> Element {
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
        rows = rows.child(rsx! {Grid{column_definitions:"*,44",
            {row(&bookmark.title,&bookmark.url,move||browse(&open,&state,&url))}
            Border{Grid.column:1,{flat("×",move||remove.delete_bookmark(id))}}
        }});
    }
    scroll(rows.into())
}

fn browse(vm: &PhoneViewModel, ui: &Session, value: &str) {
    match super::web_url(value) {
        Ok(url) => {
            ui.browser_url.set(url.clone());
            ui.entry.set(url.clone());
            ui.pivot.set(0);
            ui.menu.set(false);
            vm.request(HostRequest::Browse { url });
        }
        Err(error) => vm.set_status(error),
    }
}
