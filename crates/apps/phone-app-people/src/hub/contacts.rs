//! Local all/favorites/search lists share data rules, not scroll ownership.
use super::*;
use ferroui_dynamic_data::IObservableCache;
use phone_controls::ListGroup;

#[derive(Clone, PartialEq)]
enum Item {
    Header(char),
    Contact(Rc<Contact>),
}

pub(super) fn group_letter(name: &str) -> char {
    name.trim_start()
        .chars()
        .next()
        .filter(char::is_ascii_alphabetic)
        .map(|letter| letter.to_ascii_lowercase())
        .unwrap_or('#')
}

fn rows(vm: &PhoneViewModel, favorites: bool, query: &str) -> Vec<Rc<Contact>> {
    let mut contacts = vm
        .contacts
        .items()
        .into_iter()
        .filter(|contact| !favorites || contact.favorite)
        .filter(|contact| {
            format!(
                "{} {} {} {}",
                contact.name, contact.phone, contact.email, contact.company
            )
            .to_lowercase()
            .contains(query)
        })
        .collect::<Vec<_>>();
    contacts.sort_by_key(|contact| (group_letter(&contact.name), contact.name.to_lowercase()));
    contacts
}

pub(super) fn letters(vm: &PhoneViewModel, favorites: bool, query: &str) -> Vec<char> {
    let mut result = rows(vm, favorites, query)
        .into_iter()
        .map(|contact| group_letter(&contact.name))
        .collect::<Vec<_>>();
    result.dedup();
    result
}

fn letter(ui: &Session, value: char) -> Element {
    let show = ui.alphabet;
    let palette = phone_controls::palette();
    rsx! {Border {height:56.0,background:phone_controls::palette().background,
        Button {name:format!("PeopleLetter{value}"),class:"phone-flat",margin:(20.0,14.0,0.0,4.0),padding:0.0,
            width:38.0,height:38.0,horizontal_alignment:H::Left,automation_properties_name:format!("Jump from letter {value}"),
            on_click:move |_|show.set(true),
            Border {border_brush:palette.accent.as_str(),border_thickness:2.0,width:34.0,height:34.0,
                TextBlock {text:value.to_string(),font_size:23.0,foreground:palette.accent.as_str(),horizontal_alignment:H::Center}}
        }
    }}
}

pub(super) fn pane(
    vm: Rc<PhoneViewModel>,
    ui: Rc<Session>,
    favorites: bool,
    searching: bool,
) -> Element {
    rerender(move || {
        let state = ui.state::<State>();
        let query = if searching {
            state.query.get().trim().to_lowercase()
        } else {
            String::new()
        };
        if searching {
            state.query_changed(
                if favorites {
                    SearchScope::Favorites
                } else {
                    SearchScope::All
                },
                &query,
            );
        }
        let (name, memory) = if searching {
            ("PeopleSearchList", &state.local_search)
        } else if favorites {
            ("PeopleFavoritesList", &state.favorites)
        } else {
            ("PeopleAllList", &state.all)
        };
        let contacts = rows(&vm, favorites, &query);
        let mut items = Vec::new();
        let mut groups = Vec::new();
        let mut previous = None;
        for contact in contacts {
            let letter = group_letter(&contact.name);
            if previous != Some(letter) {
                groups.push(ListGroup::new(letter.to_string(), items.len()));
                items.push(Item::Header(letter));
                previous = Some(letter);
            }
            items.push(Item::Contact(contact));
        }
        memory.scroll.set_groups(groups);
        let is_empty = items.is_empty();
        let model = vm.clone();
        let session = ui.clone();
        let epoch = vm.document_epoch();
        let list = phone_sdk::device::grouped_list(
            name,
            items,
            Rc::new(move |item| match item {
                Item::Header(value) => letter(&session, value),
                Item::Contact(contact) => {
                    let vm = model.clone();
                    let id = contact.id;
                    row(&contact.name, &contact.company, move || {
                        if vm.document_epoch() == epoch {
                            vm.navigate(Route::Contact(id));
                        }
                    })
                }
            }),
            memory.node,
            memory.scroll.clone(),
        );
        // The native list's source/template hooks belong to this rerender even
        // when its empty state is shown. A query may cross that boundary in
        // either direction without changing the component's hook sequence.
        if is_empty {
            return empty(
                "No contacts",
                if query.is_empty() {
                    if favorites {
                        "Mark a contact as a favorite to keep them here."
                    } else {
                        "Tap new to add someone."
                    }
                } else {
                    "Try a different name or number."
                },
            );
        }
        let sticky = memory
            .scroll
            .current_group()
            .and_then(|key| key.chars().next());
        rsx! {Grid {name:format!("{name}Pane"),clip_to_bounds:true,
            {list}
            if let Some(value)=sticky {Border {name:format!("{name}StickyHeader"),vertical_alignment:V::Top,
                render_transform:Some(memory.scroll.translation().into()),
                {letter(&ui,value)}
            }}
        }}
    })
}
