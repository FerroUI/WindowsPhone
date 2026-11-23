use ferroui_base::BoxedValue;
use ferroui_controls::ItemsSource;
use ferroui_view::prelude::*;
use phone_domain::{Id, Route};
use phone_state::PhoneViewModel;
use std::{cell::RefCell, rc::Rc};
#[derive(Clone, PartialEq)]
pub(super) struct PageRow {
    pub id: Id,
    pub title: String,
    pub detail: String,
}
struct Cached {
    rows: Vec<PageRow>,
    source: ItemsSource,
}
impl Cached {
    fn new(rows: Vec<PageRow>) -> Self {
        let source = ItemsSource::from_items(
            rows.iter()
                .cloned()
                .map(|row| Some(Rc::new(row) as BoxedValue)),
        );
        Self { rows, source }
    }
}
pub(super) fn view(rows: Vec<PageRow>, vm: Rc<PhoneViewModel>) -> Element {
    rerender(move || {
        let cache = use_hook(|| Rc::new(RefCell::new(Cached::new(rows.clone()))));
        let source = {
            let mut cached = cache.borrow_mut();
            if cached.rows != rows {
                *cached = Cached::new(rows.clone());
            }
            cached.source.clone()
        };
        let template = use_hook({
            let vm = vm.clone();
            move || {
                template(move |row: PageRow| {
                    let model = vm.clone();
                    phone_sdk::common::row(&row.title, &row.detail, move || {
                        model.navigate(Route::Note(row.id))
                    })
                })
            }
        });
        rsx! {ListBox{name:"NotePageList",class:"phone-list",items_source:source,item_template:template}}
    })
}
