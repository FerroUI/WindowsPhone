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
    owner: Owner,
    states: RefCell<std::collections::HashMap<std::any::TypeId, Rc<dyn std::any::Any>>>,
    registry: Rc<crate::AppRegistry>,
    activation: RefCell<Option<(String, serde_json::Value)>>,
    activation_revision: Signal<u64>,
    pub document_epoch: Cell<u64>,
    pub live_clock: Signal<i64>,
    pub live_clock_enabled: Cell<bool>,
    pub start_wallpaper: ferroui_base::Ref<ferroui_base::media::TranslateTransform>,
    pub pivot: Signal<usize>,
    pub selected: Signal<Option<u64>>,
    pub query: Signal<String>,
    pub entry: Signal<String>,
    pub menu: Signal<bool>,
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
    pub media_title: Signal<String>,
    pub media_source: Signal<String>,
    pub media_playing: Signal<bool>,
    pub last_route: RefCell<Route>,
    pub app_pivots: RefCell<BTreeMap<String, usize>>,
    pub app_selections: RefCell<BTreeMap<String, Option<u64>>>,
    pub scrolls: RefCell<BTreeMap<String, f64>>,
    pub app_scroll: NodeRef<ScrollViewer>,
}
impl Session {
    pub fn new(now: i64, registry: Rc<crate::AppRegistry>) -> Self {
        let owner = Owner::new();
        owner.with(|| Self {
            owner,
            states: RefCell::new(Default::default()),
            registry,
            activation: RefCell::new(None),
            activation_revision: signal(0),
            document_epoch: Cell::new(0),
            live_clock: signal(now),
            live_clock_enabled: Cell::new(false),
            start_wallpaper: ferroui_base::media::TranslateTransform::new(),
            pivot: signal(0),
            selected: signal(None),
            query: signal(String::new()),
            entry: signal(String::new()),
            menu: signal(false),
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
            media_title: signal(String::new()),
            media_source: signal(String::new()),
            media_playing: signal(false),
            last_route: RefCell::new(Route::Start),
            app_pivots: RefCell::new(BTreeMap::new()),
            app_selections: RefCell::new(BTreeMap::new()),
            scrolls: RefCell::new(BTreeMap::new()),
            app_scroll: NodeRef::new(),
        })
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
        self.form_with_action(title, Vec::new(), "confirm", Rc::new(move |_| callback()));
    }

    pub fn open_app(&self, vm: &phone_state::PhoneViewModel, app: AppId) {
        self.folder.set(None);
        self.edit_tile.set(None);
        self.switcher_open.set(false);
        vm.resume_app(app);
    }

    pub fn state<T: Default + 'static>(&self) -> Rc<T> {
        let id = std::any::TypeId::of::<T>();
        if let Some(existing) = self.states.borrow().get(&id).cloned() {
            return existing.downcast::<T>().expect("session type key");
        }
        let created = self.owner.with(|| Rc::new(T::default()));
        self.states.borrow_mut().insert(id, created.clone());
        created
    }
    /// Queue one bounded data activation, delivered after destination lifecycle reset.
    pub fn queue_activation(&self, id: &str, value: serde_json::Value) -> Result<(), String> {
        phone_domain::validate_app_value(&value)?;
        if self.registry.by_id(id).is_none() {
            return Err("Application is not registered.".into());
        }
        if serde_json::to_vec(&value).map_err(|e| e.to_string())?.len() > 65536 {
            return Err("Activation data exceeds 64 KiB.".into());
        }
        *self.activation.borrow_mut() = Some((id.into(), value));
        self.activation_revision
            .set(self.activation_revision.get_untracked().wrapping_add(1));
        Ok(())
    }
    pub fn take_activation(&self, id: &str) -> Option<serde_json::Value> {
        self.activation_revision.get();
        let mut value = self.activation.borrow_mut();
        if value.as_ref().is_some_and(|(target, _)| target == id) {
            value.take().map(|(_, value)| value)
        } else {
            None
        }
    }
    pub fn clear_activation(&self) {
        self.activation.borrow_mut().take();
    }
    pub fn registry(&self) -> Rc<crate::AppRegistry> {
        self.registry.clone()
    }
    pub fn app_manifests(&self) -> Vec<crate::AppManifest> {
        self.registry.manifests()
    }
}

impl Drop for Session {
    fn drop(&mut self) {
        crate::tasks::cancel_for_session(self);
        self.owner.dispose();
    }
}
