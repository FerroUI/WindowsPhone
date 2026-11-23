use super::*;
use phone_domain::PhoneData;
use std::cell::Cell;

fn file(name: &str, bytes: impl Into<Vec<u8>>) -> PickedFile {
    PickedFile {
        name: name.into(),
        mime: "text/plain".into(),
        bytes: bytes.into(),
    }
}

#[test]
fn imports_unicode_text_preserving_line_endings_and_bounds_the_title() {
    let text = "\u{feff}Résumé\r\n第二行\n";
    let note = parse(file(" journal.2026.md ", text.as_bytes()), 42).unwrap();
    assert_eq!(note.id, 0);
    assert_eq!(note.title, "journal.2026");
    assert_eq!(note.body, "Résumé\r\n第二行\n");
    assert_eq!(note.modified, 42);
    let note = parse(file(&format!("{}.txt", "é".repeat(200)), []), 0).unwrap();
    assert_eq!(note.title.chars().count(), 120);
    assert_eq!(parse(file(".txt", []), 0).unwrap().title, "Imported note");
}

#[test]
fn invalid_text_never_mutates_phone_and_success_persists_once() {
    let writes = Rc::new(Cell::new(0));
    let observed = writes.clone();
    let vm = PhoneViewModel::new(
        PhoneData::demo(),
        Rc::new(move |_| observed.set(observed.get() + 1)),
    );
    let original = vm.data().clone();
    let revision = vm.revision();
    for invalid in [
        file("bad.txt", [0xf0, 0x28, 0x8c, 0x28]),
        file("binary.txt", b"hello\0world".as_slice()),
        file("large.md", vec![b'x'; MAX_NOTE_BYTES as usize + 1]),
    ] {
        assert!(save(&vm, invalid).is_err());
        assert_eq!(*vm.data(), original);
        assert_eq!(vm.revision(), revision);
        assert_eq!(writes.get(), 0);
    }
    let id = save(&vm, file("project.txt", b"First\nsecond".as_slice())).unwrap();
    assert_eq!(id, original.next_id);
    assert_eq!(vm.data().note(id).unwrap().title, "project");
    assert_eq!(vm.data().note(id).unwrap().body, "First\nsecond");
    assert_eq!(vm.data().notes.len(), original.notes.len() + 1);
    assert_eq!(writes.get(), 1);
}
