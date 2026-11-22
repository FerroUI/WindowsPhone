use ferroui_view::prelude::*;
use phone_domain::Id;
use std::collections::BTreeSet;

pub struct MessageState {
    pub selected: Signal<BTreeSet<Id>>,
    pub selecting: Signal<bool>,
    pub searching: Signal<bool>,
    pub contacts: Signal<bool>,
    pub context: Signal<Option<Id>>,
}
impl MessageState {
    pub fn new() -> Self {
        Self {
            selected: signal(BTreeSet::new()),
            selecting: signal(false),
            searching: signal(false),
            contacts: signal(false),
            context: signal(None),
        }
    }
    pub fn begin(&self, id: Option<Id>) {
        self.selecting.set(true);
        self.searching.set(false);
        self.selected.set(id.into_iter().collect());
    }
    pub fn toggle(&self, id: Id) {
        self.selected.update(|ids| {
            if !ids.remove(&id) {
                ids.insert(id);
            }
        });
    }
    pub fn reset(&self) {
        self.selected.set(BTreeSet::new());
        self.selecting.set(false);
        self.searching.set(false);
        self.contacts.set(false);
        self.context.set(None);
    }
    pub fn dismiss(&self) -> bool {
        if self.context.get_untracked().is_some() {
            self.context.set(None);
        } else if self.contacts.get_untracked() {
            self.contacts.set(false);
        } else if self.selecting.get_untracked() {
            self.selecting.set(false);
            self.selected.set(BTreeSet::new());
        } else if self.searching.get_untracked() {
            self.searching.set(false);
        } else {
            return false;
        }
        true
    }
}
