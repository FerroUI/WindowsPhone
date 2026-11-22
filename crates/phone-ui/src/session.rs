use ferroui_view::prelude::*;
use phone_domain::{AppId, Route};
use std::{
    cell::{Cell, RefCell},
    collections::BTreeMap,
    rc::Rc,
};

#[derive(Clone)]
pub struct Dialog {
    pub title: String,
    pub action_label: String,
    pub labels: Vec<String>,
    pub save: Rc<dyn Fn(Vec<String>)>,
}

pub struct Session {
    document_epoch: Cell<u64>,
    pub hardware: crate::hardware::HardwareState,
    pub office: crate::apps::office::OfficeSession,
    pub files: crate::apps::files::FilesSession,
    pub browser: crate::apps::services::BrowserSession,
    pub mail: crate::apps::mail::MailState,
    pub messages: crate::apps::messaging::MessageState,
    pub(crate) previews: crate::previews::PreviewCache,
    pub pivot: Signal<usize>,
    pub selected: Signal<Option<u64>>,
    pub query: Signal<String>,
    pub entry: Signal<String>,
    pub menu: Signal<bool>,
    pub calendar_month: Signal<i32>,
    pub calendar_date: Signal<String>,
    pub mail_folder: Signal<String>,
    pub message_drafts: Signal<BTreeMap<u64, String>>,
    pub edit_tile: Signal<Option<u64>>,
    pub drop_target: Signal<Option<u64>>,
    pub folder: Signal<Option<u64>>,
    pub alphabet: Signal<bool>,
    pub notification_open: Signal<bool>,
    pub notification_peek: Signal<bool>,
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
    pub app_selections: RefCell<BTreeMap<String, Option<u64>>>,
    pub scrolls: RefCell<BTreeMap<String, f64>>,
    pub app_scroll: NodeRef<ScrollViewer>,
}

impl Session {
    pub fn new() -> Self {
        Self {
            document_epoch: Cell::new(0),
            hardware: crate::hardware::HardwareState::new(),
            office: crate::apps::office::OfficeSession::default(),
            files: crate::apps::files::FilesSession::default(),
            browser: crate::apps::services::BrowserSession::default(),
            mail: crate::apps::mail::MailState::new(),
            messages: crate::apps::messaging::MessageState::new(),
            previews: crate::previews::PreviewCache::default(),
            pivot: signal(0),
            selected: signal(None),
            query: signal(String::new()),
            entry: signal(String::new()),
            menu: signal(false),
            calendar_month: signal(0),
            calendar_date: signal(String::new()),
            mail_folder: signal("inbox".into()),
            message_drafts: signal(BTreeMap::new()),
            edit_tile: signal(None),
            drop_target: signal(None),
            folder: signal(None),
            alphabet: signal(false),
            notification_open: signal(false),
            notification_peek: signal(false),
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
            app_selections: RefCell::new(BTreeMap::new()),
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
        self.form_with_action(title, fields, "save", callback);
    }

    pub fn form_with_action(
        &self,
        title: &str,
        fields: Vec<(&str, String)>,
        action_label: &str,
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
            action_label: action_label.into(),
            labels: fields.into_iter().take(6).map(|f| f.0.into()).collect(),
            save: callback,
        }));
    }

    pub fn confirm(&self, title: &str, callback: Rc<dyn Fn()>) {
        self.form(title, Vec::new(), Rc::new(move |_| callback()));
    }

    pub fn dismiss(&self) -> bool {
        if self.hardware.is_open() {
            self.hardware.close();
        } else if self.dialog.get_untracked().is_some() {
            self.dialog.set(None);
        } else if self.notification_open.get_untracked() {
            self.notification_open.set(false);
            self.notification_peek.set(false);
        } else if self.switcher_open.get_untracked() {
            self.switcher_open.set(false);
        } else if self.menu.get_untracked() {
            self.menu.set(false);
        } else if self.mail.dismiss() || self.messages.dismiss() {
            self.query.set(String::new());
        } else if self.office.show_new.get_untracked() {
            self.office.show_new.set(false);
        } else if self.office.show_find.get_untracked() {
            self.office.show_find.set(false);
            self.query.set(String::new());
        } else if self.files.folder_menu.get_untracked().is_some() {
            self.files.folder_menu.set(None);
        } else if self.files.selecting.get_untracked() {
            self.files.selecting.set(false);
            self.files.selection.set(Default::default());
        } else if self.files.searching.get_untracked() {
            self.files.searching.set(false);
            self.query.set(String::new());
        } else if self.files.transfer.get_untracked().is_some() {
            self.files.transfer.set(None);
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
        self.mail.reset();
        self.messages.reset();
        self.office.show_new.set(false);
        self.office.show_find.set(false);
        self.office.formatting.set(false);
        if !matches!(route, Route::FileFolder(_) | Route::App(AppId::Files)) {
            self.clear_files();
        }
        if let Route::App(app) = previous {
            self.app_pivots
                .borrow_mut()
                .insert(app.label().into(), self.pivot.get_untracked());
            self.app_selections
                .borrow_mut()
                .insert(app.label().into(), self.selected.get_untracked());
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
        if matches!(route, Route::App(AppId::InternetExplorer)) {
            self.entry.set(self.browser_url.get_untracked());
        }
        self.query.set(String::new());
        self.menu.set(false);
        self.alphabet.set(false);
        self.selected.set(if let Route::App(app) = route {
            self.app_selections
                .borrow()
                .get(app.label())
                .copied()
                .flatten()
        } else {
            None
        });
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

    pub fn clear_overlays(&self) {
        self.hardware.close();
        self.mail.reset();
        self.messages.reset();
        self.office.show_new.set(false);
        self.office.show_find.set(false);
        self.office.formatting.set(false);
        self.clear_files();
        self.dialog.set(None);
        self.menu.set(false);
        self.notification_open.set(false);
        self.notification_peek.set(false);
        self.switcher_open.set(false);
        self.alphabet.set(false);
        self.edit_tile.set(None);
        self.folder.set(None);
    }

    fn clear_files(&self) {
        self.files.folder_menu.set(None);
        self.files.selecting.set(false);
        self.files.selection.set(Default::default());
        self.files.searching.set(false);
        self.files.transfer.set(None);
    }

    pub(crate) fn reconcile_document(&self, epoch: u64) {
        if self.document_epoch.replace(epoch) == epoch {
            return;
        }
        crate::gestures::cancel();
        self.clear_overlays();
        self.previews.clear();
        self.app_pivots.borrow_mut().clear();
        self.app_selections.borrow_mut().clear();
        *self.last_route.borrow_mut() = Route::Start;
        self.message_drafts.set(BTreeMap::new());
        self.query.set(String::new());
        self.entry.set(String::new());
        self.browser_url.set(String::new());
        self.selected.set(None);
        self.pivot.set(0);
        self.mail_folder.set("inbox".into());
        self.office.paragraph.set(0);
        self.office.sheet.set(0);
        self.office.row_page.set(0);
        self.office.col_page.set(0);
        self.office.slide.set(0);
        for field in &self.fields {
            field.set(String::new());
        }
    }

    pub(crate) fn preview_obscured(&self, route: &Route) -> bool {
        route.is_overlay()
            || self.hardware.is_open()
            || self.notification_open.get_untracked()
            || self.switcher_open.get_untracked()
            || self.locked.get_untracked()
            || self.dialog.get_untracked().is_some()
            || self.menu.get_untracked()
            || self.alphabet.get_untracked()
            || self.edit_tile.get_untracked().is_some()
            || self.folder.get_untracked().is_some()
    }
}
