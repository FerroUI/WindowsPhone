use crate::{
    commands,
    model::{Barcode, Card},
};
use phone_controls::flat as menu_action;
use phone_sdk::prelude::*;
use phone_state::HostRequest;
pub fn view(cx: AppContext) -> Element {
    let saved = match commands::load(&cx) {
        Ok(saved) => saved,
        Err(error) => {
            return page(
                "wallet",
                empty("Wallet unavailable", &error),
                Element::Empty,
            )
        }
    };
    if let Route::Extension { path, .. } = cx.vm.route() {
        if let Some(card) = path
            .strip_prefix("card/")
            .and_then(|id| id.parse::<u32>().ok())
            .and_then(|id| saved.cards.iter().find(|card| card.id == id))
        {
            return details(cx, card);
        }
    }
    let mut tabs = stack_panel()
        .orientation(ferroui_base::layout::Orientation::Horizontal)
        .spacing(18.0);
    for (index, label) in ["all", "favorites", "archived"].into_iter().enumerate() {
        let ui = cx.session.clone();
        tabs = tabs.child(pivot(label, ui.pivot.get() == index, move || {
            ui.pivot.set(index)
        }));
    }
    let query = cx.session.query.get().to_lowercase();
    let pivot = cx.session.pivot.get();
    let tabs =
        phone_controls::pivot_header(tabs.into(), ["all", "favorites", "archived"][pivot.min(2)]);
    let mut body = stack_panel().margin(20.0).spacing(18.0).child(tabs).child(
        input("Search cards")
            .name("WalletSearch")
            .text(cx.session.query.two_way()),
    );
    let cards = saved
        .cards
        .iter()
        .filter(|card| {
            if pivot == 2 {
                card.archived
            } else {
                !card.archived && (pivot != 1 || card.favorite)
            }
        })
        .filter(|card| {
            format!("{} {} {}", card.title, card.issuer, card.notes)
                .to_lowercase()
                .contains(&query)
        })
        .collect::<Vec<_>>();
    if cards.is_empty() {
        body = body.child(empty(
            "your wallet",
            "Keep membership cards, tickets and offers together. Add a card to get started.",
        ));
    }
    for card in cards {
        let open = cx.clone();
        let path = format!("card/{}", card.id);
        let status = format!(
            "{}{}{}",
            card.kind.label(),
            if card.expired(cx.vm.local_now()) {
                " · expired"
            } else {
                ""
            },
            if card.favorite { " · favorite" } else { "" }
        );
        body=body.child(rsx!{Button{name:format!("WalletCard-{}",card.id),class:"phone-row",padding:16.0,background:"#174E75",automation_properties_name:card.title.clone(),on_click:move |_|open.navigate(&path),StackPanel{spacing:7.0,TextBlock{text:card.issuer.clone(),foreground:"White",font_size:15.0},TextBlock{text:card.title.clone(),foreground:"White",font_size:27.0,text_wrapping:ferroui_base::media::TextWrapping::Wrap},TextBlock{text:status,foreground:"White",font_size:14.0}}}});
    }
    body=body.child(muted("Cards are saved with this phone and included in its backups. This wallet stores membership information; payments and NFC are unavailable.",14.0));
    let add = cx.clone();
    let import = cx.clone();
    let backup = cx.clone();
    let more = cx.session.clone();
    page(
        "wallet",
        scroll(body.into()),
        app_bar(
            vec![
                action(Icon::Plus, "add", move || {
                    commands::edit(add.clone(), Card::default())
                }),
                action(Icon::Download, "import", move || {
                    commands::import(import.clone())
                }),
                action(Icon::More, "more", move || {
                    more.menu.set(!more.menu.get_untracked())
                }),
            ],
            cx.session.menu.get(),
            vec![menu_action("copy wallet JSON", move || {
                commands::copy_backup(&backup)
            })],
        ),
    )
}
fn details(cx: AppContext, card: &Card) -> Element {
    let mut body = stack_panel()
        .margin(20.0)
        .spacing(18.0)
        .child(text(&card.issuer, 21.0))
        .child(text(&card.title, 38.0))
        .child(muted(card.kind.label(), 16.0));
    if card.barcode == Barcode::Code39 {
        match phone_controls::barcode::code39(&card.code) {
            Ok(code) => {
                let width = (cx.vm.viewport().0.min(400.0) - 40.0).max(80.0).min(
                    phone_controls::barcode::code39_modules(&card.code)
                        .map_or(305.0, |bits| bits.len() as f64),
                );
                body=body.child(rsx!{Viewbox{name:"WalletBarcode",width:width,stretch:ferroui_base::media::Stretch::Uniform,{code}}});
            }
            Err(error) => body = body.child(muted(&error, 16.0)),
        }
    }
    if !card.code.is_empty() {
        body = body.child(text(&card.code, 28.0));
    }
    if !card.expires.is_empty() {
        body = body.child(muted(
            &format!(
                "{} {}",
                if card.expired(cx.vm.local_now()) {
                    "expired"
                } else {
                    "expires"
                },
                card.expires
            ),
            19.0,
        ));
    }
    if !card.notes.is_empty() {
        body = body.child(text(&card.notes, 20.0));
    }
    let id = card.id;
    let edit = cx.clone();
    let edited = card.clone();
    let favorite = cx.clone();
    let pin = cx.clone();
    let title = card.title.chars().take(80).collect::<String>();
    let more = cx.session.clone();
    let barcode = cx.clone();
    let archive = cx.clone();
    let kind = cx.clone();
    let remove = cx.clone();
    let copy = cx.clone();
    let value = card.code.clone();
    let share = cx.clone();
    let shared = card.clone();
    let menu = vec![
        menu_action(
            if card.barcode == Barcode::Code39 {
                "use text code"
            } else {
                "use Code 39 barcode"
            },
            move || {
                commands::update(&barcode, id, |card| {
                    card.barcode = if card.barcode == Barcode::Code39 {
                        Barcode::Text
                    } else {
                        Barcode::Code39
                    }
                })
            },
        ),
        menu_action("change card type", move || {
            commands::update(&kind, id, |card| card.kind = card.kind.next())
        }),
        menu_action(
            if card.archived {
                "unarchive"
            } else {
                "archive"
            },
            move || commands::update(&archive, id, |card| card.archived = !card.archived),
        ),
        menu_action("copy code", move || {
            commands::report(
                &copy,
                copy.request(HostRequest::CopyText {
                    text: value.clone(),
                }),
            )
        }),
        menu_action("share card", move || {
            commands::report(
                &share,
                share.request(HostRequest::Share {
                    title: shared.title.clone(),
                    text: format!(
                        "{}\n{}\n{}\n{}",
                        shared.issuer, shared.code, shared.expires, shared.notes
                    ),
                    url: String::new(),
                }),
            )
        }),
        menu_action("delete", move || commands::remove(remove.clone(), id)),
    ];
    page(
        "wallet",
        scroll(body.into()),
        app_bar(
            vec![
                action(Icon::Edit, "edit", move || {
                    commands::edit(edit.clone(), edited.clone())
                }),
                action(
                    Icon::Star,
                    if card.favorite {
                        "unfavorite"
                    } else {
                        "favorite"
                    },
                    move || commands::update(&favorite, id, |card| card.favorite = !card.favorite),
                ),
                action(Icon::Pin, "pin card", move || {
                    commands::report(&pin, pin.pin(&format!("card/{id}"), &title))
                }),
                action(Icon::More, "more", move || {
                    more.menu.set(!more.menu.get_untracked())
                }),
            ],
            cx.session.menu.get(),
            menu,
        ),
    )
}
