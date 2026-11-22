use ferroui_view::prelude::*;
use phone_domain::{AppId, Route};
use std::{cell::RefCell, collections::BTreeMap, rc::Rc};

#[derive(Clone)]
pub struct Dialog {
    pub title: String,
    pub labels: Vec<String>,
    pub save: Rc<dyn Fn(Vec<String>)>,
}

pub struct Session {
    pub pivot: Signal<usize>,
    pub selected: Signal<Option<u64>>,
    pub query: Signal<String>,
    pub entry: Signal<String>,
    pub menu: Signal<bool>,
    pub calendar_month: Signal<i32>,
    pub edit_tile: Signal<Option<u64>>,
    pub drop_target: Signal<Option<u64>>,
    pub folder: Signal<Option<u64>>,
    pub alphabet: Signal<bool>,
    pub notification_open: Signal<bool>,
    pub switcher_open: Signal<bool>,
    pub locked: Signal<bool>,
    pub dialog: Signal<Option<Dialog>>,
    pub fields: [Signal<String>; 6],
    pub form_error: Signal<String>,
    pub dial: Signal<String>,
    pub calculator: RefCell<phone_domain::Calculator>,
    pub calculator_revision: Signal<u64>,
    pub media_title: Signal<String>,
    pub media_source: Signal<String>,
    pub media_playing: Signal<bool>,
    pub game_board: Signal<[u8; 9]>,
    pub game_score: Signal<[u32; 3]>,
    pub browser_url: Signal<String>,
    pub photo_zoom: Signal<f64>,
    pub weather: Signal<Option<(String, f64, String)>>,
    pub last_route: RefCell<Route>,
    pub app_pivots: RefCell<BTreeMap<String, usize>>,
    pub scrolls: RefCell<BTreeMap<String, f64>>,
    pub app_scroll: NodeRef<ScrollViewer>,
}

impl Session {
    pub fn new() -> Self {
        Self {
            pivot: signal(0),
            selected: signal(None),
            query: signal(String::new()),
            entry: signal(String::new()),
            menu: signal(false),
            calendar_month: signal(0),
            edit_tile: signal(None),
            drop_target: signal(None),
            folder: signal(None),
            alphabet: signal(false),
            notification_open: signal(false),
            switcher_open: signal(false),
            locked: signal(false),
            dialog: signal(None),
            fields: std::array::from_fn(|_| signal(String::new())),
            form_error: signal(String::new()),
            dial: signal(String::new()),
            calculator: RefCell::new(phone_domain::Calculator::default()),
            calculator_revision: signal(0),
            media_title: signal(String::new()),
            media_source: signal(String::new()),
            media_playing: signal(false),
            game_board: signal([0; 9]),
            game_score: signal([0; 3]),
            browser_url: signal(String::new()),
            photo_zoom: signal(1.0),
            weather: signal(None),
            last_route: RefCell::new(Route::Start),
            app_pivots: RefCell::new(BTreeMap::new()),
            scrolls: RefCell::new(BTreeMap::new()),
            app_scroll: NodeRef::new(),
        }
    }

    pub fn form(
        &self,
        title: &str,
        fields: Vec<(&str, String)>,
        callback: Rc<dyn Fn(Vec<String>)>,
    ) {
        self.menu.set(false);
        self.form_error.set(String::new());
        for (index, field) in self.fields.iter().enumerate() {
            field.set(
                fields
                    .get(index)
                    .map(|item| item.1.clone())
                    .unwrap_or_default(),
            );
        }
        self.dialog.set(Some(Dialog {
            title: title.into(),
            labels: fields.into_iter().take(6).map(|f| f.0.into()).collect(),
            save: callback,
        }));
    }

    pub fn confirm(&self, title: &str, callback: Rc<dyn Fn()>) {
        self.form(title, Vec::new(), Rc::new(move |_| callback()));
    }

    pub fn dismiss(&self) -> bool {
        if self.dialog.get_untracked().is_some() {
            self.dialog.set(None);
        } else if self.menu.get_untracked() {
            self.menu.set(false);
        } else if self.notification_open.get_untracked() {
            self.notification_open.set(false);
        } else if self.switcher_open.get_untracked() {
            self.switcher_open.set(false);
        } else if self.alphabet.get_untracked() {
            self.alphabet.set(false);
        } else if self.edit_tile.get_untracked().is_some() {
            self.edit_tile.set(None);
        } else if self.folder.get_untracked().is_some() {
            self.folder.set(None);
        } else if self.selected.get_untracked().is_some() {
            self.selected.set(None);
        } else {
            return false;
        }
        true
    }

    pub fn route_changed(&self, route: &Route) {
        let previous = self.last_route.borrow().clone();
        if previous == *route {
            return;
        }
        if let Route::App(app) = previous {
            self.app_pivots
                .borrow_mut()
                .insert(app.label().into(), self.pivot.get_untracked());
        }
        let pivot = if let Route::App(app) = route {
            self.app_pivots
                .borrow()
                .get(app.label())
                .copied()
                .unwrap_or(0)
        } else {
            0
        };
        self.pivot.set(pivot);
        self.query.set(String::new());
        self.menu.set(false);
        self.alphabet.set(false);
        self.selected.set(None);
        self.dialog.set(None);
        self.edit_tile.set(None);
        *self.last_route.borrow_mut() = route.clone();
    }

    pub fn open_app(&self, vm: &phone_state::PhoneViewModel, app: AppId) {
        self.folder.set(None);
        self.edit_tile.set(None);
        self.switcher_open.set(false);
        vm.resume_app(app);
    }
}
