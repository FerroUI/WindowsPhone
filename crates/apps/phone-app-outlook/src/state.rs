use ferroui_view::prelude::*;
use phone_domain::Id;
use std::collections::BTreeSet;

pub struct MailState {
    pub folder: Signal<String>,
    pub selected: Signal<BTreeSet<Id>>,
    pub selecting: Signal<bool>,
    pub searching: Signal<bool>,
    pub folders: Signal<bool>,
}
impl MailState {
    pub fn new() -> Self {
        Self {
            folder: signal("inbox".into()),
            selected: signal(BTreeSet::new()),
            selecting: signal(false),
            searching: signal(false),
            folders: signal(false),
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
        self.folders.set(false);
    }
    pub fn dismiss(&self) -> bool {
        if self.folders.get_untracked() {
            self.folders.set(false);
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

impl Default for MailState {
    fn default() -> Self {
        Self::new()
    }
}
