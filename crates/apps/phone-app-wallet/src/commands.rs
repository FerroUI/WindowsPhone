use crate::model::{Card, Saved};
use phone_sdk::prelude::*;
pub fn load(cx: &AppContext) -> Result<Saved, String> {
    let saved = cx.storage().read::<Saved>()?;
    saved.validate()?;
    Ok(saved)
}
pub fn change(
    cx: &AppContext,
    apply: impl FnOnce(&mut Saved) -> Result<(), String>,
) -> Result<(), String> {
    let mut saved = load(cx)?;
    apply(&mut saved)?;
    saved.validate()?;
    cx.storage().write(&saved)
}
pub fn report(cx: &AppContext, result: Result<(), String>) {
    if let Err(error) = result {
        cx.vm.set_status(error);
    }
}
pub fn edit(cx: AppContext, card: Card) {
    let fields = vec![
        ("Name", card.title.clone()),
        ("Issuer", card.issuer.clone()),
        ("Card code", card.code.clone()),
        ("Expires (YYYY-MM-DD)", card.expires.clone()),
        ("Notes", card.notes.clone()),
    ];
    let edited = cx.clone();
    let epoch = cx.vm.document_epoch();
    cx.session.form(
        if card.id == 0 {
            "add a card"
        } else {
            "edit card"
        },
        fields,
        Rc::new(move |fields| {
            if edited.vm.document_epoch() != epoch {
                return;
            }
            let mut card = card.clone();
            card.title = fields[0].trim().into();
            card.issuer = fields[1].trim().into();
            card.code = fields[2].clone();
            card.expires = fields[3].trim().into();
            card.notes = fields[4].clone();
            let mut id = 0;
            let result = change(&edited, |saved| {
                id = saved.save(card)?;
                Ok(())
            });
            match result {
                Ok(()) => {
                    edited.session.dialog.set(None);
                    edited.navigate(&format!("card/{id}"));
                }
                Err(error) => edited.session.form_error.set(error),
            }
        }),
    );
}
pub fn update(cx: &AppContext, id: u32, apply: impl FnOnce(&mut Card)) {
    report(
        cx,
        change(cx, |saved| {
            let card = saved
                .cards
                .iter_mut()
                .find(|card| card.id == id)
                .ok_or("This card no longer exists.")?;
            apply(card);
            card.validate()
        }),
    );
}
pub fn remove(cx: AppContext, id: u32) {
    let confirmed = cx.clone();
    cx.session.confirm(
        "Delete this card?",
        Rc::new(move || {
            match change(&confirmed, |saved| {
                saved.cards.retain(|card| card.id != id);
                Ok(())
            }) {
                Ok(()) => {
                    confirmed.session.dialog.set(None);
                    confirmed.navigate("");
                }
                Err(error) => confirmed.session.form_error.set(error),
            }
        }),
    );
}
pub fn import(cx: AppContext) {
    let origin = cx.vm.route();
    let task = cx.choose_file(phone_sdk::FilePickerOptions {
        extensions: vec![".json".into()],
        max_bytes: 256 * 1024,
    });
    let epoch = cx.vm.document_epoch();
    drop(
        ferroui_base::threading::Dispatcher::current_dispatcher().spawn_local(async move {
            let result = task.await;
            if cx.vm.document_epoch() != epoch {
                return;
            }
            match result {
                Ok(Some(file)) => {
                    let mut count = 0;
                    match change(&cx, |saved| {
                        count = saved.import(&file.bytes)?;
                        Ok(())
                    }) {
                        Ok(()) => {
                            cx.vm.set_status(format!("Imported {count} cards."));
                            if cx.vm.route() == origin {
                                cx.navigate("");
                            }
                        }
                        Err(error) => cx.vm.set_status(error),
                    }
                }
                Ok(None) => {}
                Err(error) => cx.vm.set_status(error),
            }
        }),
    );
}
pub fn copy_backup(cx: &AppContext) {
    report(
        cx,
        load(cx)
            .and_then(|saved| {
                serde_json::to_string_pretty(&saved).map_err(|error| error.to_string())
            })
            .and_then(|text| cx.request(phone_state::HostRequest::CopyText { text })),
    );
}
