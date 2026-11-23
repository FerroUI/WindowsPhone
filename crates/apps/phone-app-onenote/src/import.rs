use ferroui_base::threading::Dispatcher;
use phone_domain::{AppId, Id, Note, Route};
use phone_sdk::{session::Session, AppContext, FilePickerOptions, PickedFile};
use phone_state::PhoneViewModel;
use std::rc::Rc;

const MAX_NOTE_BYTES: u32 = 256 * 1024;

pub fn start(vm: Rc<PhoneViewModel>, session: Rc<Session>) {
    let context = AppContext {
        vm: vm.clone(),
        session,
        id: "phone.onenote",
    };
    // Request synchronously in the app-bar click to retain browser activation.
    let selection = context.choose_file(FilePickerOptions {
        extensions: vec![".txt".into(), ".md".into()],
        max_bytes: MAX_NOTE_BYTES,
    });
    let epoch = vm.document_epoch();
    drop(Dispatcher::current_dispatcher().spawn_local(async move {
        let result = selection.await;
        if vm.document_epoch() != epoch {
            return;
        }
        let result = match result {
            Ok(Some(file)) => save(&vm, file),
            Ok(None) => return,
            Err(error) => Err(error),
        };
        if vm.route().app() == Some(AppId::OneNote) {
            match result {
                Ok(id) => vm.navigate(Route::Note(id)),
                Err(error) => vm.set_status(error),
            }
        }
    }));
}

fn save(vm: &PhoneViewModel, file: PickedFile) -> Result<Id, String> {
    let note = parse(file, vm.now())?;
    let id = vm.data().next_id;
    vm.save_note(note);
    if vm.data().note(id).is_some() {
        Ok(id)
    } else {
        Err(vm.status())
    }
}

fn parse(file: PickedFile, now: i64) -> Result<Note, String> {
    if file.bytes.len() > MAX_NOTE_BYTES as usize {
        return Err("This note exceeds the 256 KiB import limit.".into());
    }
    let body = String::from_utf8(file.bytes)
        .map_err(|_| "Choose a text or Markdown file saved as UTF-8.".to_owned())?;
    if body.contains('\0') {
        return Err("This file contains binary data and cannot be imported as a note.".into());
    }
    let title = file
        .name
        .rsplit_once('.')
        .map_or(file.name.as_str(), |(stem, _)| stem)
        .trim()
        .chars()
        .take(120)
        .collect::<String>();
    Ok(Note {
        id: 0,
        title: if title.is_empty() {
            "Imported note".into()
        } else {
            title
        },
        body: body.strip_prefix('\u{feff}').unwrap_or(&body).to_owned(),
        modified: now,
    })
}

#[cfg(test)]
mod tests;
