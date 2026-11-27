use super::*;
use ferroui_controls::ListBox;
use phone_controls::GroupedListState;
use std::cell::{Cell, RefCell};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SearchScope {
    All,
    Favorites,
    Device,
}

pub(crate) struct ListState {
    pub node: NodeRef<ListBox>,
    pub scroll: Rc<GroupedListState>,
}

impl Default for ListState {
    fn default() -> Self {
        Self {
            node: NodeRef::new(),
            scroll: GroupedListState::new(56.0),
        }
    }
}

pub(crate) struct State {
    pub search: Signal<Option<SearchScope>>,
    pub query: Signal<String>,
    pub all: ListState,
    pub favorites: ListState,
    pub local_search: ListState,
    pub device: ListState,
    pub device_search: ListState,
    pub epoch: Cell<Option<u64>>,
    query_key: RefCell<Option<(SearchScope, String)>>,
}

impl Default for State {
    fn default() -> Self {
        Self {
            search: signal(None),
            query: signal(String::new()),
            all: Default::default(),
            favorites: Default::default(),
            local_search: Default::default(),
            device: Default::default(),
            device_search: Default::default(),
            epoch: Cell::new(None),
            query_key: RefCell::new(None),
        }
    }
}

impl State {
    pub fn reset(&self) {
        self.search.set(None);
        self.query.set(String::new());
        self.epoch.set(None);
        self.query_key.borrow_mut().take();
        for pane in [
            &self.all,
            &self.favorites,
            &self.local_search,
            &self.device,
            &self.device_search,
        ] {
            pane.scroll.reset();
        }
    }

    pub fn begin_search(&self, mode: usize) {
        self.query.set(String::new());
        self.query_key.borrow_mut().take();
        self.local_search.scroll.reset();
        self.device_search.scroll.reset();
        self.search.set(Some(match mode {
            1 => SearchScope::Favorites,
            2 => SearchScope::Device,
            _ => SearchScope::All,
        }));
    }

    pub fn query_changed(&self, scope: SearchScope, query: &str) {
        let key = (scope, query.to_owned());
        if self.query_key.borrow().as_ref() != Some(&key) {
            if scope == SearchScope::Device {
                self.device_search.scroll.reset();
            } else {
                self.local_search.scroll.reset();
            }
            *self.query_key.borrow_mut() = Some(key);
        }
    }
}
