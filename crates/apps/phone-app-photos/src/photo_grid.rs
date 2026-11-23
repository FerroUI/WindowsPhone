use ferroui_base::{layout::Orientation, BoxedValue};
use ferroui_controls::ItemsSource;
use ferroui_view::prelude::*;
use phone_domain::Photo;
use phone_sdk::{images, session::Session};
use std::{cell::RefCell, rc::Rc};

#[derive(Clone, PartialEq)]
struct PhotoRow {
    photos: Vec<Photo>,
    edge: f64,
}

struct Rows {
    photos: Vec<Photo>,
    edge: f64,
    source: ItemsSource,
}

impl Rows {
    fn new(photos: Vec<Photo>, edge: f64) -> Self {
        let source = ItemsSource::from_items(photos.chunks(3).map(|row| {
            Some(Rc::new(PhotoRow {
                photos: row.to_vec(),
                edge,
            }) as BoxedValue)
        }));
        Self {
            photos,
            edge,
            source,
        }
    }
}

/// Three-column rows virtualize vertically through FerroUI's native ListBox panel.
/// Sources and recycling templates keep their identities through clock/menu renders.
pub fn view(photos: Vec<Photo>, edge: f64, ui: Rc<Session>) -> Element {
    rerender(move || {
        let cached = use_hook(|| Rc::new(RefCell::new(Rows::new(photos.clone(), edge))));
        let source = {
            let mut cached = cached.borrow_mut();
            if cached.photos != photos || cached.edge != edge {
                *cached = Rows::new(photos.clone(), edge);
            }
            cached.source.clone()
        };
        let template = use_hook({
            let ui = ui.clone();
            move || {
                template(move |row: PhotoRow| {
                    let mut cells = stack_panel().orientation(Orientation::Horizontal);
                    for photo in row.photos {
                        let open = ui.clone();
                        let id = photo.id;
                        cells = cells.child(rsx! {
                        Button { class:"phone-flat",width:row.edge,height:row.edge,
                            margin:(0.0,0.0,8.0,8.0),automation_properties_name:photo.name,
                            on_click:move |_| {open.state::<crate::PhotosSession>().zoom.set(1.0);open.selected.set(Some(id));},
                            {images::thumbnail(&photo.source)}
                        }
                    });
                    }
                    cells
                })
            }
        });
        rsx! {ListBox{class:"phone-list",margin:(20.0,12.0,12.0,0.0),
        items_source:source,item_template:template}}
    })
}
